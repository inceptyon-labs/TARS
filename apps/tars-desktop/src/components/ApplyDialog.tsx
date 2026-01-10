import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';
import { DiffPreview } from './DiffPreview';
import { AlertTriangle, CheckCircle, Loader2, RotateCcw, Play, Eye } from 'lucide-react';
import type {
  DiffPreview as DiffPreviewType,
  ProfileInfo,
  ProjectInfo,
  BackupInfo,
} from '../lib/types';

interface ApplyDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  profile: ProfileInfo;
  project: ProjectInfo;
  onApplySuccess?: (backup: BackupInfo) => void;
}

type ApplyStep = 'preview' | 'applying' | 'success' | 'error';

export function ApplyDialog({
  open,
  onOpenChange,
  profile,
  project,
  onApplySuccess,
}: ApplyDialogProps) {
  const [step, setStep] = useState<ApplyStep>('preview');
  const [diffPreview, setDiffPreview] = useState<DiffPreviewType | null>(null);
  const [backup, setBackup] = useState<BackupInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const loadPreview = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const preview = await invoke<DiffPreviewType>('generate_diff_preview', {
        profileId: profile.id,
        projectId: project.id,
      });
      setDiffPreview(preview);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleApply = async () => {
    setStep('applying');
    setError(null);
    try {
      const result = await invoke<BackupInfo>('apply_profile', {
        profileId: profile.id,
        projectId: project.id,
      });
      setBackup(result);
      setStep('success');
      onApplySuccess?.(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStep('error');
    }
  };

  const handleRollback = async () => {
    if (!backup) return;

    setIsLoading(true);
    setError(null);
    try {
      await invoke('rollback_backup', {
        backupId: backup.id,
        projectId: project.id,
      });
      // Close dialog after successful rollback
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    setStep('preview');
    setDiffPreview(null);
    setBackup(null);
    setError(null);
    onOpenChange(false);
  };

  // Load preview when dialog opens
  if (open && !diffPreview && !isLoading && !error && step === 'preview') {
    loadPreview();
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="max-w-4xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {step === 'preview' && (
              <>
                <Eye className="h-5 w-5" />
                Apply Profile: {profile.name}
              </>
            )}
            {step === 'applying' && (
              <>
                <Loader2 className="h-5 w-5 animate-spin" />
                Applying Changes...
              </>
            )}
            {step === 'success' && (
              <>
                <CheckCircle className="h-5 w-5 text-green-500" />
                Applied Successfully
              </>
            )}
            {step === 'error' && (
              <>
                <AlertTriangle className="h-5 w-5 text-red-500" />
                Apply Failed
              </>
            )}
          </DialogTitle>
          <DialogDescription>
            {step === 'preview' && `Review changes that will be applied to ${project.name}`}
            {step === 'applying' && 'Please wait while the profile is being applied...'}
            {step === 'success' &&
              'The profile has been applied successfully. A backup was created.'}
            {step === 'error' && 'An error occurred while applying the profile.'}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-hidden">
          {/* Loading state */}
          {isLoading && step === 'preview' && (
            <div className="flex items-center justify-center h-64">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          )}

          {/* Preview step */}
          {step === 'preview' && diffPreview && (
            <div className="h-full overflow-auto">
              <DiffPreview preview={diffPreview} />
            </div>
          )}

          {/* Applying step */}
          {step === 'applying' && (
            <div className="flex flex-col items-center justify-center h-64 gap-4">
              <Loader2 className="h-12 w-12 animate-spin text-primary" />
              <div className="text-center">
                <p className="text-sm text-muted-foreground">
                  Creating backup and applying changes...
                </p>
              </div>
            </div>
          )}

          {/* Success step */}
          {step === 'success' && backup && (
            <div className="flex flex-col items-center justify-center h-64 gap-4">
              <CheckCircle className="h-12 w-12 text-green-500" />
              <div className="text-center space-y-2">
                <p className="font-medium">Profile applied successfully!</p>
                <p className="text-sm text-muted-foreground">
                  Backup created: {backup.files_count} files backed up
                </p>
                <p className="text-xs text-muted-foreground">
                  You can rollback to the previous state at any time.
                </p>
              </div>
            </div>
          )}

          {/* Error step */}
          {step === 'error' && (
            <div className="flex flex-col items-center justify-center h-64 gap-4">
              <AlertTriangle className="h-12 w-12 text-red-500" />
              <div className="text-center space-y-2">
                <p className="font-medium text-red-600">Failed to apply profile</p>
                <p className="text-sm text-muted-foreground max-w-md">{error}</p>
              </div>
            </div>
          )}

          {/* Error in preview */}
          {step === 'preview' && error && !isLoading && (
            <div className="flex flex-col items-center justify-center h-64 gap-4">
              <AlertTriangle className="h-12 w-12 text-red-500" />
              <div className="text-center space-y-2">
                <p className="font-medium text-red-600">Failed to generate preview</p>
                <p className="text-sm text-muted-foreground max-w-md">{error}</p>
              </div>
              <Button variant="outline" onClick={loadPreview}>
                Retry
              </Button>
            </div>
          )}
        </div>

        <DialogFooter>
          {step === 'preview' && diffPreview && (
            <>
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
              <Button onClick={handleApply} disabled={diffPreview.operations.length === 0}>
                <Play className="h-4 w-4 mr-2" />
                Apply Changes
              </Button>
            </>
          )}

          {step === 'success' && (
            <>
              <Button variant="outline" onClick={handleRollback} disabled={isLoading}>
                <RotateCcw className="h-4 w-4 mr-2" />
                Rollback
              </Button>
              <Button onClick={handleClose}>Done</Button>
            </>
          )}

          {step === 'error' && (
            <>
              <Button variant="outline" onClick={handleClose}>
                Close
              </Button>
              <Button onClick={() => setStep('preview')}>Try Again</Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
