//! Markdown output formatter

use crate::inventory::Inventory;

/// Convert inventory to Markdown report
#[must_use]
pub fn to_markdown(inventory: &Inventory) -> String {
    let mut output = String::new();

    output.push_str("# TARS Inventory Report\n\n");
    output.push_str(&format!(
        "**Scanned at:** {}\n\n",
        inventory.scanned_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Host info
    output.push_str("## Host\n\n");
    output.push_str(&format!("- **OS:** {}\n", inventory.host.os));
    output.push_str(&format!("- **User:** {}\n", inventory.host.username));
    output.push_str(&format!(
        "- **Home:** {}\n\n",
        inventory.host.home_dir.display()
    ));

    // User scope
    output.push_str("## User Scope\n\n");
    output.push_str(&format!(
        "- **Skills:** {}\n",
        inventory.user_scope.skills.len()
    ));
    output.push_str(&format!(
        "- **Commands:** {}\n",
        inventory.user_scope.commands.len()
    ));
    output.push_str(&format!(
        "- **Agents:** {}\n\n",
        inventory.user_scope.agents.len()
    ));

    // Projects
    output.push_str("## Projects\n\n");
    if inventory.projects.is_empty() {
        output.push_str("_No projects scanned_\n\n");
    } else {
        for project in &inventory.projects {
            output.push_str(&format!("### {}\n\n", project.name));
            output.push_str(&format!("- **Path:** {}\n", project.path.display()));
            output.push_str(&format!("- **Skills:** {}\n", project.skills.len()));
            output.push_str(&format!("- **Commands:** {}\n", project.commands.len()));
            output.push_str(&format!("- **Agents:** {}\n\n", project.agents.len()));
        }
    }

    // Plugins
    output.push_str("## Plugins\n\n");
    output.push_str(&format!(
        "- **Marketplaces:** {}\n",
        inventory.plugins.marketplaces.len()
    ));
    output.push_str(&format!(
        "- **Installed:** {}\n\n",
        inventory.plugins.installed.len()
    ));

    // Collisions
    output.push_str("## Collisions\n\n");
    if inventory.collisions.has_collisions() {
        output.push_str(&format!(
            "**Total:** {} collisions detected\n\n",
            inventory.collisions.total_count()
        ));

        if !inventory.collisions.skills.is_empty() {
            output.push_str("### Skill Collisions\n\n");
            for collision in &inventory.collisions.skills {
                output.push_str(&format!(
                    "- **{}** (winner: {:?})\n",
                    collision.name, collision.winner_scope
                ));
            }
            output.push('\n');
        }

        if !inventory.collisions.commands.is_empty() {
            output.push_str("### Command Collisions\n\n");
            for collision in &inventory.collisions.commands {
                output.push_str(&format!(
                    "- **{}** (winner: {:?})\n",
                    collision.name, collision.winner_scope
                ));
            }
            output.push('\n');
        }

        if !inventory.collisions.agents.is_empty() {
            output.push_str("### Agent Collisions\n\n");
            for collision in &inventory.collisions.agents {
                output.push_str(&format!(
                    "- **{}** (winner: {:?})\n",
                    collision.name, collision.winner_scope
                ));
            }
            output.push('\n');
        }
    } else {
        output.push_str("_No collisions detected_\n\n");
    }

    output
}
