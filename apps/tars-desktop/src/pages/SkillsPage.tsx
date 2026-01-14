import { useQuery } from '@tanstack/react-query';
import { Cpu, Plus, RefreshCw, Search, Trash2, FolderOpen } from 'lucide-react';
import { useState, useMemo } from 'react';
import { toast } from 'sonner';
import {
  scanUserScope,
  scanProjects,
  readSkill,
  saveSkill,
  createSkill,
  deleteSkill,
  listProjects,
  listProfiles,
  addToolsFromSource,
  scanProfiles,
} from '../lib/ipc';
import { SkillEditor } from '../components/SkillEditor';
import { Button } from '../components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';
import { ConfirmDialog } from '../components/config/ConfirmDialog';
import { HelpButton } from '../components/HelpButton';
import type { SkillInfo, SkillDetails, SkillScope } from '../lib/types';

/** Get scope category for grouping */
function getScopeCategory(scope: SkillScope): string {
  switch (scope.type) {
    case 'User':
      return 'user';
    case 'Project':
    case 'Local':
      return 'project';
    case 'Managed':
      return 'managed';
    case 'Plugin':
      return 'plugin';
  }
}

function isProfileToolPath(path: string): boolean {
  const normalized = path.replace(/\\/g, '/').toLowerCase();
  return (
    normalized.includes('/.tars/profiles/') ||
    normalized.includes('/.claude/plugins/marketplaces/tars-profiles/')
  );
}

function isProfileMarketplacePath(path: string): boolean {
  const normalized = path.replace(/\\/g, '/').toLowerCase();
  return (
    normalized.includes('/.claude/plugins/marketplaces/tars-profiles/') ||
    normalized.includes('/.claude/plugins/cache/tars-profiles/')
  );
}

/** Check if a skill is editable (user-created skills only) */
function isSkillEditable(scope: SkillScope | string, path?: string): boolean {
  // Handle string scope (from create_skill command)
  if (typeof scope === 'string') {
    return scope === 'user' || scope === 'project' || scope === 'profile';
  }
  // Handle object scope (from scanner)
  if (scope.type === 'User' || scope.type === 'Project' || scope.type === 'Local') {
    return true;
  }
  return scope.type === 'Plugin' && !!path && isProfileToolPath(path);
}

