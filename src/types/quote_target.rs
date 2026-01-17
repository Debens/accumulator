use crate::types::quote::Quote;

#[derive(Debug, Clone)]
pub struct QuoteTarget {
    pub bid: Option<Quote>,
    pub ask: Option<Quote>,
}
