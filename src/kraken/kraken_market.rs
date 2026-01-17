use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info};

use crate::events::MarketEvent;
use crate::market::market_source::MarketDataSource;
use crate::types::instrument::Instrument;
use crate::types::price::Price;

#[derive(Debug)]
pub struct KrakenMarket {
    websocket_url: String,
}

impl Default for KrakenMarket {
    fn default() -> Self {
        Self::new("wss://ws.kraken.com")
    }
}

impl KrakenMarket {
    pub fn new(websocket_url: impl Into<String>) -> Self {
        Self {
            websocket_url: websocket_url.into(),
        }
    }

    fn subscription_for_trades(&self, instrument: &Instrument) -> Value {
        json!({
            "event": "subscribe",
            "pair": [instrument.to_string()],
            "subscription": { "name": "trade" }
        })
    }

    fn subscription_for_spread(&self, instrument: &Instrument) -> Value {
        json!({
            "event": "subscribe",
            "pair": [instrument.to_string()],
            "subscription": { "name": "spread" }
        })
    }

    fn subscriptions(&self, instrument: &Instrument) -> Vec<Value> {
        vec![
            self.subscription_for_trades(instrument),
            self.subscription_for_spread(instrument),
        ]
    }

    fn parse_market_event_from_text(instrument: &Instrument, text: &str) -> Option<MarketEvent> {
        let parsed: Value = serde_json::from_str(text).ok()?;

        /* Ignore object messages like subscription_status or system_status */
        if parsed.is_object() {
            return None;
        }

        /* [channel_id, payload, channel_name, pair] */
        let array = parsed.as_array()?;
        if array.len() < 4 {
            return None;
        }

        let channel_name = array[2].as_str()?;
        let payload = &array[1];

        match channel_name {
            "trade" => Self::parse_trade(instrument, payload),
            "spread" => Self::parse_spread_top_of_book(instrument, payload),
            _ => {
                error!("Kraken websocket received unknown channel: {channel_name}");

                None
            }
        }
    }

    fn parse_trade(instrument: &Instrument, payload: &Value) -> Option<MarketEvent> {
        let trades = payload.as_array()?;
        let first_trade = trades.first()?.as_array()?;

        let price_str = first_trade.get(0)?.as_str()?;
        let quantity_str = first_trade.get(1)?.as_str()?;
        let time_str = first_trade.get(2)?.as_str()?;

        let price_value: f64 = price_str.parse().ok()?;
        let quantity_value: f64 = quantity_str.parse().ok()?;

        let timestamp_ms: u64 = time_str
            .parse::<f64>()
            .ok()
            .map(|seconds| (seconds * 1000.0) as u64)
            .unwrap_or(0);

        Some(MarketEvent::Trade {
            instrument: instrument.clone(),
            price: Price::new(price_value),
            quantity: quantity_value,
            timestamp_ms,
        })
    }

    fn parse_spread_top_of_book(instrument: &Instrument, payload: &Value) -> Option<MarketEvent> {
        let fields = payload.as_array()?;

        let bid_str = fields.get(0)?.as_str()?;
        let ask_str = fields.get(1)?.as_str()?;

        let best_bid: f64 = bid_str.parse().ok()?;
        let best_ask: f64 = ask_str.parse().ok()?;

        let timestamp_ms: u64 = fields
            .get(2)
            .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
            .map(|seconds| (seconds * 1000.0) as u64)
            .unwrap_or(0);

        Some(MarketEvent::TopOfBook {
            instrument: instrument.clone(),
            best_bid: Price::new(best_bid),
            best_ask: Price::new(best_ask),
            timestamp_ms,
        })
    }
}

#[async_trait]
impl MarketDataSource for KrakenMarket {
    async fn subscribe(&self, instrument: &Instrument, channel: Sender<MarketEvent>) -> Result<()> {
        let (stream, _http_response) = connect_async(&self.websocket_url).await?;
        let (mut writer, mut reader) = stream.split();

        for subscription in self.subscriptions(instrument) {
            writer.send(Message::Text(subscription.to_string())).await?;
        }

        info!("Kraken websocket connected");

        while let Some(message) = reader.next().await {
            let message_text: Option<String> = match message? {
                Message::Text(text) => Some(text),
                Message::Binary(binary) => match String::from_utf8(binary) {
                    Ok(text) => Some(text),
                    Err(_) => None,
                },
                Message::Ping(_) | Message::Pong(_) => None,
                Message::Close(frame) => {
                    error!("Kraken websocket closed: {:?}", frame);
                    break;
                }
                _ => None,
            };

            if let Some(text) = message_text {
                if let Some(market_event) =
                    KrakenMarket::parse_market_event_from_text(instrument, &text)
                {
                    if channel.send(market_event).await.is_err() {
                        error!("Failed to send market event");

                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
