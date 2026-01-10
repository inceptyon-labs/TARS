import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  BookOpen,
  Zap,
  Bot,
  Terminal,
  Webhook,
  Server,
  Package,
  ExternalLink,
  ChevronRight,
  Search
} from 'lucide-react';

interface KnowledgeSection {
  id: string;
  title: string;
  icon: React.ReactNode;
  description: string;
  content: React.ReactNode;
  docs?: { label: string; url: string }[];
}

const sections: KnowledgeSection[] = [
  {
    id: 'skills',
    title: 'Skills',
    icon: <Zap className="h-5 w-5" />,
    description: 'Reusable prompt templates that can be invoked with /skill-name',
    content: (
      <div className="space-y-4">
        <p>
          Skills are markdown files with YAML frontmatter that define reusable prompts.
          They live in <code className="px-1.5 py-0.5 bg-muted rounded text-sm">~/.claude/skills/</code> (user)
          or <code className="px-1.5 py-0.5 bg-muted rounded text-sm">.claude/skills/</code> (project).
        </p>

        <h4 className="font-semibold mt-6">Structure</h4>
        <pre className="bg-secondary text-secondary-foreground p-4 rounded-lg text-sm overflow-x-auto font-mono">{`---
name: my-skill
description: What this skill does
user_invocable: true
allowed_tools:
  - Read
  - Write
  - Edit
---

# Skill Instructions

Your prompt template goes here.
Use $ARGUMENTS to reference user input.`}</pre>

        <h4 className="font-semibold mt-6">Key Properties</h4>
        <ul className="list-disc list-inside space-y-2 text-muted-foreground">
          <li><strong>name</strong> - Unique identifier, used as /name to invoke</li>
          <li><strong>description</strong> - Shown in skill listings and help</li>
          <li><strong>user_invocable</strong> - If true, users can call with /name</li>
          <li><strong>allowed_tools</strong> - Restrict which tools the skill can use</li>
          <li><strong>model</strong> - Override the default model (e.g., "haiku" for fast tasks)</li>
        </ul>

        <h4 className="font-semibold mt-6">Scope Precedence</h4>
        <p className="text-muted-foreground">
          When the same skill name exists in multiple scopes, the most specific wins:
          <br />
          <span className="text-primary">Managed → Local → Project → User → Plugin</span>
        </p>
      </div>
    ),
    docs: [
      { label: 'Claude Code Skills Documentation', url: 'https://docs.anthropic.com/en/docs/claude-code/skills' },
    ],
  },
  {
    id: 'agents',
    title: 'Agents',
    icon: <Bot className="h-5 w-5" />,
    description: 'Specialized task handlers with their own tools and capabilities',
    content: (
      <div className="space-y-4">
        <p>
          Agents are autonomous task handlers that can be spawned via the Task tool.
          They have their own set of allowed tools and can work independently.
          Located in <code className="px-1.5 py-0.5 bg-muted rounded text-sm">~/.claude/agents/</code> or <code className="px-1.5 py-0.5 bg-muted rounded text-sm">.claude/agents/</code>.
        </p>

        <h4 className="font-semibold mt-6">Structure</h4>
        <pre className="bg-secondary text-secondary-foreground p-4 rounded-lg text-sm overflow-x-auto font-mono">{`---
name: code-reviewer
description: Reviews code for quality and security
tools:
  - Read
  - Glob
  - Grep
model: sonnet
---

# Code Reviewer Agent

You are an expert code reviewer. When given code to review:

1. Check for security vulnerabilities
2. Identify performance issues
3. Suggest improvements
4. Note any best practice violations`}</pre>

        <h4 className="font-semibold mt-6">Key Properties</h4>
        <ul className="list-disc list-inside space-y-2 text-muted-foreground">
          <li><strong>name</strong> - Identifier used when spawning the agent</li>
          <li><strong>description</strong> - Explains what the agent does</li>
          <li><strong>tools</strong> - List of tools the agent can use</li>
          <li><strong>model</strong> - Which model to use (sonnet, opus, haiku)</li>
          <li><strong>skills</strong> - Skills the agent can invoke</li>
        </ul>

        <h4 className="font-semibold mt-6">Spawning Agents</h4>
        <p className="text-muted-foreground">
          Agents are spawned using the Task tool with <code className="px-1.5 py-0.5 bg-muted rounded text-sm">subagent_type</code>
          matching the agent name. They run autonomously and return results when complete.
        </p>
      </div>
    ),
    docs: [
      { label: 'Claude Code Agents Documentation', url: 'https://docs.anthropic.com/en/docs/claude-code/agents' },
    ],
  },
  {
    id: 'commands',
    title: 'Commands',
    icon: <Terminal className="h-5 w-5" />,
    description: 'Custom slash commands that extend Claude Code functionality',
    content: (
      <div className="space-y-4">
        <p>
          Commands are custom slash commands you can create. Unlike skills, commands
          are simpler and don't have the full frontmatter options.
          Located in <code className="px-1.5 py-0.5 bg-muted rounded text-sm">~/.claude/commands/</code> or <code className="px-1.5 py-0.5 bg-muted rounded text-sm">.claude/commands/</code>.
        </p>

        <h4 className="font-semibold mt-6">Structure</h4>
        <pre className="bg-secondary text-secondary-foreground p-4 rounded-lg text-sm overflow-x-auto font-mono">{`---
description: Generate a git commit message
---

Analyze the staged changes and generate a commit message following
conventional commits format. Use $ARGUMENTS for any specific instructions.`}</pre>

        <h4 className="font-semibold mt-6">Commands vs Skills</h4>
        <table className="w-full text-sm mt-4">
          <thead>
            <tr className="border-b border-border">
              <th className="text-left py-2">Feature</th>
              <th className="text-left py-2">Commands</th>
              <th className="text-left py-2">Skills</th>
            </tr>
          </thead>
          <tbody className="text-muted-foreground">
            <tr className="border-b border-border/50">
              <td className="py-2">Tool restrictions</td>
              <td className="py-2">No</td>
              <td className="py-2">Yes (allowed_tools)</td>
            </tr>
            <tr className="border-b border-border/50">
              <td className="py-2">Model override</td>
              <td className="py-2">No</td>
              <td className="py-2">Yes</td>
            </tr>
            <tr className="border-b border-border/50">
              <td className="py-2">Complexity</td>
              <td className="py-2">Simple</td>
              <td className="py-2">Full-featured</td>
            </tr>
          </tbody>
        </table>
      </div>
    ),
    docs: [
      { label: 'Claude Code Commands Documentation', url: 'https://docs.anthropic.com/en/docs/claude-code/commands' },
    ],
  },
  {
    id: 'hooks',
    title: 'Hooks',
    icon: <Webhook className="h-5 w-5" />,
    description: 'Automated actions triggered by Claude Code events',
    content: (
      <div className="space-y-4">
        <p>
          Hooks let you run shell commands or inject prompts in response to Claude Code events.
          Configured in <code className="px-1.5 py-0.5 bg-muted rounded text-sm">settings.json</code> under the <code className="px-1.5 py-0.5 bg-muted rounded text-sm">hooks</code> key.
        </p>

        <h4 className="font-semibold mt-6">Hook Events</h4>
        <ul className="list-disc list-inside space-y-2 text-muted-foreground">
          <li><strong>PreToolUse</strong> - Before a tool is executed</li>
          <li><strong>PostToolUse</strong> - After a tool completes</li>
          <li><strong>Notification</strong> - When Claude sends a notification</li>
          <li><strong>Stop</strong> - When Claude stops processing</li>
          <li><strong>SubagentStop</strong> - When a subagent finishes</li>
        </ul>

        <h4 className="font-semibold mt-6">Configuration Example</h4>
        <pre className="bg-secondary text-secondary-foreground p-4 rounded-lg text-sm overflow-x-auto font-mono">{`{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'File modification starting'"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "notify-send 'Command completed'"
          }
        ]
      }
    ]
  }
}`}</pre>

        <h4 className="font-semibold mt-6">Hook Types</h4>
        <ul className="list-disc list-inside space-y-2 text-muted-foreground">
          <li><strong>command</strong> - Run a shell command</li>
          <li><strong>prompt</strong> - Inject a prompt into the conversation</li>
        </ul>
      </div>
    ),
    docs: [
      { label: 'Claude Code Hooks Documentation', url: 'https://docs.anthropic.com/en/docs/claude-code/hooks' },
    ],
  },
  {
    id: 'mcp',
    title: 'MCP Servers',
    icon: <Server className="h-5 w-5" />,
    description: 'Model Context Protocol servers that extend Claude\'s capabilities',
    content: (
      <div className="space-y-4">
        <p>
          MCP (Model Context Protocol) servers provide additional tools and resources to Claude.
          Configured in <code className="px-1.5 py-0.5 bg-muted rounded text-sm">~/.claude.json</code> (user)
          or <code className="px-1.5 py-0.5 bg-muted rounded text-sm">.mcp.json</code> (project).
        </p>

        <h4 className="font-semibold mt-6">Server Types</h4>
        <ul className="list-disc list-inside space-y-2 text-muted-foreground">
          <li><strong>stdio</strong> - Local process communication via stdin/stdout</li>
          <li><strong>sse</strong> - Server-Sent Events over HTTP</li>
          <li><strong>http</strong> - HTTP-based communication</li>
        </ul>

        <h4 className="font-semibold mt-6">Configuration Example</h4>
        <pre className="bg-secondary text-secondary-foreground p-4 rounded-lg text-sm overflow-x-auto font-mono">{`{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-server-filesystem"],
      "env": {
        "ALLOWED_PATHS": "/Users/me/projects"
      }
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-server-github"],
      "env": {
        "GITHUB_TOKEN": "your-token"
      }
    }
  }
}`}</pre>

        <h4 className="font-semibold mt-6">Popular MCP Servers</h4>
        <ul className="list-disc list-inside space-y-2 text-muted-foreground">
          <li><strong>filesystem</strong> - Extended file operations</li>
          <li><strong>github</strong> - GitHub API integration</li>
          <li><strong>postgres</strong> - Database queries</li>
          <li><strong>puppeteer</strong> - Browser automation</li>
        </ul>
      </div>
    ),
    docs: [
      { label: 'Model Context Protocol', url: 'https://modelcontextprotocol.io/' },
      { label: 'MCP Servers Registry', url: 'https://github.com/modelcontextprotocol/servers' },
    ],
  },
  {
    id: 'plugins',
    title: 'Plugins',
    icon: <Package className="h-5 w-5" />,
    description: 'Packaged collections of skills, agents, commands, and hooks',
    content: (
      <div className="space-y-4">
        <p>
          Plugins bundle skills, agents, commands, hooks, and MCP servers into distributable packages.
          They can be installed from marketplaces or local directories.
        </p>

        <h4 className="font-semibold mt-6">Plugin Structure</h4>
        <pre className="bg-secondary text-secondary-foreground p-4 rounded-lg text-sm overflow-x-auto font-mono">{`.claude-plugin/
├── plugin.json       # Manifest file
├── skills/          # Skill definitions
│   └── my-skill.md
├── agents/          # Agent definitions
│   └── my-agent.md
├── commands/        # Command definitions
│   └── my-command.md
└── hooks.json       # Hook configurations`}</pre>

        <h4 className="font-semibold mt-6">Plugin Manifest (plugin.json)</h4>
        <pre className="bg-secondary text-secondary-foreground p-4 rounded-lg text-sm overflow-x-auto font-mono">{`{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "My awesome plugin",
  "author": {
    "name": "Your Name"
  },
  "skills": "skills/",
  "agents": "agents/",
  "commands": "commands/",
  "hooks": "hooks.json"
}`}</pre>

        <h4 className="font-semibold mt-6">Installation Scopes</h4>
        <ul className="list-disc list-inside space-y-2 text-muted-foreground">
          <li><strong>User</strong> - Available in all projects (~/.claude/plugins/)</li>
          <li><strong>Project</strong> - Only in specific project (.claude/plugins/)</li>
          <li><strong>Managed</strong> - System-wide, admin-controlled</li>
        </ul>

        <h4 className="font-semibold mt-6">Marketplaces</h4>
        <p className="text-muted-foreground">
          Plugins can be installed from GitHub repositories or local directories.
          Marketplaces are configured in <code className="px-1.5 py-0.5 bg-muted rounded text-sm">~/.claude/plugins/known_marketplaces.json</code>.
        </p>
      </div>
    ),
    docs: [
      { label: 'Claude Code Plugins Documentation', url: 'https://docs.anthropic.com/en/docs/claude-code/plugins' },
    ],
  },
];

