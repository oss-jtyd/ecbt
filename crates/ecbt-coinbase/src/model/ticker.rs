use super::shared::iso8601_datetime_from_string;
use super::shared::string_to_decimal;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

/// This struct represents a ticker
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ticker {
    pub trade_id: i64,
    #[serde(with = "string_to_decimal")]
    pub price: Decimal,
    #[serde(with = "string_to_decimal")]
    pub size: Decimal,
    #[serde(with = "string_to_decimal")]
    pub bid: Decimal,
    #[serde(with = "string_to_decimal")]
    pub ask: Decimal,
    #[serde(with = "string_to_decimal")]
    pub volume: Decimal,
    #[serde(with = "iso8601_datetime_from_string")]
    pub time: OffsetDateTime,
}
