//! JSON output formatter

use crate::error::ScanResult;
use crate::inventory::Inventory;

/// Convert inventory to JSON string
///
/// # Errors
/// Returns an error if serialization fails
pub fn to_json(inventory: &Inventory) -> ScanResult<String> {
    serde_json::to_string_pretty(inventory).map_err(Into::into)
}
