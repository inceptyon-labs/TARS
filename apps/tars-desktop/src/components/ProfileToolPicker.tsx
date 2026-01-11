import { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { open } from '@tauri-apps/plugin-dialog';
import {
  X,
  Server,
  Sparkles,
  Bot,
  Webhook,
  Search,
  Check,
  Plus,
  AlertCircle,
  RefreshCw,
  Settings,
  ChevronDown,
  ChevronRight,
  FolderOpen,
  Folder,
} from 'lucide-react';
import { discoverClaudeProjects, scanProjects } from '../lib/ipc';
import { useUIStore } from '../stores/ui-store';
import { Button } from './ui/button';
import { ToolPermissionsEditor } from './ToolPermissionsEditor';
import type { ToolRef, ToolType, ToolPermissions } from '../lib/types';

interface ProfileToolPickerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAddTools: (tools: ToolRef[]) => void;
  existingTools: ToolRef[];
}

type TabType = 'mcp' | 'skill' | 'agent';

interface ToolItem {
  name: string;
  description: string | null;
  toolType: ToolType;
  sourceProject: string; // Project name where this tool was found
  sourcePath: string; // Project path
}

function getToolIcon(type: ToolType) {
  switch (type) {
    case 'mcp':
      return Server;
    case 'skill':
      return Sparkles;
    case 'agent':
      return Bot;
    case 'hook':
      return Webhook;
    default:
      return Server;
  }
}