export function SkillsPage() {
  const [selectedSkill, setSelectedSkill] = useState<SkillDetails | null>(null);
  const [loadingSkill, setLoadingSkill] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newSkillName, setNewSkillName] = useState('');
  const [createScope, setCreateScope] = useState<'user' | 'project' | 'profile'>('user');
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [addToProfile, setAddToProfile] = useState(false);
  const [selectedProfileId, setSelectedProfileId] = useState('');
  const [creating, setCreating] = useState(false);
  const [skillToDelete, setSkillToDelete] = useState<SkillInfo | null>(null);
  const [deleting, setDeleting] = useState(false);

  // Get configured projects for project picker
  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const { data: profiles = [] } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  });

  // Scan user scope
  const {
    data: inventory,
    isLoading: isLoadingUserScope,
    refetch: refetchUserScope,
  } = useQuery({
    queryKey: ['user-scope'],
    queryFn: scanUserScope,
  });

  // Scan all configured projects
  const {
    data: projectsInventory,
    isLoading: isLoadingProjects,
    refetch: refetchProjects,
  } = useQuery({
    queryKey: ['projects-scan', projects.map((p) => p.path)],
    queryFn: () =>
      projects.length > 0 ? scanProjects(projects.map((p) => p.path)) : Promise.resolve(null),
    enabled: projects.length > 0,
  });

  const {
    data: profilesInventory,
    isLoading: isLoadingProfiles,
    refetch: refetchProfiles,
  } = useQuery({
    queryKey: ['profiles-scan'],
    queryFn: scanProfiles,
  });

  const isLoading = isLoadingUserScope || isLoadingProjects || isLoadingProfiles;

  async function refetch() {
    await Promise.all([refetchUserScope(), refetchProjects(), refetchProfiles()]);
  }

  // Combine skills from user scope and all projects
  const skills = useMemo(() => {
    const allSkills: SkillInfo[] = [];

    // Add user scope skills
    if (inventory?.user_scope.skills) {
      allSkills.push(...inventory.user_scope.skills);
    }

    // Add skills from scanned projects
    if (projectsInventory?.projects) {
      for (const project of projectsInventory.projects) {
        if (project.skills) {
          allSkills.push(...project.skills);
        }
      }
    }

    if (profilesInventory?.skills) {
      allSkills.push(...profilesInventory.skills);
    }

    return allSkills.filter((skill) => {
      if (skill.scope.type === 'Plugin' && isProfileMarketplacePath(skill.path)) {
        return false;
      }
      return true;
    });
  }, [inventory, projectsInventory, profilesInventory]);

  // Group skills by category
  const groupedSkills = useMemo(() => {
    const filtered = searchQuery
      ? skills.filter(
          (s) =>
            s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            s.description?.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : skills;

    const groups: Record<string, SkillInfo[]> = {
      user: [],
      project: [],
      plugin: [],
      managed: [],
    };

    for (const skill of filtered) {
      const category = getScopeCategory(skill.scope);
      groups[category].push(skill);
    }

    return groups;
  }, [skills, searchQuery]);

  async function handleSelectSkill(skill: SkillInfo) {
    setLoadingSkill(true);
    try {
      const details = await readSkill(skill.path);
      setSelectedSkill(details);
    } catch (err) {
      console.error('Failed to load skill:', err);
      toast.error('Failed to load skill', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setLoadingSkill(false);
    }
  }

  async function handleSaveSkill(path: string, content: string) {
    try {
      await saveSkill(path, content);
      // Reload the skill
      const details = await readSkill(path);
      setSelectedSkill(details);
      toast.success('Skill saved');
    } catch (err) {
      console.error('Failed to save skill:', err);
      toast.error('Failed to save skill', {
        description: err instanceof Error ? err.message : String(err),
      });
      throw err;
    }
  }

  async function handleCreateSkill() {
    if (!newSkillName.trim()) return;
    if (createScope === 'project' && !selectedProject) {
      toast.error('Please select a project');
      return;
    }
    if (createScope === 'profile' && !selectedProfileId) {
      toast.error('Please select a profile');
      return;
    }
    if (addToProfile && !selectedProfileId) {
      toast.error('Please select a profile');
      return;
    }

    setCreating(true);
    try {
      const toolName = newSkillName.trim();
      const projectPath = selectedProject ?? undefined;
      const shouldAddToProfile =
        addToProfile && !!selectedProfileId && (createScope === 'user' || !!selectedProject);

      const details = await createSkill(
        toolName,
        createScope,
        createScope === 'project' ? projectPath : undefined,
        createScope === 'profile' ? selectedProfileId : undefined
      );
      const scopeDesc =
        createScope === 'user'
          ? 'user scope'
          : createScope === 'project'
            ? `project "${projects.find((p) => p.path === selectedProject)?.name}"`
            : `profile "${profiles.find((p) => p.id === selectedProfileId)?.name}"`;
      toast.success(`Created skill "${toolName}"`, {
        description: `Added to ${scopeDesc}`,
      });

      if (shouldAddToProfile && createScope !== 'profile') {
        try {
          await addToolsFromSource(
            selectedProfileId,
            createScope === 'project' ? selectedProject : undefined,
            [{ name: toolName, tool_type: 'skill' }],
            createScope
          );
          const profileName =
            profiles.find((profile) => profile.id === selectedProfileId)?.name || 'profile';
          toast.success(`Added to profile "${profileName}"`);
        } catch (err) {
          toast.error('Failed to add skill to profile', {
            description: err instanceof Error ? err.message : String(err),
          });
        }
      }

      setShowCreateDialog(false);
      setNewSkillName('');
      setCreateScope('user');
      setSelectedProject(null);
      setAddToProfile(false);
      setSelectedProfileId('');
      // Refresh the list and select the new skill
      await refetch();
      setSelectedSkill(details);
    } catch (err) {
      console.error('Failed to create skill:', err);
      toast.error('Failed to create skill', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setCreating(false);
    }
  }

  async function handleDeleteSkill() {
    if (!skillToDelete) return;

    setDeleting(true);
    try {
      await deleteSkill(skillToDelete.path);
      toast.success(`Deleted skill "${skillToDelete.name}"`);

      // Clear selection if the deleted skill was selected
      if (selectedSkill?.path === skillToDelete.path) {
        setSelectedSkill(null);
      }

      // Refresh the list
      await refetch();
    } catch (err) {
      console.error('Failed to delete skill:', err);
      toast.error('Failed to delete skill', {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDeleting(false);
      setSkillToDelete(null);
    }
  }

  function renderSkillGroup(title: string, groupSkills: SkillInfo[], showActions = false) {
    if (groupSkills.length === 0) return null;

    return (
      <div key={title} className="mb-4">
        <h3 className="text-xs font-semibold text-primary uppercase tracking-wider px-3 py-2 border-b border-primary/20 mb-2">
          {title} <span className="text-primary/60">({groupSkills.length})</span>
        </h3>
        <ul className="space-y-1">
          {groupSkills.map((skill) => (
            <li key={skill.path} className="group relative">
              <button
                onClick={() => handleSelectSkill(skill)}
                className={`tars-nav-item w-full text-left px-3 py-2.5 rounded text-sm transition-all ${
                  selectedSkill?.path === skill.path
                    ? 'active text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground'
                } ${showActions ? 'pr-12' : ''}`}
              >
                <div className="flex items-center gap-2">
                  <span className="font-medium flex-1 truncate">{skill.name}</span>
                  {isProfileToolPath(skill.path) && (
                    <span className="inline-flex items-center justify-center h-7 px-2.5 text-xs bg-emerald-500/10 text-emerald-500 rounded">
                      Profile
                    </span>
                  )}
                  {skill.user_invocable && (
                    <span className="inline-flex items-center justify-center h-7 w-7 text-xs bg-primary/10 text-primary rounded">
                      /
                    </span>
                  )}
                </div>
                {skill.description && (
                  <div className="text-xs opacity-60 truncate mt-0.5">{skill.description}</div>
                )}
              </button>
              {showActions && isSkillEditable(skill.scope, skill.path) && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setSkillToDelete(skill);
                  }}
                  className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 rounded opacity-0 group-hover:opacity-100 hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-all"
                  title="Delete skill"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              )}
            </li>
          ))}
        </ul>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Skills</h2>
          <HelpButton section="SKILLS" />
        </div>
        <Button onClick={() => setShowCreateDialog(true)} size="sm">
          <Plus className="h-4 w-4 mr-2" />
          New Skill
        </Button>
      </header>

      {/* Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Skills list sidebar */}
        <div className="w-72 border-r border-border flex flex-col tars-panel">
          <div className="p-3 border-b border-border">
            <div className="relative flex items-center">
              <input
                type="search"
                placeholder="Search skills..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="tars-input w-full pl-9 pr-3 py-2 text-sm rounded"
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="off"
                spellCheck={false}
                data-form-type="other"
              />
              <Search className="absolute left-3 h-4 w-4 text-muted-foreground pointer-events-none" />
            </div>
          </div>

          <div className="tars-segment-line" />

          <div className="flex-1 overflow-auto p-3">
            {isLoading ? (
              <div className="flex flex-col items-center justify-center py-12 gap-3">
                <RefreshCw className="h-5 w-5 animate-spin text-primary" />
                <span className="text-xs text-muted-foreground">Loading...</span>
              </div>
            ) : skills.length === 0 ? (
              <div className="text-center py-12 px-4">
                <div className="w-16 h-16 rounded-lg tars-panel flex items-center justify-center mx-auto mb-4">
                  <Cpu className="h-8 w-8 text-muted-foreground" />
                </div>
                <p className="text-sm font-medium text-foreground">No skills found</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Skills live in ~/.claude/skills/
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4"
                  onClick={() => setShowCreateDialog(true)}
                >
                  <Plus className="h-4 w-4 mr-2" />
                  Create your first skill
                </Button>
              </div>
            ) : (
              <>
                {renderSkillGroup('User Skills', groupedSkills.user, true)}
                {renderSkillGroup('Project Skills', groupedSkills.project, true)}
                {renderSkillGroup('Plugin Skills', groupedSkills.plugin, true)}
                {renderSkillGroup('Managed Skills', groupedSkills.managed, false)}
              </>
            )}
          </div>
        </div>

        {/* Skill editor */}
        <div className="flex-1 overflow-hidden bg-background">
          {loadingSkill ? (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="relative">
                <RefreshCw className="h-8 w-8 animate-spin text-primary" />
                <div className="absolute inset-0 blur-lg bg-primary/30 rounded-full" />
              </div>
              <p className="text-sm text-muted-foreground">Loading skill...</p>
            </div>
          ) : selectedSkill ? (
            <SkillEditor
              skill={selectedSkill}
              onSave={handleSaveSkill}
              readOnly={!isSkillEditable(selectedSkill.scope, selectedSkill.path)}
            />
          ) : (
            <div className="flex flex-col items-center justify-center h-full gap-4">
              <div className="w-20 h-20 rounded-lg tars-panel flex items-center justify-center">
                <Cpu className="h-10 w-10 text-muted-foreground/50" />
              </div>
              <div className="text-center">
                <p className="text-sm text-muted-foreground">Select a skill to edit</p>
                <p className="text-xs text-muted-foreground/60 mt-1">
                  Customize Claude Code capabilities
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Create Skill Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New Skill</DialogTitle>
            <DialogDescription>
              Create a new skill in your user, project, or profile scope.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div>
              <Label htmlFor="skill-name">Skill Name</Label>
              <Input
                id="skill-name"
                value={newSkillName}
                onChange={(e) => setNewSkillName(e.target.value)}
                placeholder="my-skill"
                className="mt-2"
                onKeyDown={(e) => {
                  if (
                    e.key === 'Enter' &&
                    newSkillName.trim() &&
                    (createScope === 'user' ||
                      (createScope === 'project' && selectedProject) ||
                      (createScope === 'profile' && selectedProfileId))
                  ) {
                    handleCreateSkill();
                  }
                }}
              />
              <p className="text-xs text-muted-foreground mt-2">
                Use lowercase letters, numbers, and hyphens.
              </p>
            </div>

            <div>
              <Label>Scope</Label>
              <div className="flex gap-4 mt-2">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="skill-scope"
                    checked={createScope === 'user'}
                    onChange={() => {
                      setCreateScope('user');
                      setSelectedProject(null);
                    }}
                    className="accent-primary"
                  />
                  <span className="text-sm">User (~/.claude/skills/)</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="skill-scope"
                    checked={createScope === 'project'}
                    onChange={() => {
                      setCreateScope('project');
                      setAddToProfile(false);
                    }}
                    className="accent-primary"
                  />
                  <span className="text-sm">Project</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="skill-scope"
                    checked={createScope === 'profile'}
                    onChange={() => {
                      setCreateScope('profile');
                      setSelectedProject(null);
                      setAddToProfile(false);
                    }}
                    className="accent-primary"
                  />
                  <span className="text-sm">Profile</span>
                </label>
              </div>
            </div>

            {createScope === 'project' && (
              <div>
                <Label>Project</Label>
                {projects.length === 0 ? (
                  <p className="text-sm text-muted-foreground mt-2">
                    No projects configured. Add a project first.
                  </p>
                ) : (
                  <div className="mt-2 space-y-1">
                    {projects.map((project) => (
                      <label
                        key={project.path}
                        className={`flex items-center gap-2 p-2 rounded cursor-pointer transition-colors ${
                          selectedProject === project.path
                            ? 'bg-primary/10 border border-primary/30'
                            : 'hover:bg-muted border border-transparent'
                        }`}
                      >
                        <input
                          type="radio"
                          name="skill-project"
                          checked={selectedProject === project.path}
                          onChange={() => setSelectedProject(project.path)}
                          className="accent-primary"
                        />
                        <FolderOpen className="h-4 w-4 text-muted-foreground" />
                        <div className="flex-1 min-w-0">
                          <div className="text-sm font-medium truncate">{project.name}</div>
                          <div className="text-xs text-muted-foreground truncate">
                            {project.path}
                          </div>
                        </div>
                      </label>
                    ))}
                  </div>
                )}
              </div>
            )}

            {createScope === 'profile' && (
              <div>
                <Label>Profile</Label>
                {profiles.length === 0 ? (
                  <p className="text-sm text-muted-foreground mt-2">
                    No profiles configured. Create a profile first.
                  </p>
                ) : (
                  <div className="mt-2 space-y-1">
                    {profiles.map((profile) => (
                      <label
                        key={profile.id}
                        className={`flex items-center gap-2 p-2 rounded cursor-pointer transition-colors ${
                          selectedProfileId === profile.id
                            ? 'bg-primary/10 border border-primary/30'
                            : 'hover:bg-muted border border-transparent'
                        }`}
                      >
                        <input
                          type="radio"
                          name="skill-profile"
                          checked={selectedProfileId === profile.id}
                          onChange={() => setSelectedProfileId(profile.id)}
                          className="accent-primary"
                        />
                        <div className="flex-1 min-w-0">
                          <div className="text-sm font-medium truncate">{profile.name}</div>
                          {profile.description && (
                            <div className="text-xs text-muted-foreground truncate">
                              {profile.description}
                            </div>
                          )}
                        </div>
                      </label>
                    ))}
                  </div>
                )}
              </div>
            )}

            {(createScope === 'project' || createScope === 'user') && (
              <div className="rounded-md border border-border p-3 space-y-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <Label htmlFor="add-skill-to-profile">Add to profile</Label>
                    <p className="text-xs text-muted-foreground mt-1">
                      Copies this skill into a profile for reuse.
                    </p>
                  </div>
                  <input
                    id="add-skill-to-profile"
                    type="checkbox"
                    checked={addToProfile}
                    onChange={(e) => {
                      const next = e.target.checked;
                      setAddToProfile(next);
                      if (!next) {
                        setSelectedProfileId('');
                      }
                    }}
                    className="accent-primary"
                    disabled={
                      profiles.length === 0 || (createScope === 'project' && !selectedProject)
                    }
                  />
                </div>

                {profiles.length === 0 && (
                  <p className="text-xs text-muted-foreground">
                    Create a profile first to enable this option.
                  </p>
                )}
                {createScope === 'project' && !selectedProject && (
                  <p className="text-xs text-muted-foreground">
                    Select a project to enable this option.
                  </p>
                )}

                {addToProfile && profiles.length > 0 && (
                  <div className="space-y-2">
                    <Label>Profile</Label>
                    <Select
                      value={selectedProfileId || undefined}
                      onValueChange={(value) => setSelectedProfileId(value)}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="Select profile" />
                      </SelectTrigger>
                      <SelectContent>
                        {profiles.map((profile) => (
                          <SelectItem key={profile.id} value={profile.id}>
                            {profile.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                )}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowCreateDialog(false)}
              disabled={creating}
            >
              Cancel
            </Button>
            <Button
              onClick={handleCreateSkill}
              disabled={
                creating ||
                !newSkillName.trim() ||
                (createScope === 'project' && !selectedProject) ||
                (createScope === 'profile' && !selectedProfileId) ||
                (addToProfile && !selectedProfileId)
              }
            >
              {creating ? 'Creating...' : 'Create Skill'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={!!skillToDelete}
        onOpenChange={(open) => !open && setSkillToDelete(null)}
        title="Delete Skill"
        description={`Are you sure you want to delete "${skillToDelete?.name}"? This action cannot be undone.`}
        confirmLabel="Delete"
        confirmVariant="destructive"
        onConfirm={handleDeleteSkill}
        loading={deleting}
      />
    </div>
  );
}