export function CasePage() {
  const { section } = useParams<{ section?: string }>();
  const navigate = useNavigate();
  const [selectedSection, setSelectedSection] = useState<string>(section || 'skills');
  const [searchQuery, setSearchQuery] = useState('');

  // Sync URL param with selected section
  useEffect(() => {
    if (section && sections.find(s => s.id === section)) {
      setSelectedSection(section);
    }
  }, [section]);

  const handleSelectSection = (id: string) => {
    setSelectedSection(id);
    navigate(`/case/${id}`, { replace: true });
  };

  const filteredSections = searchQuery
    ? sections.filter(
        (s) =>
          s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
          s.description.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : sections;

  const currentSection = sections.find((s) => s.id === selectedSection);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 brushed-metal relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">CASE</h2>
          <span className="text-xs text-muted-foreground">Knowledge Base</span>
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Navigation sidebar */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="p-3 border-b border-border">
            <div className="relative flex items-center">
              <input
                type="search"
                placeholder="Search topics..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="tars-input w-full pl-9 pr-3 py-2 text-sm rounded"
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="off"
                spellCheck={false}
              />
              <Search className="absolute left-3 h-4 w-4 text-muted-foreground pointer-events-none" />
            </div>
          </div>

          <div className="tars-segment-line" />

          <div className="flex-1 overflow-auto p-3">
            <div className="mb-4">
              <h3 className="text-xs font-semibold text-primary uppercase tracking-wider px-3 py-2 border-b border-primary/20 mb-2">
                Topics
              </h3>
              <ul className="space-y-1">
                {filteredSections.map((sec) => (
                  <li key={sec.id}>
                    <button
                      onClick={() => handleSelectSection(sec.id)}
                      className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all ${
                        selectedSection === sec.id
                          ? 'active text-foreground font-medium'
                          : 'text-muted-foreground hover:text-foreground'
                      }`}
                    >
                      <div className="flex items-center gap-2">
                        {sec.icon}
                        <span className="font-medium">{sec.title}</span>
                      </div>
                      <div className="text-xs opacity-60 truncate mt-0.5 ml-7">
                        {sec.description}
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>

        {/* Content area */}
        <div className="flex-1 overflow-auto bg-background">
          {currentSection ? (
            <div className="max-w-3xl mx-auto p-8">
              <div className="flex items-center gap-3 mb-2">
                <div className="p-2 rounded-lg bg-primary/10 text-primary">
                  {currentSection.icon}
                </div>
                <h1 className="text-2xl font-bold">{currentSection.title}</h1>
              </div>
              <p className="text-muted-foreground mb-8">{currentSection.description}</p>

              <div className="prose dark:prose-invert max-w-none">
                {currentSection.content}
              </div>

              {currentSection.docs && currentSection.docs.length > 0 && (
                <div className="mt-8 pt-6 border-t border-border">
                  <h4 className="text-sm font-semibold mb-3 flex items-center gap-2">
                    <BookOpen className="h-4 w-4" />
                    Official Documentation
                  </h4>
                  <div className="space-y-2">
                    {currentSection.docs.map((doc) => (
                      <a
                        key={doc.url}
                        href={doc.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex items-center gap-2 text-sm text-primary hover:underline"
                      >
                        <ExternalLink className="h-3.5 w-3.5" />
                        {doc.label}
                        <ChevronRight className="h-3.5 w-3.5 opacity-50" />
                      </a>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="w-20 h-20 rounded-lg tars-panel flex items-center justify-center">
                <BookOpen className="h-10 w-10 text-muted-foreground/50" />
              </div>
              <div className="text-center">
                <p className="text-sm text-muted-foreground">
                  Select a topic to learn more
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// Export section IDs for linking from other pages
export const CASE_SECTIONS = {
  SKILLS: 'skills',
  AGENTS: 'agents',
  COMMANDS: 'commands',
  HOOKS: 'hooks',
  MCP: 'mcp',
  PLUGINS: 'plugins',
} as const;
