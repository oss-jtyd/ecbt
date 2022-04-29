use super::shared::string_to_decimal;
use super::OrderSide;
use super::Reason;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Done {
    Limit {
        time: String,
        product_id: String,
        sequence: Option<usize>,
        #[serde(with = "string_to_decimal")]
        price: Decimal,
        order_id: String,
        reason: Reason,
        side: OrderSide,
        #[serde(with = "string_to_decimal")]
        remaining_size: Decimal,
        user_id: Option<String>,
        #[serde(default)]
        profile_id: Option<String>,
    },
    Market {
        time: String,
        product_id: String,
        sequence: usize,
        order_id: String,
        reason: Reason,
        side: OrderSide,
    },
}
