use super::shared::string_to_decimal;
use super::BookLevel;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;
use serde::Serialize;

/// This struct represents a level 2 book record
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookRecordL2 {
    #[serde(with = "string_to_decimal")]
    pub price: Decimal,
    #[serde(with = "string_to_decimal")]
    pub size: Decimal,
    pub num_orders: usize,
}

impl BookLevel for BookRecordL2 {
    fn level() -> u8 {
        2
    }
}
