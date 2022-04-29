use super::shared::string_to_decimal;
use rust_decimal::prelude::Decimal;
use serde::Deserialize;

/// This struct represents a level 2 snapshot record
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Level2SnapshotRecord {
    #[serde(with = "string_to_decimal")]
    pub price: Decimal,
    #[serde(with = "string_to_decimal")]
    pub size: Decimal,
}
