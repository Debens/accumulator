use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256, Sha512};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use url::form_urlencoded;

use crate::execution::order_action::Side;
use crate::kraken::kraken_config::KrakenConfig;
use crate::types::{instrument::Instrument, price::Price};

type HmacSha512 = Hmac<Sha512>;

#[derive(Clone, Debug)]
pub struct KrakenClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    api_secret_b64: String,
    last_nonce: Arc<AtomicU64>,
}

impl KrakenClient {
    pub fn new(config: KrakenConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: "https://api.kraken.com".to_string(),
            api_key: config.api_key,
            api_secret_b64: config.api_secret,
            last_nonce: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn limit_order(
        &self,
        instrument: &Instrument,
        side: Side,
        price: Price,
        quantity: f64,
        client_order_id: &str,
    ) -> Result<AddOrderResult> {
        let uri_path = "/0/private/AddOrder";
        let pair = instrument_to_kraken_pair(instrument);

        let side_str = match side {
            Side::Buy => "buy",
            Side::Sell => "sell",
        };

        let params = vec![
            ("ordertype".to_string(), "limit".to_string()),
            ("type".to_string(), side_str.to_string()),
            ("pair".to_string(), pair),
            ("price".to_string(), format_price(price.as_f64())),
            ("volume".to_string(), format_volume(quantity)),
            ("oflags".to_string(), "post".to_string()),
            ("cl_ord_id".to_string(), client_order_id.to_string()),
        ];

        let result: AddOrderResult = self.private_post_form(uri_path, &params).await?;
        Ok(result)
    }

    pub async fn cancel_all_orders(&self) -> Result<CancelAllResult> {
        let uri_path = "/0/private/CancelAll";

        let params: Vec<(String, String)> = Vec::new();

        let result: CancelAllResult = self.private_post_form(uri_path, &params).await?;
        Ok(result)
    }

    pub async fn cancel_order(&self, client_order_id: &str) -> Result<CancelOrderResult> {
        let uri_path = "/0/private/CancelOrder";

        let params = vec![("cl_ord_id".to_string(), client_order_id.to_string())];

        let result: CancelOrderResult = self.private_post_form(uri_path, &params).await?;

        tracing::info!(client_order_id = %client_order_id, count = result.count, "cancel order result");

        Ok(result)
    }

    async fn private_post_form<T: DeserializeOwned>(
        &self,
        uri_path: &str,
        params: &[(String, String)],
    ) -> Result<T> {
        let nonce = self.next_nonce();
        let mut all_params: Vec<(String, String)> = Vec::with_capacity(params.len() + 1);
        all_params.push(("nonce".to_string(), nonce.to_string()));
        all_params.extend_from_slice(params);

        let encoded_payload = encode_form(&all_params);
        let headers = self.signed_headers(uri_path, nonce, &encoded_payload)?;

        let resp = self
            .http
            .post(format!("{}{}", self.base_url, uri_path))
            .headers(headers)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(encoded_payload)
            .send()
            .await
            .context("kraken private POST failed")?;

        let status = resp.status();
        let text = resp.text().await.context("read response body failed")?;

        if !status.is_success() {
            anyhow::bail!("kraken http error {status}: {text}");
        }

        let parsed: KrakenResponse<T> = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = %e, %text, "failed to parse kraken JSON response");
                anyhow::bail!("parse kraken response JSON failed: {e}; raw={text}");
            }
        };

        if !parsed.error.is_empty() {
            anyhow::bail!("kraken api error: {:?}", parsed.error);
        }

        match parsed.result {
            Some(result) => Ok(result),
            None => {
                anyhow::bail!("kraken response missing `result` but `error` was empty; raw={text}")
            }
        }
    }

    fn signed_headers(
        &self,
        uri_path: &str,
        nonce: u64,
        encoded_payload: &str,
    ) -> Result<HeaderMap> {
        let secret = general_purpose::STANDARD
            .decode(&self.api_secret_b64)
            .map_err(|_| anyhow!("invalid base64 api secret"))?;

        let mut sha256 = Sha256::new();
        sha256.update(nonce.to_string().as_bytes());
        sha256.update(encoded_payload.as_bytes());
        let sha256_digest = sha256.finalize();

        let mut mac =
            HmacSha512::new_from_slice(&secret).map_err(|_| anyhow!("invalid HMAC key"))?;
        mac.update(uri_path.as_bytes());
        mac.update(&sha256_digest);
        let signature = mac.finalize().into_bytes();

        let api_sign = general_purpose::STANDARD.encode(signature);

        let mut headers = HeaderMap::new();
        headers.insert(
            "API-Key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|_| anyhow!("invalid API key header value"))?,
        );
        headers.insert(
            "API-Sign",
            HeaderValue::from_str(&api_sign)
                .map_err(|_| anyhow!("invalid API sign header value"))?,
        );

        Ok(headers)
    }

    fn next_nonce(&self) -> u64 {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        loop {
            let prev = self.last_nonce.load(Ordering::Relaxed);
            let next = if now_ms > prev { now_ms } else { prev + 1 };
            if self
                .last_nonce
                .compare_exchange(prev, next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return next;
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct KrakenResponse<T> {
    #[serde(default)]
    error: Vec<String>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct AddOrderResult {
    pub txid: Vec<String>,
    pub descr: AddOrderDescr,
}

#[derive(Debug, Deserialize)]
pub struct AddOrderDescr {
    pub order: String,
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderResult {
    pub count: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct CancelAllResult {
    pub count: i64,
}

fn encode_form(params: &[(String, String)]) -> String {
    let mut ser = form_urlencoded::Serializer::new(String::new());
    for (k, v) in params {
        ser.append_pair(k, v);
    }
    ser.finish()
}

fn format_volume(qty: f64) -> String {
    format!("{:.12}", qty)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn format_price(p: f64) -> String {
    format!("{:.10}", p)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn instrument_to_kraken_pair(instrument: &Instrument) -> String {
    let base = instrument.base().to_uppercase();
    let quote = instrument.quote().to_uppercase();

    let base = match base.as_str() {
        "BTC" => "XBT".to_string(),
        _ => base,
    };

    format!("{base}{quote}")
}
