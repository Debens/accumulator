use crate::kraken::kraken_config::KrakenConfig;
use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};

type HmacSha512 = Hmac<Sha512>;

fn sign_request(
    url_path: &str,
    postdata: &str,
    nonce: &str,
    secret_b64: &str,
) -> anyhow::Result<String> {
    let secret = general_purpose::STANDARD.decode(secret_b64)?;

    let mut sha256 = Sha256::new();
    sha256.update(nonce.as_bytes());
    sha256.update(postdata.as_bytes());
    let hash = sha256.finalize();

    let mut mac = HmacSha512::new_from_slice(&secret)?;
    mac.update(url_path.as_bytes());
    mac.update(&hash);

    Ok(general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

pub async fn get_websocket_token(config: &KrakenConfig) -> anyhow::Result<String> {
    let nonce = format!("{}", chrono::Utc::now().timestamp_millis());
    let postdata = format!("nonce={}", nonce);
    let path = "/0/private/GetWebSocketsToken";

    let sign = sign_request(path, &postdata, &nonce, &config.api_secret)?;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("https://api.kraken.com{}", path))
        .header("API-Key", &config.api_key)
        .header("API-Sign", sign)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(postdata)
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    let token = resp["result"]["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No token in response: {:?}", resp))?;

    Ok(token.to_string())
}
