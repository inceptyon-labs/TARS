//! Runtime compatibility metadata for scanned artifacts.

use serde::{Deserialize, Serialize};

/// Canonical runtime identifier used across discovery and UI layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Runtime {
    ClaudeCode,
    Codex,
    Universal,
}

/// Compatibility level for a given runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeSupport {
    Native,
    Convertible,
    Partial,
    Unsupported,
}

/// Compatibility metadata for one runtime target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCompatibility {
    pub runtime: Runtime,
    pub support: RuntimeSupport,
}

#[must_use]
pub fn skill_runtime_support() -> Vec<RuntimeCompatibility> {
    vec![
        RuntimeCompatibility {
            runtime: Runtime::ClaudeCode,
            support: RuntimeSupport::Native,
        },
        RuntimeCompatibility {
            runtime: Runtime::Codex,
            support: RuntimeSupport::Convertible,
        },
    ]
}

#[must_use]
pub fn codex_skill_runtime_support() -> Vec<RuntimeCompatibility> {
    vec![
        RuntimeCompatibility {
            runtime: Runtime::ClaudeCode,
            support: RuntimeSupport::Convertible,
        },
        RuntimeCompatibility {
            runtime: Runtime::Codex,
            support: RuntimeSupport::Native,
        },
    ]
}

#[must_use]
pub fn agent_runtime_support() -> Vec<RuntimeCompatibility> {
    vec![
        RuntimeCompatibility {
            runtime: Runtime::ClaudeCode,
            support: RuntimeSupport::Native,
        },
        RuntimeCompatibility {
            runtime: Runtime::Codex,
            support: RuntimeSupport::Convertible,
        },
    ]
}

#[must_use]
pub fn codex_agent_runtime_support() -> Vec<RuntimeCompatibility> {
    vec![
        RuntimeCompatibility {
            runtime: Runtime::ClaudeCode,
            support: RuntimeSupport::Unsupported,
        },
        RuntimeCompatibility {
            runtime: Runtime::Codex,
            support: RuntimeSupport::Native,
        },
    ]
}

#[must_use]
pub fn command_runtime_support() -> Vec<RuntimeCompatibility> {
    vec![
        RuntimeCompatibility {
            runtime: Runtime::ClaudeCode,
            support: RuntimeSupport::Native,
        },
        RuntimeCompatibility {
            runtime: Runtime::Codex,
            support: RuntimeSupport::Unsupported,
        },
    ]
}

#[must_use]
pub fn hook_runtime_support() -> Vec<RuntimeCompatibility> {
    vec![
        RuntimeCompatibility {
            runtime: Runtime::ClaudeCode,
            support: RuntimeSupport::Native,
        },
        RuntimeCompatibility {
            runtime: Runtime::Codex,
            support: RuntimeSupport::Partial,
        },
    ]
}

#[must_use]
pub fn mcp_runtime_support() -> Vec<RuntimeCompatibility> {
    vec![
        RuntimeCompatibility {
            runtime: Runtime::ClaudeCode,
            support: RuntimeSupport::Native,
        },
        RuntimeCompatibility {
            runtime: Runtime::Codex,
            support: RuntimeSupport::Convertible,
        },
    ]
}
