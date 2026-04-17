//! Pricing data ingestion and storage.
//!
//! - [`litellm`]: parse `LiteLLM`'s `model_prices_and_context_window.json` into
//!   per-`(provider_id, model_id)` price entries.
//! - [`cache`]: persist parsed prices into the existing `provider_models` cache
//!   without disturbing user overrides, and track refresh metadata.
//!
//! HTTP fetching lives in the application layer (`tars-desktop`) so this crate
//! stays free of network dependencies.

pub mod cache;
pub mod litellm;

pub use cache::{
    delete_metadata, effective_price_for, get_metadata, set_metadata, update_prices,
    EffectivePrice, PriceUpdateRow, PricingMetadata, METADATA_KEY_LAST_ERROR,
    METADATA_KEY_LAST_REFRESH,
};
pub use litellm::{parse_litellm_prices, ParsedPrice, LITELLM_PRICES_URL};
