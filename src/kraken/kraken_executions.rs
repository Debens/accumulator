use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::execution::ReportSender;
use crate::execution::order_action::Side;
use crate::execution::order_report::OrderReport;
use crate::kraken::kraken_config::KrakenConfig;
use crate::kraken::utils::get_websocket_token;
use crate::types::{instrument::Instrument, price::Price};

pub struct KrakenExecutions {
    _task: tokio::task::JoinHandle<()>,
}

impl KrakenExecutions {
    pub async fn spawn(on_report: ReportSender) -> Result<Self> {
        let config = KrakenConfig::from_env()?;
        let ws_token = get_websocket_token(&config).await?;

        let task = tokio::spawn(async move {
            let url = "wss://ws-auth.kraken.com/v2";

            loop {
                if let Err(e) = run_once(url, &ws_token, on_report.clone()).await {
                    tracing::error!(error = %e, "kraken executions stream failed");
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        Ok(Self { _task: task })
    }
}

async fn run_once(url: &str, token: &str, report_tx: broadcast::Sender<OrderReport>) -> Result<()> {
    let (mut ws, _) = connect_async(url)
        .await
        .with_context(|| format!("connect_async({url}) failed"))?;

    tracing::info!("Kraken executions websocket connected");

    let sub = serde_json::json!({
        "method": "subscribe",
        "params": {
            "channel": "executions",
            "token": token,
            "snap_orders": true,
            "snap_trades": true,
            "order_status": true
        }
    });
    ws.send(Message::Text(sub.to_string())).await?;

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        let Ok(text) = msg.into_text() else { continue };

        let frame: WsFrame = match serde_json::from_str(&text) {
            Ok(f) => f,
            Err(_) => continue, // ignore heartbeats/acks/unrelated
        };

        if frame.channel.as_deref() != Some("executions") {
            continue;
        }

        let Some(reports) = frame.data else { continue };

        for report in reports {
            if let Some(or) = to_order_report(&report) {
                let _ = report_tx.send(or);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct WsFrame {
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    data: Option<Vec<serde_json::Value>>,
}

fn to_order_report(v: &serde_json::Value) -> Option<OrderReport> {
    let exec_type = v.get("exec_type")?.as_str()?.to_string();
    let cl_ord_id = v
        .get("cl_ord_id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())?;
    let symbol = v.get("symbol").and_then(|x| x.as_str()).unwrap_or_default();
    let side = Side::from_str(v.get("side")?.as_str()?).ok()?;

    let instrument = Instrument::from_str(symbol).ok()?;

    let price = parse_f64(v.get("price").or_else(|| v.get("avg_price")))?;
    let last_qty = parse_f64(
        v.get("last_qty")
            .or_else(|| v.get("qty"))
            .or_else(|| v.get("order_qty")),
    )?;
    let cum_qty = parse_f64(v.get("cum_qty")).unwrap_or(0.0);

    match exec_type.as_str() {
        "new" => Some(OrderReport::Accepted {
            order_id: cl_ord_id,
            instrument,
            side,
            price: Price::new(price),
            quantity: last_qty,
        }),

        "trade" => Some(OrderReport::PartiallyFilled {
            order_id: cl_ord_id,
            instrument,
            side,
            price: Price::new(price),
            quantity: last_qty,
            cum_quantity: cum_qty.max(last_qty),
        }),

        "filled" => Some(OrderReport::Filled {
            order_id: cl_ord_id,
            instrument,
            side,
            price: Price::new(price),
            quantity: last_qty,
            cum_quantity: cum_qty.max(last_qty),
        }),

        "canceled" => Some(OrderReport::Cancelled {
            order_id: cl_ord_id,
            instrument,
            side,
        }),

        "expired" => Some(OrderReport::Rejected {
            order_id: cl_ord_id,
            instrument,
            side,
            reason: "expired".to_string(),
        }),

        //  "pending_new", "status", "restated"
        _ => None,
    }
}

fn parse_f64(v: Option<&serde_json::Value>) -> Option<f64> {
    let v = v?;
    if let Some(x) = v.as_f64() {
        return Some(x);
    }

    v.as_str()?.parse::<f64>().ok()
}
