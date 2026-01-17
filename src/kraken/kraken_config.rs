use std::env;

pub struct KrakenConfig {
    pub api_key: String,
    pub api_secret: String,
}

impl KrakenConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key =
            env::var("KRAKEN_API_KEY").map_err(|_| anyhow::anyhow!("KRAKEN_API_KEY not set"))?;

        let api_secret = env::var("KRAKEN_API_SECRET")
            .map_err(|_| anyhow::anyhow!("KRAKEN_API_SECRET not set"))?;

        Ok(Self {
            api_key,
            api_secret,
        })
    }
}
