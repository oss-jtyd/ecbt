use super::shared::string_to_decimal;
use super::BookLevel;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;
use serde::Serialize;

/// This struct represents a level 3 book record
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookRecordL3 {
    #[serde(with = "string_to_decimal")]
    pub price: Decimal,
    #[serde(with = "string_to_decimal")]
    pub size: Decimal,
    pub order_id: String,
}

impl BookLevel for BookRecordL3 {
    fn level() -> u8 {
        3
    }
}
