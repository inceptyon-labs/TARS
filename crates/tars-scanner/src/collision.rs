//! Collision detection types

use crate::types::Scope;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Report of detected collisions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollisionReport {
    /// Skill name collisions
    #[serde(default)]
    pub skills: Vec<Collision>,
    /// Command name collisions
    #[serde(default)]
    pub commands: Vec<Collision>,
    /// Agent name collisions
    #[serde(default)]
    pub agents: Vec<Collision>,
}

/// A single collision (same name in multiple scopes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collision {
    /// The colliding name
    pub name: String,
    /// Scope of the winner (highest precedence)
    pub winner_scope: Scope,
    /// All occurrences
    pub occurrences: Vec<CollisionOccurrence>,
}

/// A single occurrence of a colliding name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionOccurrence {
    /// Scope where found
    pub scope: Scope,
    /// Path to the artifact
    pub path: PathBuf,
}

impl CollisionReport {
    /// Check if there are any collisions
    #[must_use]
    pub fn has_collisions(&self) -> bool {
        !self.skills.is_empty() || !self.commands.is_empty() || !self.agents.is_empty()
    }

    /// Get total number of collisions
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.skills.len() + self.commands.len() + self.agents.len()
    }
}
