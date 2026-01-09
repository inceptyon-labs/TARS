import { useState } from 'react';
import { Sparkles, Terminal, Bot, Globe, AlertTriangle } from 'lucide-react';
import type { Inventory } from '../lib/types';
import { CollisionBadge } from './CollisionBadge';

interface InventoryViewProps {
  inventory: Inventory;
}

type Tab = 'skills' | 'commands' | 'agents' | 'mcp' | 'collisions';

export function InventoryView({ inventory }: InventoryViewProps) {
  const [activeTab, setActiveTab] = useState<Tab>('skills');

  const userScope = inventory.user_scope;
  const collisions = inventory.collisions;

  const totalCollisions =
    collisions.skills.length + collisions.commands.length + collisions.agents.length;

  const tabs: { id: Tab; label: string; icon: React.ElementType; count: number }[] = [
    { id: 'skills', label: 'Skills', icon: Sparkles, count: userScope.skills.length },
    { id: 'commands', label: 'Commands', icon: Terminal, count: userScope.commands.length },
    { id: 'agents', label: 'Agents', icon: Bot, count: userScope.agents.length },
    { id: 'mcp', label: 'MCP Servers', icon: Globe, count: userScope.mcp?.servers.length || 0 },
    { id: 'collisions', label: 'Collisions', icon: AlertTriangle, count: totalCollisions },
  ];

  return (
    <div className="h-full flex flex-col">
      {/* Tabs */}
      <div className="flex border-b">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex items-center gap-2 px-4 py-2 text-sm border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            <tab.icon className="h-4 w-4" />
            {tab.label}
            {tab.count > 0 && (
              <span
                className={`px-1.5 py-0.5 rounded-full text-xs ${
                  tab.id === 'collisions' && tab.count > 0
                    ? 'bg-destructive/10 text-destructive'
                    : 'bg-muted'
                }`}
              >
                {tab.count}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-auto p-4">
        {activeTab === 'skills' && (
          <div className="space-y-2">
            {userScope.skills.length === 0 ? (
              <p className="text-muted-foreground">No skills found</p>
            ) : (
              userScope.skills.map((skill) => (
                <div key={skill.path} className="border rounded-lg p-3">
                  <div className="font-medium">{skill.name}</div>
                  {skill.description && (
                    <div className="text-sm text-muted-foreground">{skill.description}</div>
                  )}
                  <div className="text-xs text-muted-foreground mt-1">{skill.path}</div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === 'commands' && (
          <div className="space-y-2">
            {userScope.commands.length === 0 ? (
              <p className="text-muted-foreground">No commands found</p>
            ) : (
              userScope.commands.map((cmd) => (
                <div key={cmd.path} className="border rounded-lg p-3">
                  <div className="font-medium">/{cmd.name}</div>
                  {cmd.description && (
                    <div className="text-sm text-muted-foreground">{cmd.description}</div>
                  )}
                  <div className="text-xs text-muted-foreground mt-1">{cmd.path}</div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === 'agents' && (
          <div className="space-y-2">
            {userScope.agents.length === 0 ? (
              <p className="text-muted-foreground">No agents found</p>
            ) : (
              userScope.agents.map((agent) => (
                <div key={agent.path} className="border rounded-lg p-3">
                  <div className="font-medium">{agent.name}</div>
                  {agent.description && (
                    <div className="text-sm text-muted-foreground">{agent.description}</div>
                  )}
                  <div className="text-xs text-muted-foreground mt-1">{agent.path}</div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === 'mcp' && (
          <div className="space-y-2">
            {!userScope.mcp || userScope.mcp.servers.length === 0 ? (
              <p className="text-muted-foreground">No MCP servers configured</p>
            ) : (
              userScope.mcp.servers.map((server) => (
                <div key={server.name} className="border rounded-lg p-3">
                  <div className="font-medium">{server.name}</div>
                  <div className="text-sm font-mono text-muted-foreground">
                    {server.command} {server.args.join(' ')}
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === 'collisions' && (
          <div className="space-y-4">
            {totalCollisions === 0 ? (
              <p className="text-muted-foreground">No collisions detected</p>
            ) : (
              <>
                {collisions.skills.length > 0 && (
                  <div>
                    <h4 className="font-medium mb-2">Skill Collisions</h4>
                    {collisions.skills.map((collision) => (
                      <CollisionBadge key={collision.name} collision={collision} type="skill" />
                    ))}
                  </div>
                )}
                {collisions.commands.length > 0 && (
                  <div>
                    <h4 className="font-medium mb-2">Command Collisions</h4>
                    {collisions.commands.map((collision) => (
                      <CollisionBadge key={collision.name} collision={collision} type="command" />
                    ))}
                  </div>
                )}
                {collisions.agents.length > 0 && (
                  <div>
                    <h4 className="font-medium mb-2">Agent Collisions</h4>
                    {collisions.agents.map((collision) => (
                      <CollisionBadge key={collision.name} collision={collision} type="agent" />
                    ))}
                  </div>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
