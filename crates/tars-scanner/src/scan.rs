//! Main scanner implementation

use crate::collision::{Collision, CollisionOccurrence, CollisionReport};
use crate::error::ScanResult;
use crate::inventory::{Inventory, ManagedScope, ProjectScope, UserScope};
use crate::plugins::PluginInventory;
use crate::scope::{managed, project, user};
use crate::types::HostInfo;
use chrono::Utc;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// The main scanner struct
#[derive(Debug, Default)]
pub struct Scanner {
    /// Whether to include managed scope
    pub include_managed: bool,
}

impl Scanner {
    /// Create a new scanner
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable scanning managed scope
    #[must_use]
    pub fn with_managed(mut self, include: bool) -> Self {
        self.include_managed = include;
        self
    }

    /// Perform a full inventory scan
    ///
    /// # Errors
    /// Returns an error if scanning fails
    pub fn scan_all(&self, project_paths: &[&Path]) -> ScanResult<Inventory> {
        let host = HostInfo::current();

        // Scan plugins first (only once) to share with user scope
        let plugins = self.scan_plugins()?;

        // Pass plugins to user scope to avoid duplicate scanning
        let user_scope = user::scan_user_scope_with_plugins(&plugins)?;

        let managed_scope = if self.include_managed {
            self.scan_managed_scope()?
        } else {
            None
        };

        // Scan projects in parallel using rayon
        let projects: Vec<ProjectScope> = project_paths
            .par_iter()
            .filter_map(|path| match self.scan_project(path) {
                Ok(proj) => Some(proj),
                Err(e) => {
                    eprintln!("Warning: Failed to scan project {path:?}: {e}");
                    None
                }
            })
            .collect();

        let collisions = self.detect_collisions(&user_scope, &managed_scope, &projects, &plugins);

        Ok(Inventory {
            host,
            user_scope,
            managed_scope,
            projects,
            plugins,
            collisions,
            scanned_at: Utc::now(),
        })
    }

    /// Scan user-level scope
    ///
    /// # Errors
    /// Returns an error if scanning fails
    pub fn scan_user_scope(&self) -> ScanResult<UserScope> {
        user::scan_user_scope()
    }

    /// Scan managed scope
    ///
    /// # Errors
    /// Returns an error if scanning fails
    pub fn scan_managed_scope(&self) -> ScanResult<Option<ManagedScope>> {
        managed::scan_managed_scope()
    }

    /// Scan a project directory
    ///
    /// # Errors
    /// Returns an error if scanning fails
    pub fn scan_project(&self, path: &Path) -> ScanResult<ProjectScope> {
        project::scan_project(path)
    }

    /// Scan installed plugins from Claude Code plugins directory
    ///
    /// # Errors
    /// Returns an error if scanning fails
    pub fn scan_plugins(&self) -> ScanResult<PluginInventory> {
        PluginInventory::scan()
    }

    /// Detect collisions across all scopes
    fn detect_collisions(
        &self,
        user_scope: &UserScope,
        _managed_scope: &Option<ManagedScope>,
        projects: &[ProjectScope],
        _plugins: &PluginInventory,
    ) -> CollisionReport {
        use crate::types::Scope;

        // Track names and their occurrences
        let mut skill_occurrences: HashMap<String, Vec<CollisionOccurrence>> = HashMap::new();
        let mut command_occurrences: HashMap<String, Vec<CollisionOccurrence>> = HashMap::new();
        let mut agent_occurrences: HashMap<String, Vec<CollisionOccurrence>> = HashMap::new();

        // Collect from user scope
        for skill in &user_scope.skills {
            skill_occurrences
                .entry(skill.name.clone())
                .or_default()
                .push(CollisionOccurrence {
                    scope: Scope::User,
                    path: skill.path.clone(),
                });
        }
        for cmd in &user_scope.commands {
            command_occurrences
                .entry(cmd.name.clone())
                .or_default()
                .push(CollisionOccurrence {
                    scope: Scope::User,
                    path: cmd.path.clone(),
                });
        }
        for agent in &user_scope.agents {
            agent_occurrences
                .entry(agent.name.clone())
                .or_default()
                .push(CollisionOccurrence {
                    scope: Scope::User,
                    path: agent.path.clone(),
                });
        }

        // Collect from projects
        for proj in projects {
            for skill in &proj.skills {
                skill_occurrences
                    .entry(skill.name.clone())
                    .or_default()
                    .push(CollisionOccurrence {
                        scope: Scope::Project,
                        path: skill.path.clone(),
                    });
            }
            for cmd in &proj.commands {
                command_occurrences
                    .entry(cmd.name.clone())
                    .or_default()
                    .push(CollisionOccurrence {
                        scope: Scope::Project,
                        path: cmd.path.clone(),
                    });
            }
            for agent in &proj.agents {
                agent_occurrences
                    .entry(agent.name.clone())
                    .or_default()
                    .push(CollisionOccurrence {
                        scope: Scope::Project,
                        path: agent.path.clone(),
                    });
            }
        }

        // Helper to determine winner scope based on precedence
        // Precedence: Managed > Local > Project > User > Plugin
        fn determine_winner(occurrences: &[CollisionOccurrence]) -> Scope {
            for occ in occurrences {
                if matches!(occ.scope, Scope::Managed) {
                    return Scope::Managed;
                }
            }
            for occ in occurrences {
                if matches!(occ.scope, Scope::Local) {
                    return Scope::Local;
                }
            }
            for occ in occurrences {
                if matches!(occ.scope, Scope::Project) {
                    return Scope::Project;
                }
            }
            for occ in occurrences {
                if matches!(occ.scope, Scope::User) {
                    return Scope::User;
                }
            }
            Scope::User
        }

        // Build collision reports (only for names with multiple occurrences)
        let skills: Vec<Collision> = skill_occurrences
            .into_iter()
            .filter(|(_, occs)| occs.len() > 1)
            .map(|(name, occurrences)| Collision {
                winner_scope: determine_winner(&occurrences),
                name,
                occurrences,
            })
            .collect();

        let commands: Vec<Collision> = command_occurrences
            .into_iter()
            .filter(|(_, occs)| occs.len() > 1)
            .map(|(name, occurrences)| Collision {
                winner_scope: determine_winner(&occurrences),
                name,
                occurrences,
            })
            .collect();

        let agents: Vec<Collision> = agent_occurrences
            .into_iter()
            .filter(|(_, occs)| occs.len() > 1)
            .map(|(name, occurrences)| Collision {
                winner_scope: determine_winner(&occurrences),
                name,
                occurrences,
            })
            .collect();

        CollisionReport {
            skills,
            commands,
            agents,
        }
    }
}
