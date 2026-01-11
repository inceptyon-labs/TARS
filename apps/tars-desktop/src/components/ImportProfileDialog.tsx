import { useState } from 'react';
import { X, FileUp, AlertCircle } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { previewProfileImport, importProfileJson } from '../lib/ipc';
import { Button } from './ui/button';
import type { PreviewImportResponse } from '../lib/types';

interface ImportProfileDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImportComplete: () => void;
}

export function ImportProfileDialog({
  open: isOpen,
  onOpenChange,
  onImportComplete,
}: ImportProfileDialogProps) {
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [preview, setPreview] = useState<PreviewImportResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [renameTo, setRenameTo] = useState('');
  const [importing, setImporting] = useState(false);

  const handleClose = () => {
    setSelectedFile(null);
    setPreview(null);
    setError(null);
    setRenameTo('');
    onOpenChange(false);
  };

  const handleSelectFile = async () => {
    try {
      const path = await open({
        filters: [{ name: 'TARS Profile', extensions: ['tars-profile.json', 'json'] }],
        multiple: false,
      });

      if (path && typeof path === 'string') {
        setSelectedFile(path);
        setError(null);
        setLoading(true);

        try {
          const previewResult = await previewProfileImport(path);
          setPreview(previewResult);
          if (previewResult.has_name_collision) {
            setRenameTo(previewResult.name + ' (imported)');
          }
        } catch (err) {
          setError(`Failed to preview file: ${err}`);
          setPreview(null);
        } finally {
          setLoading(false);
        }
      }
    } catch (err) {
      setError(`Failed to open file picker: ${err}`);
    }
  };

  const handleImport = async () => {
    if (!selectedFile) return;

    setImporting(true);
    setError(null);

    try {
      const finalName = preview?.has_name_collision ? renameTo.trim() : undefined;
      await importProfileJson(selectedFile, finalName);
      onImportComplete();
      handleClose();
    } catch (err) {
      setError(`Import failed: ${err}`);
    } finally {
      setImporting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={handleClose} />

      {/* Dialog */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b">
          <h2 className="text-lg font-semibold">Import Profile</h2>
          <button
            onClick={handleClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-4">
          {!selectedFile ? (
            <button
              onClick={handleSelectFile}
              className="w-full border-2 border-dashed border-border rounded-lg p-8 text-center hover:border-primary/50 hover:bg-muted/30 transition-colors"
            >
              <FileUp className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
              <p className="text-sm font-medium">Select a profile file</p>
              <p className="text-xs text-muted-foreground mt-1">.tars-profile.json files</p>
            </button>
          ) : loading ? (
            <div className="flex items-center justify-center py-8">
              <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
            </div>
          ) : preview ? (
            <div className="space-y-4">
              <div className="bg-muted/30 rounded-lg p-4">
                <div className="text-sm font-medium">{preview.name}</div>
                {preview.description && (
                  <div className="text-xs text-muted-foreground mt-1">{preview.description}</div>
                )}
                <div className="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
                  <span>{preview.tool_count} tools</span>
                  <span>v{preview.version}</span>
                </div>
              </div>

              {preview.has_name_collision && (
                <div className="space-y-2">
                  <div className="flex items-start gap-2 text-amber-500 text-sm">
                    <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
                    <span>
                      A profile named "{preview.name}" already exists. Please choose a new name.
                    </span>
                  </div>
                  <input
                    type="text"
                    value={renameTo}
                    onChange={(e) => setRenameTo(e.target.value)}
                    placeholder="Enter new name..."
                    className="w-full px-3 py-2 text-sm border border-border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>
              )}

              <p className="text-xs text-muted-foreground">
                File: {selectedFile.split(/[\\/]/).pop()}
              </p>
            </div>
          ) : null}

          {error && (
            <div className="flex items-start gap-2 text-destructive text-sm">
              <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
              <span>{error}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t bg-muted/30">
          <div>
            {selectedFile && (
              <Button variant="ghost" size="sm" onClick={handleSelectFile}>
                Choose Different File
              </Button>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" onClick={handleClose}>
              Cancel
            </Button>
            <Button
              onClick={handleImport}
              disabled={!preview || importing || (preview.has_name_collision && !renameTo.trim())}
            >
              {importing ? 'Importing...' : 'Import Profile'}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