export function ProfileToolPicker({
  open: isOpen,
  onOpenChange,
  onAddTools,
  existingTools,
}: ProfileToolPickerProps) {
  const [activeTab, setActiveTab] = useState<TabType>('mcp');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedTools, setSelectedTools] = useState<ToolItem[]>([]);
  const [toolPermissions, setToolPermissions] = useState<Record<string, ToolPermissions | null>>(
    {}
  );
  const [expandedTool, setExpandedTool] = useState<string | null>(null);

  const developmentFolder = useUIStore((state) => state.developmentFolder);
  const setDevelopmentFolder = useUIStore((state) => state.setDevelopmentFolder);

  const getToolKey = (tool: ToolItem) => `${tool.toolType}:${tool.name}:${tool.sourcePath}`;

  // Discover Claude projects in the development folder
  const {
    data: discoveredProjects,
    isLoading: isDiscovering,
    error: discoverError,
    refetch: refetchProjects,
  } = useQuery({
    queryKey: ['discover-projects', developmentFolder],
    queryFn: async () => {
      if (!developmentFolder) return [];
      return discoverClaudeProjects(developmentFolder);
    },
    enabled: isOpen && !!developmentFolder,
  });

  // Scan discovered projects for their tools
  const {
    data: inventory,
    isLoading: isScanning,
    error: scanError,
    refetch: refetchInventory,
  } = useQuery({
    queryKey: ['project-tools', discoveredProjects?.map((p) => p.path)],
    queryFn: async () => {
      if (!discoveredProjects || discoveredProjects.length === 0) return null;
      const paths = discoveredProjects.map((p) => p.path);
      return scanProjects(paths);
    },
    enabled: isOpen && !!discoveredProjects && discoveredProjects.length > 0,
  });

  const isLoading = isDiscovering || isScanning;
  const error = discoverError || scanError;
  const refetch = () => {
    refetchProjects();
    refetchInventory();
  };

  // Extract project-scoped tools from all projects
  const availableTools = useMemo(() => {
    if (!inventory || !discoveredProjects) return { mcp: [], skill: [], agent: [] };

    const mcpServers: ToolItem[] = [];
    const skills: ToolItem[] = [];
    const agents: ToolItem[] = [];

    // Process each project's inventory
    for (const projectInv of inventory.projects || []) {
      const project = discoveredProjects.find((p) => p.path === projectInv.path);
      const projectName = project?.name || projectInv.path.split('/').pop() || 'Unknown';

      // Add project-scoped MCP servers
      for (const server of projectInv.mcp?.servers || []) {
        mcpServers.push({
          name: server.name,
          description: `Command: ${server.command}`,
          toolType: 'mcp' as ToolType,
          sourceProject: projectName,
          sourcePath: projectInv.path,
        });
      }

      // Add project-scoped skills
      for (const skill of projectInv.skills || []) {
        skills.push({
          name: skill.name,
          description: skill.description || null,
          toolType: 'skill' as ToolType,
          sourceProject: projectName,
          sourcePath: projectInv.path,
        });
      }

      // Add project-scoped agents
      for (const agent of projectInv.agents || []) {
        agents.push({
          name: agent.name,
          description: agent.description || null,
          toolType: 'agent' as ToolType,
          sourceProject: projectName,
          sourcePath: projectInv.path,
        });
      }
    }

    return { mcp: mcpServers, skill: skills, agent: agents };
  }, [inventory, discoveredProjects]);

  // Filter tools based on search query and exclude already added tools
  const filteredTools = useMemo(() => {
    const tools = availableTools[activeTab] || [];
    const existingNames = new Set(
      existingTools.filter((t) => t.tool_type === activeTab).map((t) => t.name)
    );

    return tools
      .filter((t) => !existingNames.has(t.name))
      .filter((t) =>
        searchQuery
          ? t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            t.description?.toLowerCase().includes(searchQuery.toLowerCase())
          : true
      );
  }, [availableTools, activeTab, existingTools, searchQuery]);

  const isToolSelected = (tool: ToolItem) =>
    selectedTools.some((t) => t.name === tool.name && t.toolType === tool.toolType);

  const toggleToolSelection = (tool: ToolItem) => {
    if (isToolSelected(tool)) {
      setSelectedTools(
        selectedTools.filter((t) => !(t.name === tool.name && t.toolType === tool.toolType))
      );
    } else {
      setSelectedTools([...selectedTools, tool]);
    }
  };

  const handleAddTools = () => {
    const toolRefs: ToolRef[] = selectedTools.map((t) => ({
      name: t.name,
      tool_type: t.toolType,
      source_scope: 'project',
      permissions: toolPermissions[getToolKey(t)] || null,
    }));
    onAddTools(toolRefs);
    setSelectedTools([]);
    setToolPermissions({});
    setExpandedTool(null);
    setSearchQuery('');
    onOpenChange(false);
  };

  const handleClose = () => {
    setSelectedTools([]);
    setToolPermissions({});
    setExpandedTool(null);
    setSearchQuery('');
    onOpenChange(false);
  };

  const togglePermissionsExpand = (tool: ToolItem, e: React.MouseEvent) => {
    e.stopPropagation();
    const key = getToolKey(tool);
    setExpandedTool(expandedTool === key ? null : key);
  };

  const handlePermissionsChange = (tool: ToolItem, permissions: ToolPermissions | null) => {
    const key = getToolKey(tool);
    setToolPermissions((prev) => ({
      ...prev,
      [key]: permissions,
    }));
  };

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Development Folder',
      });
      if (selected && typeof selected === 'string') {
        setDevelopmentFolder(selected);
      }
    } catch (err) {
      console.error('Failed to select folder:', err);
    }
  };

  if (!isOpen) return null;

  const tabs: { id: TabType; label: string; icon: typeof Server }[] = [
    { id: 'mcp', label: 'MCP Servers', icon: Server },
    { id: 'skill', label: 'Skills', icon: Sparkles },
    { id: 'agent', label: 'Agents', icon: Bot },
  ];

  const totalTools =
    (availableTools.mcp?.length || 0) +
    (availableTools.skill?.length || 0) +
    (availableTools.agent?.length || 0);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={handleClose} />

      {/* Dialog */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-2xl max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="text-lg font-semibold">Add Tools to Profile</h2>
          <button
            onClick={handleClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Development Folder Selector */}
        <div className="px-4 py-3 border-b bg-muted/30">
          <div className="flex items-center gap-3">
            <Folder className="h-4 w-4 text-muted-foreground shrink-0" />
            <div className="flex-1 min-w-0">
              {developmentFolder ? (
                <div className="flex items-center gap-2">
                  <span className="text-sm truncate">{developmentFolder}</span>
                  <span className="text-xs text-muted-foreground">
                    ({discoveredProjects?.length || 0} projects found)
                  </span>
                </div>
              ) : (
                <span className="text-sm text-muted-foreground">No development folder set</span>
              )}
            </div>
            <Button variant="outline" size="sm" onClick={handleSelectFolder}>
              {developmentFolder ? 'Change' : 'Select Folder'}
            </Button>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex border-b px-4">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors ${
                activeTab === tab.id
                  ? 'text-primary border-primary'
                  : 'text-muted-foreground border-transparent hover:text-foreground'
              }`}
            >
              <tab.icon className="h-4 w-4" />
              {tab.label}
              <span className="text-xs bg-muted px-1.5 py-0.5 rounded">
                {availableTools[tab.id]?.length || 0}
              </span>
            </button>
          ))}
        </div>

        {/* Search */}
        <div className="p-4 border-b">
          <div className="relative">
            <input
              type="text"
              placeholder="Search tools..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-9 pr-3 py-2 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
            />
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          </div>
        </div>

        {/* Tool List */}
        <div className="flex-1 overflow-auto p-4">
          {!developmentFolder ? (
            <div className="text-center py-12 space-y-4">
              <div className="w-16 h-16 rounded-full bg-muted flex items-center justify-center mx-auto">
                <Folder className="h-8 w-8 text-muted-foreground" />
              </div>
              <div>
                <p className="text-sm font-medium">Select a Development Folder</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Choose the folder where your projects live to discover available tools
                </p>
              </div>
              <Button onClick={handleSelectFolder}>
                <FolderOpen className="h-4 w-4 mr-2" />
                Select Folder
              </Button>
            </div>
          ) : isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
            </div>
          ) : error ? (
            <div className="text-center py-8 space-y-3">
              <AlertCircle className="h-8 w-8 text-destructive mx-auto" />
              <p className="text-sm text-destructive">Failed to load tools</p>
              <p className="text-xs text-muted-foreground">
                {error instanceof Error ? error.message : String(error)}
              </p>
              <Button variant="outline" size="sm" onClick={() => refetch()}>
                <RefreshCw className="h-4 w-4 mr-1" />
                Retry
              </Button>
            </div>
          ) : totalTools === 0 ? (
            <div className="text-center py-8 text-muted-foreground space-y-2">
              <p className="text-sm">
                No project-scoped tools found in {discoveredProjects?.length || 0} project(s)
              </p>
              <p className="text-xs">
                Projects need .claude/ directories with skills, agents, or .mcp.json files
              </p>
            </div>
          ) : filteredTools.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              {searchQuery ? 'No tools match your search' : 'No tools available in this category'}
            </div>
          ) : (
            <div className="space-y-2">
              {filteredTools.map((tool) => {
                const Icon = getToolIcon(tool.toolType);
                const selected = isToolSelected(tool);
                const toolKey = getToolKey(tool);
                const isExpanded = expandedTool === toolKey;
                const hasPermissions = toolPermissions[toolKey] != null;
                const isMcp = tool.toolType === 'mcp';

                return (
                  <div key={toolKey} className="space-y-0">
                    <button
                      onClick={() => toggleToolSelection(tool)}
                      className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                        selected
                          ? 'border-primary bg-primary/10'
                          : 'border-border hover:bg-muted/50'
                      } ${isExpanded ? 'rounded-b-none border-b-0' : ''}`}
                    >
                      <div
                        className={`w-5 h-5 rounded border flex items-center justify-center shrink-0 ${
                          selected
                            ? 'bg-primary border-primary text-primary-foreground'
                            : 'border-muted-foreground/40'
                        }`}
                      >
                        {selected && <Check className="h-3 w-3" />}
                      </div>
                      <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
                      <div className="flex-1 text-left min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-medium text-sm">{tool.name}</span>
                          <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded flex items-center gap-1">
                            <FolderOpen className="h-3 w-3" />
                            {tool.sourceProject}
                          </span>
                        </div>
                        {tool.description && (
                          <div className="text-xs text-muted-foreground truncate">
                            {tool.description}
                          </div>
                        )}
                      </div>
                      {isMcp && selected && (
                        <button
                          onClick={(e) => togglePermissionsExpand(tool, e)}
                          className={`p-1.5 rounded hover:bg-muted transition-colors shrink-0 ${
                            hasPermissions ? 'text-primary' : 'text-muted-foreground'
                          }`}
                          title="Configure permissions"
                        >
                          {isExpanded ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                          <Settings className="h-4 w-4 absolute opacity-0" />
                        </button>
                      )}
                    </button>
                    {isMcp && selected && isExpanded && (
                      <div className="border border-t-0 border-primary rounded-b-lg bg-primary/5 p-4">
                        <ToolPermissionsEditor
                          permissions={toolPermissions[toolKey] || null}
                          onChange={(perms) => handlePermissionsChange(tool, perms)}
                        />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t bg-muted/30">
          <div className="text-sm text-muted-foreground">
            {selectedTools.length} tool{selectedTools.length === 1 ? '' : 's'} selected
          </div>
          <div className="flex gap-2">
            <Button variant="outline" onClick={handleClose}>
              Cancel
            </Button>
            <Button onClick={handleAddTools} disabled={selectedTools.length === 0}>
              <Plus className="h-4 w-4 mr-2" />
              Add Tools
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
