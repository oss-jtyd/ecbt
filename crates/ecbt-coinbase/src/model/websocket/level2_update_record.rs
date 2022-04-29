use super::shared::string_to_decimal;
use super::OrderSide;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;

/// This struct represents the level 2 update record
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Level2UpdateRecord {
    pub side: OrderSide,
    #[serde(with = "string_to_decimal")]
    pub price: Decimal,
    #[serde(with = "string_to_decimal")]
    pub size: Decimal,
}
