import { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Package, CheckCircle, Loader2, AlertTriangle, FolderOpen } from 'lucide-react';
import { exportProfileAsPlugin } from '../lib/ipc';
import type { ProfileInfo } from '../lib/types';

interface ExportPluginDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  profile: Pick<ProfileInfo, 'id' | 'name'>;
  onExportSuccess?: (outputPath: string) => void;
}

type ExportStep = 'form' | 'exporting' | 'success' | 'error';

export function ExportPluginDialog({
  open,
  onOpenChange,
  profile,
  onExportSuccess,
}: ExportPluginDialogProps) {
  const [step, setStep] = useState<ExportStep>('form');
  const [pluginName, setPluginName] = useState(profile.name.toLowerCase().replace(/\s+/g, '-'));
  const [version, setVersion] = useState('1.0.0');
  const [outputPath, setOutputPath] = useState<string>('');
  const [exportedPath, setExportedPath] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  const handleSelectOutput = async () => {
    try {
      const selected = await save({
        title: 'Select Export Location',
        defaultPath: `${pluginName}.zip`,
        filters: [
          {
            name: 'ZIP Archive',
            extensions: ['zip'],
          },
        ],
      });
      if (selected) {
        setOutputPath(selected);
      }
    } catch (err) {
      console.error('Failed to select output path:', err);
    }
  };

  const handleExport = async () => {
    if (!outputPath) {
      setError('Please select an output location');
      return;
    }

    setStep('exporting');
    setError(null);

    try {
      const result = await exportProfileAsPlugin(profile.id, outputPath, {
        name: pluginName,
        version,
      });
      setExportedPath(result);
      setStep('success');
      onExportSuccess?.(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStep('error');
    }
  };

  const handleClose = () => {
    setStep('form');
    setPluginName(profile.name.toLowerCase().replace(/\s+/g, '-'));
    setVersion('1.0.0');
    setOutputPath('');
    setExportedPath('');
    setError(null);
    onOpenChange(false);
  };

  const isValidSemver = (v: string) => {
    return /^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$/.test(v);
  };

  const isValidName = (name: string) => {
    return /^[@a-z0-9][-a-z0-9/]*$/.test(name);
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {step === 'form' && (
              <>
                <Package className="h-5 w-5" />
                Export as Plugin
              </>
            )}
            {step === 'exporting' && (
              <>
                <Loader2 className="h-5 w-5 animate-spin" />
                Exporting...
              </>
            )}
            {step === 'success' && (
              <>
                <CheckCircle className="h-5 w-5 text-green-500" />
                Export Complete
              </>
            )}
            {step === 'error' && (
              <>
                <AlertTriangle className="h-5 w-5 text-red-500" />
                Export Failed
              </>
            )}
          </DialogTitle>
          <DialogDescription>
            {step === 'form' && `Export "${profile.name}" as a Claude Code plugin (ZIP)`}
            {step === 'exporting' && 'Please wait while the plugin is being created...'}
            {step === 'success' && 'Your plugin has been exported successfully.'}
            {step === 'error' && 'An error occurred while exporting the plugin.'}
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          {/* Form step */}
          {step === 'form' && (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="pluginName">Plugin Name</Label>
                <Input
                  id="pluginName"
                  value={pluginName}
                  onChange={(e) => setPluginName(e.target.value)}
                  placeholder="my-plugin"
                />
                {pluginName && !isValidName(pluginName) && (
                  <p className="text-xs text-red-500">
                    Name must be lowercase, start with a letter or @, and contain only alphanumeric
                    characters, hyphens, or slashes
                  </p>
                )}
              </div>

              <div className="space-y-2">
                <Label htmlFor="version">Version</Label>
                <Input
                  id="version"
                  value={version}
                  onChange={(e) => setVersion(e.target.value)}
                  placeholder="1.0.0"
                />
                {version && !isValidSemver(version) && (
                  <p className="text-xs text-red-500">
                    Version must be valid semver (e.g., 1.0.0, 1.0.0-beta.1)
                  </p>
                )}
              </div>

              <div className="space-y-2">
                <Label>Output Location</Label>
                <div className="flex gap-2">
                  <Input
                    value={outputPath}
                    onChange={(e) => setOutputPath(e.target.value)}
                    placeholder="Select output location..."
                    className="flex-1"
                    readOnly
                  />
                  <Button variant="outline" onClick={handleSelectOutput}>
                    <FolderOpen className="h-4 w-4" />
                  </Button>
                </div>
              </div>

              {error && <p className="text-sm text-red-500">{error}</p>}
            </div>
          )}

          {/* Exporting step */}
          {step === 'exporting' && (
            <div className="flex flex-col items-center justify-center py-8 gap-4">
              <Loader2 className="h-12 w-12 animate-spin text-primary" />
              <p className="text-sm text-muted-foreground">Creating plugin archive...</p>
            </div>
          )}

          {/* Success step */}
          {step === 'success' && (
            <div className="flex flex-col items-center justify-center py-8 gap-4">
              <CheckCircle className="h-12 w-12 text-green-500" />
              <div className="text-center space-y-2">
                <p className="font-medium">Plugin exported successfully!</p>
                <p className="text-sm text-muted-foreground break-all">{exportedPath}</p>
                <p className="text-xs text-muted-foreground mt-4">
                  Unzip and install with:{' '}
                  <code className="bg-muted px-1 py-0.5 rounded">
                    claude plugin install &lt;folder&gt;
                  </code>
                </p>
              </div>
            </div>
          )}

          {/* Error step */}
          {step === 'error' && (
            <div className="flex flex-col items-center justify-center py-8 gap-4">
              <AlertTriangle className="h-12 w-12 text-red-500" />
              <div className="text-center space-y-2">
                <p className="font-medium text-red-600">Export failed</p>
                <p className="text-sm text-muted-foreground max-w-sm">{error}</p>
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          {step === 'form' && (
            <>
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
              <Button
                onClick={handleExport}
                disabled={
                  !pluginName ||
                  !version ||
                  !outputPath ||
                  !isValidName(pluginName) ||
                  !isValidSemver(version)
                }
              >
                <Package className="h-4 w-4 mr-2" />
                Export Plugin
              </Button>
            </>
          )}

          {step === 'success' && <Button onClick={handleClose}>Done</Button>}

          {step === 'error' && (
            <>
              <Button variant="outline" onClick={handleClose}>
                Close
              </Button>
              <Button onClick={() => setStep('form')}>Try Again</Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
