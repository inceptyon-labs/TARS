import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Webhook,
  Plus,
  RefreshCw,
  Trash2,
  ChevronDown,
  ChevronRight,
  Play,
  MessageSquare,
} from 'lucide-react';
import { useState, useCallback } from 'react';
import { toast } from 'sonner';
import {
  getUserHooks,
  saveUserHooks,
  getProjectHooks,
  saveProjectHooks,
  getProfileHooks,
  saveProfileHooks,
  getHookEventTypes,
  listProjects,
  listProfiles,
} from '../lib/ipc';
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
import { ConfirmDialog } from '../components/config/ConfirmDialog';
import { HelpButton } from '../components/HelpButton';
import type { SettingsHookEvent, SettingsHookAction } from '../lib/types';

// Hook event descriptions
const EVENT_DESCRIPTIONS: Record<string, string> = {
  PreToolUse: 'Runs before a tool executes (e.g., Write, Edit, Bash)',
  PostToolUse: 'Runs after a tool has finished executing',
  Stop: 'Runs when Claude wants to stop working',
  SubagentStop: 'Runs when a subagent is about to stop',
  SessionStart: 'Runs at the start of a session',
  SessionEnd: 'Runs at the end of a session',
  UserPromptSubmit: 'Runs when the user submits a prompt',
  PreCompact: 'Runs before the conversation is compacted',
  Notification: 'Runs when a notification is triggered',
};

