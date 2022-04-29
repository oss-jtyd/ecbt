use ecbt_exchange::model::websocket::Subscription;

/// This enum represents a coinbase subscription
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoinbaseSubscription {
    Heartbeat(String),
    // Status,
    // Ticker(String),
    Level2(String),
    // User,
    Matches(String),
    // FullChannel
}

impl From<Subscription> for CoinbaseSubscription {
    fn from(subscription: Subscription) -> Self {
        match subscription {
            Subscription::OrderBookUpdates(symbol) => {
                CoinbaseSubscription::Level2(crate::model::MarketPair::from(symbol).0)
            }
            Subscription::Trades(symbol) => {
                CoinbaseSubscription::Matches(crate::model::MarketPair::from(symbol).0)
            } // Subscription::Ticker(ticket) =>
        }
    }
}
