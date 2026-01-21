use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::watch;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::inventory::InventorySource;
use crate::kraken::kraken_config::KrakenConfig;
use crate::kraken::utils::get_websocket_token;
use crate::types::instrument::Instrument;
use crate::types::inventory::Inventory;

pub struct KrakenInventory {
    tx: watch::Sender<Inventory>,
    _task: tokio::task::JoinHandle<()>,
}

impl KrakenInventory {
    pub async fn spawn(instrument: &Instrument) -> Result<Self> {
        let config = KrakenConfig::from_env()?;
        let ws_token = get_websocket_token(&config).await?;

        let (tx, _rx) = watch::channel(Inventory::default());
        let tx_task = tx.clone();

        let base_codes = kraken_balance_codes(instrument.base());
        let quote_codes = kraken_balance_codes(instrument.quote());

        let task = tokio::spawn(async move {
            let url = "wss://ws-auth.kraken.com/v2";

            loop {
                match run_once(url, &ws_token, &base_codes, &quote_codes, &tx_task).await {
                    Ok(()) => {}
                    Err(e) => eprintln!("[kraken_inventory] {e:?}"),
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        Ok(Self { tx, _task: task })
    }
}

impl InventorySource for KrakenInventory {
    fn subscribe(&self) -> watch::Receiver<Inventory> {
        self.tx.subscribe()
    }
}

async fn run_once(
    url: &str,
    ws_token: &str,
    base_codes: &[String],
    quote_codes: &[String],
    tx: &watch::Sender<Inventory>,
) -> anyhow::Result<()> {
    let (mut ws, _) = connect_async(url)
        .await
        .with_context(|| format!("connect_async({url}) failed"))?;

    let sub = serde_json::json!({
        "method": "subscribe",
        "params": {
            "channel": "balances",
            "token": ws_token
        }
    });
    ws.send(Message::Text(sub.to_string())).await?;

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        let Ok(text) = msg.into_text() else { continue };

        let frame: WsFrame = match serde_json::from_str(&text) {
            Ok(message) => message,
            /* NOTE: ignore non-frame messages */
            Err(_) => continue,
        };

        if frame.channel.as_deref() != Some("balances") {
            continue;
        }

        let Some(entries) = frame.data else {
            continue;
        };

        let mut inventory = *tx.borrow();

        if let Some(base) = pick_balance(&entries, base_codes) {
            inventory.base = base;
        }
        if let Some(quote) = pick_balance(&entries, quote_codes) {
            inventory.quote = quote;
        }

        let _ = tx.send(inventory);
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct WsFrame {
    #[serde(default)]
    channel: Option<String>,

    #[serde(default)]
    data: Option<Vec<BalanceEntry>>,
}

#[derive(Debug, Deserialize)]
struct BalanceEntry {
    asset: String,

    #[serde(default)]
    balance: f64,

    #[serde(default)]
    wallets: Vec<WalletBalance>,
}

#[derive(Debug, Deserialize)]
struct WalletBalance {
    #[serde(default)]
    r#type: String, // "spot"
    #[serde(default)]
    id: String, // "main"
    #[serde(default)]
    balance: f64,
}

fn kraken_balance_codes(sym: &str) -> Vec<String> {
    let s = sym.to_uppercase();
    match s.as_str() {
        "BTC" | "XBT" => vec!["XBT".into(), "XXBT".into(), "BTC".into()],
        "GBP" => vec!["GBP".into(), "ZGBP".into()],
        "USD" => vec!["USD".into(), "ZUSD".into()],
        "EUR" => vec!["EUR".into(), "ZEUR".into()],
        _ => vec![s],
    }
}

fn pick_balance(entries: &[BalanceEntry], codes: &[String]) -> Option<f64> {
    for code in codes {
        if let Some(e) = entries.iter().find(|e| e.asset.eq_ignore_ascii_case(code)) {
            if let Some(w) = e
                .wallets
                .iter()
                .find(|w| w.r#type == "spot" && w.id == "main")
            {
                return Some(w.balance);
            }
            return Some(e.balance);
        }
    }

    None
}
