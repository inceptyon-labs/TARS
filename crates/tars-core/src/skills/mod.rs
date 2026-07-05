//! Cross-agent standalone skills: the filesystem deployment engine.
//!
//! [`storage::skill_library`](crate::storage::skill_library) persists *intent*
//! (which skill is deployed where); this module performs the *materialization*
//! — creating and removing the symlink (or copy) that puts a library skill into
//! an agent's skills directory. Keeping the two separate means the engine is
//! pure filesystem work and can be exercised with tempdirs.

pub mod deploy;
pub mod install;
pub mod scan;

pub use deploy::{
    codex_user_skills_dir, deploy, hash_bundle, repoint_symlink, resolve_skills_dir, resync_copy,
    undeploy, Agent, DeployResult, LinkKind, Scope, SkillDeployError,
};
pub use install::{
    external_skills_dir, find_skill_bundles, install_bundles, parse_git_skill_url, GitSkillSource,
    SkillInstallError, SkillInstallReport,
};
pub use scan::{
    probe_target, scan_external_dir, scan_source, scan_sources, symlink_points_to, CatalogSkill,
    TargetProbe,
};
