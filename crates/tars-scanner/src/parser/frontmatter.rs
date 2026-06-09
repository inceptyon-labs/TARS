//! YAML frontmatter parser for skills, commands, and agents

use crate::artifacts::{AgentInfo, CommandInfo, HookDefinition, SkillInfo};
use crate::error::{ScanError, ScanResult};
use crate::runtime::{agent_runtime_support, command_runtime_support, skill_runtime_support};
use crate::types::Scope;
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Skill frontmatter structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    user_invocable: bool,
    #[serde(default)]
    disable_model_invocation: bool,
    #[serde(default, deserialize_with = "deserialize_tool_list")]
    allowed_tools: Vec<String>,
    model: Option<String>,
    context: Option<String>,
    agent: Option<String>,
    #[serde(default)]
    hooks: HashMap<String, Vec<HookDefinition>>,
}

/// Agent frontmatter structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AgentFrontmatter {
    name: String,
    description: String,
    #[serde(default, deserialize_with = "deserialize_tool_list")]
    tools: Vec<String>,
    model: Option<String>,
    #[serde(default = "default_permission_mode")]
    permission_mode: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    hooks: HashMap<String, Vec<HookDefinition>>,
}

fn default_permission_mode() -> String {
    "default".to_string()
}

/// Deserialize a tool list that may be either a YAML sequence
/// (`[Read, Grep]`) or a single string. Claude Code accepts the string form,
/// either comma-separated (`Read, Write, Edit`) or whitespace-separated
/// (`Bash Read Grep`), so we tolerate both.
fn deserialize_tool_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrSeq {
        Seq(Vec<String>),
        Str(String),
    }

    Ok(match StringOrSeq::deserialize(deserializer)? {
        StringOrSeq::Seq(items) => items,
        StringOrSeq::Str(s) => split_tool_string(&s),
    })
}

/// Split a tool string into individual tool names. Commas take precedence
/// (preserving names with internal spaces like `Bash(git add:*)`); otherwise
/// split on whitespace.
fn split_tool_string(s: &str) -> Vec<String> {
    if s.contains(',') {
        s.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect()
    } else {
        s.split_whitespace().map(String::from).collect()
    }
}

/// Command frontmatter structure
#[derive(Debug, Deserialize)]
struct CommandFrontmatter {
    description: Option<String>,
    #[serde(default)]
    thinking: bool,
}

/// Parse a SKILL.md file
///
/// # Errors
/// Returns an error if the file cannot be parsed
pub fn parse_skill(path: &Path, content: &str, scope: Scope) -> ScanResult<SkillInfo> {
    let matter = Matter::<YAML>::new();
    let result = matter.parse(content);

    let data = result
        .data
        .ok_or(ScanError::NoFrontmatter)?
        .deserialize::<SkillFrontmatter>()
        .map_err(|e| ScanError::FrontmatterParse(e.to_string()))?;

    let sha256 = compute_sha256(content);

    Ok(SkillInfo {
        path: path.to_path_buf(),
        name: data.name,
        description: data.description,
        user_invocable: data.user_invocable,
        disable_model_invocation: data.disable_model_invocation,
        allowed_tools: data.allowed_tools,
        model: data.model,
        context: data.context,
        agent: data.agent,
        hooks: data.hooks,
        sha256,
        runtime_support: skill_runtime_support(),
        scope,
    })
}

/// Parse an agent definition file
///
/// # Errors
/// Returns an error if the file cannot be parsed
pub fn parse_agent(path: &Path, content: &str, scope: Scope) -> ScanResult<AgentInfo> {
    let matter = Matter::<YAML>::new();
    let result = matter.parse(content);

    let data = result
        .data
        .ok_or(ScanError::NoFrontmatter)?
        .deserialize::<AgentFrontmatter>()
        .map_err(|e| ScanError::FrontmatterParse(e.to_string()))?;

    let sha256 = compute_sha256(content);

    Ok(AgentInfo {
        path: path.to_path_buf(),
        name: data.name,
        description: data.description,
        tools: data.tools,
        model: data.model,
        permission_mode: data.permission_mode,
        skills: data.skills,
        hooks: data.hooks,
        sha256,
        runtime_support: agent_runtime_support(),
        scope,
    })
}

/// Parse a command file
///
/// # Errors
/// Returns an error if the file cannot be parsed
pub fn parse_command(path: &Path, content: &str, scope: Scope) -> ScanResult<CommandInfo> {
    let matter = Matter::<YAML>::new();
    let result = matter.parse(content);

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let (description, thinking) = if let Some(data) = result.data {
        let fm = data
            .deserialize::<CommandFrontmatter>()
            .unwrap_or(CommandFrontmatter {
                description: None,
                thinking: false,
            });
        (fm.description, fm.thinking)
    } else {
        (None, false)
    };

    let sha256 = compute_sha256(content);

    Ok(CommandInfo {
        path: path.to_path_buf(),
        name,
        description,
        thinking,
        body: result.content,
        sha256,
        runtime_support: command_runtime_support(),
        scope,
    })
}

fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_skill() {
        let content = r"---
name: test-skill
description: A test skill
user-invocable: true
allowed-tools:
  - Read
  - Grep
---

# Test Skill Instructions
";
        let result = parse_skill(&PathBuf::from("test"), content, Scope::User);
        assert!(result.is_ok());
        let skill = result.unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.description, "A test skill");
        assert!(skill.user_invocable);
        assert_eq!(skill.allowed_tools, vec!["Read", "Grep"]);
    }

    #[test]
    fn test_parse_skill_space_separated_tools() {
        let content = r"---
name: tenet-security
description: A test skill
allowed-tools: Bash Read Grep Glob Write
---

# Instructions
";
        let skill = parse_skill(&PathBuf::from("test"), content, Scope::User).unwrap();
        assert_eq!(
            skill.allowed_tools,
            vec!["Bash", "Read", "Grep", "Glob", "Write"]
        );
    }

    #[test]
    fn test_parse_agent_comma_separated_tools() {
        let content = r"---
name: edit-applier
description: A test agent
tools: Read, Write, Edit, Bash, Glob, Grep
---

# Instructions
";
        let agent = parse_agent(&PathBuf::from("test.md"), content, Scope::User).unwrap();
        assert_eq!(
            agent.tools,
            vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep"]
        );
    }

    #[test]
    fn test_parse_command() {
        let content = r"---
description: A test command
thinking: true
---

Do something with $ARGUMENTS
";
        let result = parse_command(&PathBuf::from("test-cmd.md"), content, Scope::Project);
        assert!(result.is_ok());
        let cmd = result.unwrap();
        assert_eq!(cmd.name, "test-cmd");
        assert_eq!(cmd.description, Some("A test command".to_string()));
        assert!(cmd.thinking);
    }
}
