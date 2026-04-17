//! AI provider integrations for TARS.
//!
//! Provides a uniform `Provider` trait that each supported AI provider
//! implements, plus a static metadata registry. Real provider impls (HTTP
//! calls against each vendor API) land in a follow-up task; this crate
//! currently supplies the trait, types, and metadata used by the storage
//! and UI layers.

pub mod error;
pub mod provider;
pub mod registry;
pub mod types;

pub use error::ProviderError;
pub use provider::Provider;
pub use registry::{all_metadata, metadata_for};
pub use types::{Balance, ModelInfo, ProviderId, ProviderMetadata, ValidationResult};
