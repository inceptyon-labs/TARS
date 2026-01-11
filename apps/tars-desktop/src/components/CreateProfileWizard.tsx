import { useState, useMemo, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { open } from '@tauri-apps/plugin-dialog';
import {
  X,
  FolderOpen,
  Folder,
  Server,
  Sparkles,
  Bot,
  Webhook,
  Check,
  ChevronLeft,
  ChevronRight,
  Search,
  AlertCircle,
  RefreshCw,
  FileBox,
  Layers,
  HardDrive,
  Plus,
} from 'lucide-react';
import { listProjects, scanProjects, scanProject, discoverClaudeProjects } from '../lib/ipc';
import { useUIStore } from '../stores/ui-store';
import { Button } from './ui/button';
import type { ToolRef, ToolType } from '../lib/types';

interface CreateProfileWizardProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreate: (name: string, description: string | undefined, tools: ToolRef[]) => void;
  isLoading: boolean;
  error?: string;
}

type SourceType = 'single-project' | 'registered-projects' | 'dev-folder' | 'empty';

type Step = 'basics' | 'source' | 'tools';

interface ToolItem {
  name: string;
  description: string | null;
  toolType: ToolType;
  sourceProject: string;
  sourcePath: string;
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

export function CreateProfileWizard({
  open: isOpen,
  onOpenChange,
  onCreate,
  isLoading,
  error,
}: CreateProfileWizardProps) {
  // Step state
  const [step, setStep] = useState<Step>('basics');

  // Basic info
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');

  // Source selection
  const [sourceType, setSourceType] = useState<SourceType | null>(null);
  const [singleProjectPath, setSingleProjectPath] = useState<string | null>(null);
  const [devFolder, setDevFolder] = useState<string | null>(null);

  // Tool selection
  const [selectedTools, setSelectedTools] = useState<ToolItem[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeTab, setActiveTab] = useState<'mcp' | 'skill' | 'agent'>('mcp');

  // Store for dev folder persistence
  const storedDevFolder = useUIStore((state) => state.developmentFolder);
  const setStoredDevFolder = useUIStore((state) => state.setDevelopmentFolder);

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen) {
      setStep('basics');
      setName('');
      setDescription('');
      setSourceType(null);
      setSingleProjectPath(null);
      setDevFolder(null);
      setSelectedTools([]);
      setSearchQuery('');
      setActiveTab('mcp');
    }
  }, [isOpen]);

  // Get registered projects - always fetch when open to show count in UI
  const { data: registeredProjects } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
    enabled: isOpen,
    staleTime: 0, // Always refetch when dialog opens
  });

  // Effective dev folder (use stored if not explicitly set)
  const effectiveDevFolder = devFolder || storedDevFolder;

  // Discover projects in dev folder
  const { data: discoveredProjects, isLoading: isDiscovering } = useQuery({
    queryKey: ['discover-projects', effectiveDevFolder],
    queryFn: async () => {
      if (!effectiveDevFolder) return [];
      return discoverClaudeProjects(effectiveDevFolder);
    },
    enabled: isOpen && sourceType === 'dev-folder' && !!effectiveDevFolder,
    staleTime: 0, // Always refetch
  });

  // Determine which paths to scan based on source type
  const pathsToScan = useMemo(() => {
    if (sourceType === 'single-project' && singleProjectPath) {
      return [singleProjectPath];
    }
    if (sourceType === 'registered-projects' && registeredProjects) {
      return registeredProjects.map((p) => p.path);
    }
    if (sourceType === 'dev-folder' && discoveredProjects) {
      return discoveredProjects.map((p) => p.path);
    }
    return [];
  }, [sourceType, singleProjectPath, registeredProjects, discoveredProjects]);

  // Scan the selected projects for tools
  const {
    data: inventory,
    isLoading: isScanning,
    error: scanError,
    refetch,
  } = useQuery({
    queryKey: ['scan-tools', pathsToScan],
    queryFn: async () => {
      if (pathsToScan.length === 0) return null;
      if (pathsToScan.length === 1) {
        return scanProject(pathsToScan[0]);
      }
      return scanProjects(pathsToScan);
    },
    staleTime: 0, // Always refetch
    enabled: isOpen && step === 'tools' && pathsToScan.length > 0,
  });

  // Extract tools from inventory
  const availableTools = useMemo(() => {
    if (!inventory) return { mcp: [], skill: [], agent: [] };

    const mcpServers: ToolItem[] = [];
    const skills: ToolItem[] = [];
    const agents: ToolItem[] = [];

    for (const projectInv of inventory.projects || []) {
      const projectName = projectInv.name || projectInv.path.split('/').pop() || 'Unknown';

      for (const server of projectInv.mcp?.servers || []) {
        mcpServers.push({
          name: server.name,
          description: `Command: ${server.command}`,
          toolType: 'mcp',
          sourceProject: projectName,
          sourcePath: projectInv.path,
        });
      }

      for (const skill of projectInv.skills || []) {
        skills.push({
          name: skill.name,
          description: skill.description || null,
          toolType: 'skill',
          sourceProject: projectName,
          sourcePath: projectInv.path,
        });
      }

      for (const agent of projectInv.agents || []) {
        agents.push({
          name: agent.name,
          description: agent.description || null,
          toolType: 'agent',
          sourceProject: projectName,
          sourcePath: projectInv.path,
        });
      }
    }

    return { mcp: mcpServers, skill: skills, agent: agents };
  }, [inventory]);

  // Filter tools by search
  const filteredTools = useMemo(() => {
    const tools = availableTools[activeTab] || [];
    return tools.filter((t) =>
      searchQuery
        ? t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          t.description?.toLowerCase().includes(searchQuery.toLowerCase())
        : true
    );
  }, [availableTools, activeTab, searchQuery]);

  const totalTools =
    (availableTools.mcp?.length || 0) +
    (availableTools.skill?.length || 0) +
    (availableTools.agent?.length || 0);

  const isScanLoading = isDiscovering || isScanning;

  // Handlers
  const handleClose = () => {
    setStep('basics');
    setName('');
    setDescription('');
    setSourceType(null);
    setSingleProjectPath(null);
    setDevFolder(null);
    setSelectedTools([]);
    setSearchQuery('');
    setActiveTab('mcp');
    onOpenChange(false);
  };

  const handleSelectSingleProject = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Project',
      });
      if (selected && typeof selected === 'string') {
        setSingleProjectPath(selected);
      }
    } catch (err) {
      console.error('Failed to select project:', err);
    }
  };

  const handleSelectDevFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Development Folder',
      });
      if (selected && typeof selected === 'string') {
        setDevFolder(selected);
        setStoredDevFolder(selected); // Persist for future use
      }
    } catch (err) {
      console.error('Failed to select folder:', err);
    }
  };

  const isToolSelected = (tool: ToolItem) =>
    selectedTools.some(
      (t) =>
        t.name === tool.name && t.toolType === tool.toolType && t.sourcePath === tool.sourcePath
    );

  const toggleToolSelection = (tool: ToolItem) => {
    if (isToolSelected(tool)) {
      setSelectedTools(
        selectedTools.filter(
          (t) =>
            !(
              t.name === tool.name &&
              t.toolType === tool.toolType &&
              t.sourcePath === tool.sourcePath
            )
        )
      );
    } else {
      setSelectedTools([...selectedTools, tool]);
    }
  };

  const selectAllTools = () => {
    const allTools = [...availableTools.mcp, ...availableTools.skill, ...availableTools.agent];
    setSelectedTools(allTools);
  };

  const selectNoneTools = () => {
    setSelectedTools([]);
  };

  const handleNext = () => {
    if (step === 'basics') {
      setStep('source');
    } else if (step === 'source') {
      if (sourceType === 'empty') {
        handleCreate();
      } else {
        setStep('tools');
      }
    }
  };

  const handleBack = () => {
    if (step === 'source') {
      setStep('basics');
    } else if (step === 'tools') {
      setStep('source');
    }
  };

  const handleCreate = () => {
    const toolRefs: ToolRef[] = selectedTools.map((t) => ({
      name: t.name,
      tool_type: t.toolType,
      source_scope: 'project',
      permissions: null,
    }));
    onCreate(name.trim(), description.trim() || undefined, toolRefs);
  };

  const canProceedFromBasics = name.trim().length > 0;
  const canProceedFromSource =
    sourceType !== null &&
    (sourceType === 'empty' ||
      (sourceType === 'single-project' && singleProjectPath) ||
      (sourceType === 'registered-projects' &&
        registeredProjects &&
        registeredProjects.length > 0) ||
      (sourceType === 'dev-folder' && effectiveDevFolder));

  if (!isOpen) return null;

  const sourceOptions: {
    id: SourceType;
    label: string;
    description: string;
    icon: typeof FileBox;
  }[] = [
    {
      id: 'single-project',
      label: 'From Single Project',
      description: 'Import tools from one specific project',
      icon: FileBox,
    },
    {
      id: 'registered-projects',
      label: 'From Registered Projects',
      description: `Pick tools from projects added to TARS (${registeredProjects?.length || 0} projects)`,
      icon: Layers,
    },
    {
      id: 'dev-folder',
      label: 'Browse Development Folder',
      description: 'Scan a folder for all projects with Claude config',
      icon: HardDrive,
    },
    {
      id: 'empty',
      label: 'Start Empty',
      description: 'Create profile now, add tools later',
      icon: Plus,
    },
  ];

  const tabs: { id: 'mcp' | 'skill' | 'agent'; label: string; icon: typeof Server }[] = [
    { id: 'mcp', label: 'MCP Servers', icon: Server },
    { id: 'skill', label: 'Skills', icon: Sparkles },
    { id: 'agent', label: 'Agents', icon: Bot },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={handleClose} />

      {/* Dialog */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-2xl max-h-[85vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b shrink-0">
          <div>
            <h2 className="text-lg font-semibold">Create Profile</h2>
            <div className="flex items-center gap-2 mt-1">
              {['basics', 'source', 'tools'].map((s, i) => (
                <div key={s} className="flex items-center gap-2">
                  <div
                    className={`w-2 h-2 rounded-full ${
                      step === s
                        ? 'bg-primary'
                        : s === 'tools' && sourceType === 'empty'
                          ? 'bg-muted'
                          : 'bg-muted-foreground/30'
                    }`}
                  />
                  {i < 2 && <div className="w-8 h-px bg-border" />}
                </div>
              ))}
            </div>
          </div>
          <button
            onClick={handleClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-4">
          {/* Step 1: Basics */}
          {step === 'basics' && (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-1.5">Profile Name</label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g., rust-development, ios-mobile"
                  autoFocus
                  className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1.5">Description (optional)</label>
                <textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="What is this profile for?"
                  rows={3}
                  className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background resize-none focus:outline-none focus:ring-1 focus:ring-ring"
                />
              </div>
            </div>
          )}

          {/* Step 2: Source Selection */}
          {step === 'source' && (
            <div className="space-y-3">
              <p className="text-sm text-muted-foreground mb-4">
                Choose where to discover tools for this profile
              </p>

              {sourceOptions.map((option) => {
                const Icon = option.icon;
                const isSelected = sourceType === option.id;
                const isRegisteredDisabled =
                  option.id === 'registered-projects' &&
                  (!registeredProjects || registeredProjects.length === 0);

                return (
                  <button
                    key={option.id}
                    onClick={() => !isRegisteredDisabled && setSourceType(option.id)}
                    disabled={isRegisteredDisabled}
                    className={`w-full flex items-start gap-3 p-4 rounded-lg border transition-colors text-left ${
                      isSelected
                        ? 'border-primary bg-primary/10'
                        : isRegisteredDisabled
                          ? 'border-border opacity-50 cursor-not-allowed'
                          : 'border-border hover:bg-muted/50'
                    }`}
                  >
                    <div
                      className={`w-5 h-5 rounded-full border-2 flex items-center justify-center shrink-0 mt-0.5 ${
                        isSelected ? 'border-primary bg-primary' : 'border-muted-foreground/40'
                      }`}
                    >
                      {isSelected && <div className="w-2 h-2 rounded-full bg-primary-foreground" />}
                    </div>
                    <Icon
                      className={`h-5 w-5 shrink-0 mt-0.5 ${isSelected ? 'text-primary' : 'text-muted-foreground'}`}
                    />
                    <div className="flex-1 min-w-0">
                      <div className="font-medium text-sm">{option.label}</div>
                      <div className="text-xs text-muted-foreground mt-0.5">
                        {option.description}
                      </div>

                      {/* Additional UI for specific source types */}
                      {isSelected && option.id === 'single-project' && (
                        <div className="mt-3 flex items-center gap-2">
                          <input
                            type="text"
                            value={singleProjectPath || ''}
                            readOnly
                            placeholder="No project selected"
                            className="flex-1 px-3 py-1.5 text-xs border border-border rounded bg-background"
                          />
                          <Button size="sm" variant="outline" onClick={handleSelectSingleProject}>
                            <FolderOpen className="h-3 w-3 mr-1" />
                            Browse
                          </Button>
                        </div>
                      )}

                      {isSelected && option.id === 'dev-folder' && (
                        <div className="mt-3 flex items-center gap-2">
                          <input
                            type="text"
                            value={effectiveDevFolder || ''}
                            readOnly
                            placeholder="No folder selected"
                            className="flex-1 px-3 py-1.5 text-xs border border-border rounded bg-background"
                          />
                          <Button size="sm" variant="outline" onClick={handleSelectDevFolder}>
                            <FolderOpen className="h-3 w-3 mr-1" />
                            {effectiveDevFolder ? 'Change' : 'Browse'}
                          </Button>
                        </div>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          )}

          {/* Step 3: Tool Selection */}
          {step === 'tools' && (
            <div className="space-y-4">
              {/* Source info */}
              <div className="flex items-center justify-between text-sm">
                <div className="flex items-center gap-2 text-muted-foreground">
                  <Folder className="h-4 w-4" />
                  <span>
                    {sourceType === 'single-project' && singleProjectPath?.split('/').pop()}
                    {sourceType === 'registered-projects' &&
                      `${registeredProjects?.length || 0} registered projects`}
                    {sourceType === 'dev-folder' &&
                      `${discoveredProjects?.length || 0} projects in ${effectiveDevFolder?.split('/').pop()}`}
                  </span>
                </div>
                {totalTools > 0 && (
                  <div className="flex items-center gap-2">
                    <button
                      onClick={selectAllTools}
                      className="text-xs text-primary hover:underline"
                    >
                      Select all
                    </button>
                    <span className="text-muted-foreground">|</span>
                    <button
                      onClick={selectNoneTools}
                      className="text-xs text-primary hover:underline"
                    >
                      Select none
                    </button>
                  </div>
                )}
              </div>

              {/* Tabs */}
              <div className="flex border-b">
                {tabs.map((tab) => (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id)}
                    className={`flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
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

              {/* Tool list */}
              <div className="max-h-64 overflow-auto space-y-2">
                {isScanLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
                  </div>
                ) : scanError ? (
                  <div className="text-center py-8 space-y-3">
                    <AlertCircle className="h-8 w-8 text-destructive mx-auto" />
                    <p className="text-sm text-destructive">Failed to load tools</p>
                    <Button variant="outline" size="sm" onClick={() => refetch()}>
                      <RefreshCw className="h-4 w-4 mr-1" />
                      Retry
                    </Button>
                  </div>
                ) : totalTools === 0 ? (
                  <div className="text-center py-8 text-muted-foreground space-y-2">
                    <p className="text-sm">No project-scoped tools found</p>
                    <p className="text-xs">
                      Projects need .claude/ directories with skills, agents, or .mcp.json files
                    </p>
                  </div>
                ) : filteredTools.length === 0 ? (
                  <div className="text-center py-8 text-muted-foreground">
                    {searchQuery ? 'No tools match your search' : 'No tools in this category'}
                  </div>
                ) : (
                  filteredTools.map((tool) => {
                    const Icon = getToolIcon(tool.toolType);
                    const selected = isToolSelected(tool);
                    const toolKey = `${tool.toolType}:${tool.name}:${tool.sourcePath}`;

                    return (
                      <button
                        key={toolKey}
                        onClick={() => toggleToolSelection(tool)}
                        className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                          selected
                            ? 'border-primary bg-primary/10'
                            : 'border-border hover:bg-muted/50'
                        }`}
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
                      </button>
                    );
                  })
                )}
              </div>
            </div>
          )}

          {error && <p className="text-sm text-destructive mt-4">{error}</p>}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t bg-muted/30 shrink-0">
          <div>
            {step === 'tools' && (
              <span className="text-sm text-muted-foreground">
                {selectedTools.length} tool{selectedTools.length === 1 ? '' : 's'} selected
              </span>
            )}
          </div>
          <div className="flex gap-2">
            {step !== 'basics' && (
              <Button variant="outline" onClick={handleBack}>
                <ChevronLeft className="h-4 w-4 mr-1" />
                Back
              </Button>
            )}
            {step === 'basics' && (
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
            )}
            {step === 'basics' && (
              <Button onClick={handleNext} disabled={!canProceedFromBasics}>
                Next
                <ChevronRight className="h-4 w-4 ml-1" />
              </Button>
            )}
            {step === 'source' && (
              <Button onClick={handleNext} disabled={!canProceedFromSource}>
                {sourceType === 'empty' ? 'Create Profile' : 'Next'}
                {sourceType !== 'empty' && <ChevronRight className="h-4 w-4 ml-1" />}
              </Button>
            )}
            {step === 'tools' && (
              <Button onClick={handleCreate} disabled={isLoading}>
                {isLoading ? 'Creating...' : 'Create Profile'}
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