export function HooksPage() {
  const queryClient = useQueryClient();
  const [selectedScope, setSelectedScope] = useState<'user' | 'project' | 'profile'>('user');
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set());
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [showAddMatcherDialog, setShowAddMatcherDialog] = useState(false);
  const [showAddHookDialog, setShowAddHookDialog] = useState(false);
  const [selectedEvent, setSelectedEvent] = useState<string | null>(null);
  const [selectedMatcherIndex, setSelectedMatcherIndex] = useState<number | null>(null);
  const [hookToDelete, setHookToDelete] = useState<{
    event: string;
    matcherIndex: number;
    hookIndex?: number;
  } | null>(null);
  const [deleting, setDeleting] = useState(false);

  // New hook form state
  const [newEventType, setNewEventType] = useState('');
  const [newMatcher, setNewMatcher] = useState('*');
  const [newHookType, setNewHookType] = useState<'command' | 'prompt'>('command');
  const [newCommand, setNewCommand] = useState('');
  const [newPrompt, setNewPrompt] = useState('');
  const [newTimeout, setNewTimeout] = useState('30');

  // Get configured projects
  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: listProjects,
  });

  const { data: profiles = [] } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  });

  // Get hook event types
  const { data: eventTypes = [] } = useQuery({
    queryKey: ['hook-event-types'],
    queryFn: getHookEventTypes,
  });

  // Get hooks for selected scope
  const { data: hooksConfig, isLoading } = useQuery({
    queryKey: ['hooks', selectedScope, selectedProject, selectedProfileId],
    queryFn: () => {
      if (selectedScope === 'user') {
        return getUserHooks();
      } else if (selectedScope === 'project' && selectedProject) {
        return getProjectHooks(selectedProject);
      } else if (selectedScope === 'profile' && selectedProfileId) {
        return getProfileHooks(selectedProfileId);
      }
      return Promise.resolve({ path: '', scope: 'user', events: [] });
    },
    enabled:
      selectedScope === 'user' ||
      (selectedScope === 'project' && !!selectedProject) ||
      (selectedScope === 'profile' && !!selectedProfileId),
  });

  const events = hooksConfig?.events || [];
  const canEditScope =
    selectedScope === 'user' ||
    (selectedScope === 'project' && !!selectedProject) ||
    (selectedScope === 'profile' && !!selectedProfileId);

  // Save hooks mutation
  const saveMutation = useMutation({
    mutationFn: async (newEvents: SettingsHookEvent[]) => {
      if (selectedScope === 'user') {
        await saveUserHooks(newEvents);
      } else if (selectedScope === 'project' && selectedProject) {
        await saveProjectHooks(selectedProject, newEvents);
      } else if (selectedScope === 'profile' && selectedProfileId) {
        await saveProfileHooks(selectedProfileId, newEvents);
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['hooks'] });
      queryClient.invalidateQueries({ queryKey: ['profile-hooks'] });
      toast.success('Hooks saved');
    },
    onError: (err) => {
      toast.error('Failed to save hooks', {
        description: err instanceof Error ? err.message : String(err),
      });
    },
  });

  const toggleEvent = useCallback((event: string) => {
    setExpandedEvents((prev) => {
      const next = new Set(prev);
      if (next.has(event)) {
        next.delete(event);
      } else {
        next.add(event);
      }
      return next;
    });
  }, []);

  function handleAddEvent() {
    if (!newEventType) return;

    const existingEvent = events.find((e) => e.event === newEventType);
    if (existingEvent) {
      toast.error('Event already exists');
      return;
    }

    const newEvents: SettingsHookEvent[] = [
      ...events,
      {
        event: newEventType,
        matchers: [
          {
            matcher: '*',
            hooks: [],
          },
        ],
      },
    ];

    saveMutation.mutate(newEvents);
    setShowAddDialog(false);
    setNewEventType('');
    setExpandedEvents((prev) => new Set([...prev, newEventType]));
  }

  function handleAddMatcher() {
    if (!selectedEvent || !newMatcher) return;

    const newEvents = events.map((e) => {
      if (e.event === selectedEvent) {
        return {
          ...e,
          matchers: [...e.matchers, { matcher: newMatcher, hooks: [] }],
        };
      }
      return e;
    });

    saveMutation.mutate(newEvents);
    setShowAddMatcherDialog(false);
    setNewMatcher('*');
    setSelectedEvent(null);
  }

  function handleAddHook() {
    if (!selectedEvent || selectedMatcherIndex === null) return;

    const newHook: SettingsHookAction = {
      type: newHookType,
      ...(newHookType === 'command' ? { command: newCommand } : { prompt: newPrompt }),
      ...(newTimeout ? { timeout: parseInt(newTimeout, 10) } : {}),
    };

    const newEvents = events.map((e) => {
      if (e.event === selectedEvent) {
        return {
          ...e,
          matchers: e.matchers.map((m, i) => {
            if (i === selectedMatcherIndex) {
              return { ...m, hooks: [...m.hooks, newHook] };
            }
            return m;
          }),
        };
      }
      return e;
    });

    saveMutation.mutate(newEvents);
    setShowAddHookDialog(false);
    setNewHookType('command');
    setNewCommand('');
    setNewPrompt('');
    setNewTimeout('30');
    setSelectedEvent(null);
    setSelectedMatcherIndex(null);
  }

  async function handleDelete() {
    if (!hookToDelete) return;

    setDeleting(true);
    try {
      let newEvents: SettingsHookEvent[];

      if (hookToDelete.hookIndex !== undefined) {
        // Delete a specific hook
        newEvents = events.map((e) => {
          if (e.event === hookToDelete.event) {
            return {
              ...e,
              matchers: e.matchers.map((m, i) => {
                if (i === hookToDelete.matcherIndex) {
                  return {
                    ...m,
                    hooks: m.hooks.filter((_, hi) => hi !== hookToDelete.hookIndex),
                  };
                }
                return m;
              }),
            };
          }
          return e;
        });
      } else {
        // Delete a matcher
        newEvents = events
          .map((e) => {
            if (e.event === hookToDelete.event) {
              const newMatchers = e.matchers.filter((_, i) => i !== hookToDelete.matcherIndex);
              if (newMatchers.length === 0) {
                return null; // Will filter out
              }
              return { ...e, matchers: newMatchers };
            }
            return e;
          })
          .filter((e): e is SettingsHookEvent => e !== null);
      }

      await saveMutation.mutateAsync(newEvents);
    } finally {
      setDeleting(false);
      setHookToDelete(null);
    }
  }

  function openAddMatcherDialog(event: string) {
    setSelectedEvent(event);
    setShowAddMatcherDialog(true);
  }

  function openAddHookDialog(event: string, matcherIndex: number) {
    setSelectedEvent(event);
    setSelectedMatcherIndex(matcherIndex);
    setShowAddHookDialog(true);
  }

  // Get events that haven't been added yet
  const availableEvents = eventTypes.filter((et) => !events.find((e) => e.event === et));

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">Hooks</h2>
          <HelpButton section="HOOKS" />
        </div>
      </header>

      {/* Controls bar */}
      <div className="p-4 border-b border-border bg-muted/30">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Label className="text-sm font-medium">Scope:</Label>
            <div className="flex gap-2">
              <Button
                variant={selectedScope === 'user' ? 'default' : 'outline'}
                size="sm"
                onClick={() => {
                  setSelectedScope('user');
                  setSelectedProject(null);
                  setSelectedProfileId(null);
                }}
              >
                User (~/.claude/)
              </Button>
              <Button
                variant={selectedScope === 'project' ? 'default' : 'outline'}
                size="sm"
                onClick={() => {
                  setSelectedScope('project');
                  setSelectedProfileId(null);
                }}
              >
                Project
              </Button>
              <Button
                variant={selectedScope === 'profile' ? 'default' : 'outline'}
                size="sm"
                onClick={() => {
                  setSelectedScope('profile');
                  setSelectedProject(null);
                }}
                disabled={profiles.length === 0}
              >
                Profile
              </Button>
            </div>
            {selectedScope === 'project' && (
              <select
                value={selectedProject || ''}
                onChange={(e) => setSelectedProject(e.target.value || null)}
                className="tars-input px-3 py-1.5 text-sm rounded"
              >
                <option value="">Select project...</option>
                {projects.map((p) => (
                  <option key={p.path} value={p.path}>
                    {p.name}
                  </option>
                ))}
              </select>
            )}
            {selectedScope === 'profile' && (
              <select
                value={selectedProfileId || ''}
                onChange={(e) => setSelectedProfileId(e.target.value || null)}
                className="tars-input px-3 py-1.5 text-sm rounded"
              >
                <option value="">Select profile...</option>
                {profiles.map((profile) => (
                  <option key={profile.id} value={profile.id}>
                    {profile.name}
                  </option>
                ))}
              </select>
            )}
          </div>
          <Button
            onClick={() => setShowAddDialog(true)}
            size="sm"
            disabled={availableEvents.length === 0 || !canEditScope}
          >
            <Plus className="h-4 w-4 mr-2" />
            Add Event
          </Button>
        </div>
        {hooksConfig?.path && (
          <p className="text-xs text-muted-foreground mt-2">{hooksConfig.path}</p>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4">
        {isLoading ? (
          <div className="flex flex-col items-center justify-center py-12 gap-3">
            <RefreshCw className="h-5 w-5 animate-spin text-primary" />
            <span className="text-xs text-muted-foreground">Loading...</span>
          </div>
        ) : events.length === 0 ? (
          <div className="text-center py-12 px-4">
            <div className="w-16 h-16 rounded-lg tars-panel flex items-center justify-center mx-auto mb-4">
              <Webhook className="h-8 w-8 text-muted-foreground" />
            </div>
            <p className="text-sm font-medium text-foreground">No hooks configured</p>
            <p className="text-xs text-muted-foreground mt-1">
              Hooks automate actions on Claude Code events
            </p>
            <Button
              variant="outline"
              size="sm"
              className="mt-4"
              onClick={() => setShowAddDialog(true)}
              disabled={availableEvents.length === 0}
            >
              <Plus className="h-4 w-4 mr-2" />
              Add your first hook
            </Button>
          </div>
        ) : (
          <div className="space-y-4">
            {events.map((event) => (
              <div key={event.event} className="tars-panel rounded-lg overflow-hidden">
                {/* Event header */}
                <button
                  onClick={() => toggleEvent(event.event)}
                  className="w-full px-4 py-3 flex items-center gap-3 hover:bg-muted/50 transition-colors"
                >
                  {expandedEvents.has(event.event) ? (
                    <ChevronDown className="h-4 w-4 text-muted-foreground" />
                  ) : (
                    <ChevronRight className="h-4 w-4 text-muted-foreground" />
                  )}
                  <h3 className="text-sm font-semibold text-primary">{event.event}</h3>
                  <span className="text-xs text-muted-foreground">
                    {EVENT_DESCRIPTIONS[event.event] || ''}
                  </span>
                  <span className="ml-auto text-xs text-muted-foreground">
                    {event.matchers.length} matcher{event.matchers.length !== 1 ? 's' : ''}
                  </span>
                </button>

                {/* Matchers */}
                {expandedEvents.has(event.event) && (
                  <div className="border-t border-border">
                    {event.matchers.map((matcher, matcherIndex) => (
                      <div key={matcherIndex} className="border-b border-border last:border-b-0">
                        {/* Matcher header */}
                        <div className="px-4 py-2 bg-muted/30 flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <code className="text-xs bg-background px-2 py-0.5 rounded font-mono">
                              {matcher.matcher}
                            </code>
                            <span className="text-xs text-muted-foreground">
                              {matcher.hooks.length} hook{matcher.hooks.length !== 1 ? 's' : ''}
                            </span>
                          </div>
                          <div className="flex items-center gap-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => openAddHookDialog(event.event, matcherIndex)}
                            >
                              <Plus className="h-3 w-3 mr-1" />
                              Hook
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => setHookToDelete({ event: event.event, matcherIndex })}
                              className="text-muted-foreground hover:text-destructive"
                            >
                              <Trash2 className="h-3 w-3" />
                            </Button>
                          </div>
                        </div>

                        {/* Hooks */}
                        <div className="px-4 py-2 space-y-2">
                          {matcher.hooks.length === 0 ? (
                            <p className="text-xs text-muted-foreground italic py-2">
                              No hooks defined. Add one to get started.
                            </p>
                          ) : (
                            matcher.hooks.map((hook, hookIndex) => (
                              <div
                                key={hookIndex}
                                className="flex items-start gap-3 p-2 rounded bg-background group"
                              >
                                <div className="flex-shrink-0 mt-0.5">
                                  {hook.type === 'command' ? (
                                    <Play className="h-4 w-4 text-green-500" />
                                  ) : (
                                    <MessageSquare className="h-4 w-4 text-blue-500" />
                                  )}
                                </div>
                                <div className="flex-1 min-w-0">
                                  <div className="flex items-center gap-2">
                                    <span className="text-xs font-medium uppercase text-muted-foreground">
                                      {hook.type}
                                    </span>
                                    {hook.timeout && (
                                      <span className="text-xs text-muted-foreground">
                                        ({hook.timeout}s timeout)
                                      </span>
                                    )}
                                  </div>
                                  <code className="text-xs font-mono break-all">
                                    {hook.type === 'command' ? hook.command : hook.prompt}
                                  </code>
                                </div>
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() =>
                                    setHookToDelete({
                                      event: event.event,
                                      matcherIndex,
                                      hookIndex,
                                    })
                                  }
                                  className="opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-destructive"
                                >
                                  <Trash2 className="h-3 w-3" />
                                </Button>
                              </div>
                            ))
                          )}
                        </div>
                      </div>
                    ))}

                    {/* Add matcher button */}
                    <div className="px-4 py-2 bg-muted/20">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => openAddMatcherDialog(event.event)}
                        className="text-xs"
                      >
                        <Plus className="h-3 w-3 mr-1" />
                        Add Matcher
                      </Button>
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Add Event Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Hook Event</DialogTitle>
            <DialogDescription>Select an event type to add hooks for.</DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="event-type">Event Type</Label>
            <select
              id="event-type"
              value={newEventType}
              onChange={(e) => setNewEventType(e.target.value)}
              className="tars-input w-full px-3 py-2 text-sm rounded mt-2"
            >
              <option value="">Select event...</option>
              {availableEvents.map((et) => (
                <option key={et} value={et}>
                  {et}
                </option>
              ))}
            </select>
            {newEventType && EVENT_DESCRIPTIONS[newEventType] && (
              <p className="text-xs text-muted-foreground mt-2">
                {EVENT_DESCRIPTIONS[newEventType]}
              </p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleAddEvent} disabled={!newEventType}>
              Add Event
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add Matcher Dialog */}
      <Dialog open={showAddMatcherDialog} onOpenChange={setShowAddMatcherDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Matcher</DialogTitle>
            <DialogDescription>
              Add a pattern to match tool names or other criteria.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Label htmlFor="matcher">Matcher Pattern</Label>
            <Input
              id="matcher"
              value={newMatcher}
              onChange={(e) => setNewMatcher(e.target.value)}
              placeholder="Write|Edit|Bash"
              className="mt-2"
            />
            <p className="text-xs text-muted-foreground mt-2">
              Use * to match all, or regex patterns like Write|Edit to match specific tools.
            </p>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddMatcherDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleAddMatcher} disabled={!newMatcher}>
              Add Matcher
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add Hook Dialog */}
      <Dialog open={showAddHookDialog} onOpenChange={setShowAddHookDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Hook</DialogTitle>
            <DialogDescription>
              Add a command or prompt to execute when the matcher triggers.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div>
              <Label>Hook Type</Label>
              <div className="flex gap-4 mt-2">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="hook-type"
                    checked={newHookType === 'command'}
                    onChange={() => setNewHookType('command')}
                    className="accent-primary"
                  />
                  <Play className="h-4 w-4 text-green-500" />
                  <span className="text-sm">Command</span>
                </label>
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="radio"
                    name="hook-type"
                    checked={newHookType === 'prompt'}
                    onChange={() => setNewHookType('prompt')}
                    className="accent-primary"
                  />
                  <MessageSquare className="h-4 w-4 text-blue-500" />
                  <span className="text-sm">Prompt</span>
                </label>
              </div>
            </div>

            {newHookType === 'command' ? (
              <div>
                <Label htmlFor="command">Command</Label>
                <Input
                  id="command"
                  value={newCommand}
                  onChange={(e) => setNewCommand(e.target.value)}
                  placeholder="bash /path/to/script.sh"
                  className="mt-2 font-mono text-sm"
                />
                <p className="text-xs text-muted-foreground mt-2">
                  Shell command to execute. Exit 0 = success, Exit 2 = error feedback.
                </p>
              </div>
            ) : (
              <div>
                <Label htmlFor="prompt">Prompt</Label>
                <textarea
                  id="prompt"
                  value={newPrompt}
                  onChange={(e) => setNewPrompt(e.target.value)}
                  placeholder="Validate the operation and return 'approve' or 'deny'..."
                  className="tars-input w-full px-3 py-2 text-sm rounded mt-2 min-h-[100px] resize-y"
                />
                <p className="text-xs text-muted-foreground mt-2">
                  Prompt for LLM to evaluate. Should return approve/deny or feedback.
                </p>
              </div>
            )}

            <div>
              <Label htmlFor="timeout">Timeout (seconds)</Label>
              <Input
                id="timeout"
                type="number"
                value={newTimeout}
                onChange={(e) => setNewTimeout(e.target.value)}
                placeholder="30"
                className="mt-2 w-24"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddHookDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleAddHook}
              disabled={newHookType === 'command' ? !newCommand : !newPrompt}
            >
              Add Hook
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={!!hookToDelete}
        onOpenChange={(open) => !open && setHookToDelete(null)}
        title={hookToDelete?.hookIndex !== undefined ? 'Delete Hook' : 'Delete Matcher'}
        description={
          hookToDelete?.hookIndex !== undefined
            ? 'Are you sure you want to delete this hook? This action cannot be undone.'
            : 'Are you sure you want to delete this matcher and all its hooks? This action cannot be undone.'
        }
        confirmLabel="Delete"
        confirmVariant="destructive"
        onConfirm={handleDelete}
        loading={deleting}
      />
    </div>
  );
}
