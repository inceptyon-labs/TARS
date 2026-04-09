import { ExternalLink, FolderOpen } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import { toast } from 'sonner';
import { Card, CardContent } from './ui/card';
import { Badge } from './ui/badge';
import { Button } from './ui/button';
import { cn } from '@/lib/utils';
import type { ProjectInfo, ProjectMetadata } from '@/lib/types';

function getLetterAvatarHue(name: string): number {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return Math.abs(hash) % 360;
}

interface AppCardProps {
  project: ProjectInfo;
  metadata: ProjectMetadata | null;
  iconDataUrl: string | null;
  category: string;
}

export function AppCard({ project, metadata, iconDataUrl, category }: AppCardProps) {
  const navigate = useNavigate();
  const hue = getLetterAvatarHue(project.name);
  const initial = project.name.charAt(0).toUpperCase();

  const primaryUrl =
    metadata?.production_url ?? (metadata?.domain ? `https://${metadata.domain}` : null);

  function handleCardClick() {
    navigate(`/projects?project=${project.id}`);
  }

  async function handleOpen() {
    try {
      if (primaryUrl) {
        await openUrl(primaryUrl);
      } else {
        await revealItemInDir(project.path);
      }
    } catch (err) {
      toast.error(`Failed to open: ${err}`);
    }
  }

  async function handleSecondaryLink(url: string) {
    try {
      await openUrl(url);
    } catch (err) {
      toast.error(`Failed to open: ${err}`);
    }
  }

  return (
    <Card
      className={cn(
        'group relative overflow-hidden py-0 gap-0',
        'hover:border-primary/30 hover:shadow-md transition-all duration-200 cursor-pointer'
      )}
      onClick={handleCardClick}
    >
      <CardContent className="p-5 flex flex-col items-center text-center gap-3">
        {/* Icon / Avatar */}
        <div className="w-16 h-16 rounded-2xl overflow-hidden shrink-0 flex items-center justify-center shadow-sm">
          {iconDataUrl ? (
            <img
              src={iconDataUrl}
              alt={project.name}
              className="w-full h-full object-contain p-1.5"
            />
          ) : (
            <div
              className="w-full h-full flex items-center justify-center text-2xl font-bold text-white"
              style={{ backgroundColor: `hsl(${hue}, 55%, 45%)` }}
            >
              {initial}
            </div>
          )}
        </div>

        {/* Name */}
        <h3 className="font-semibold text-sm truncate w-full">{project.name}</h3>

        {/* Description */}
        {metadata?.description && (
          <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">
            {metadata.description}
          </p>
        )}

        {/* Platform badges */}
        {metadata?.platforms && metadata.platforms.length > 0 && (
          <div className="flex flex-wrap gap-1 justify-center">
            {metadata.platforms.map((platform) => (
              <Badge key={platform} variant="secondary" className="text-[10px] px-1.5 py-0">
                {platform}
              </Badge>
            ))}
            {category !== 'Tools' && (
              <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                {category}
              </Badge>
            )}
          </div>
        )}

        {/* Action row */}
        <div className="flex items-center gap-1 mt-1" onClick={(e) => e.stopPropagation()}>
          <Button size="sm" variant={primaryUrl ? 'default' : 'secondary'} onClick={handleOpen}>
            {primaryUrl ? (
              <>
                <ExternalLink className="h-3 w-3" />
                Visit
              </>
            ) : (
              <>
                <FolderOpen className="h-3 w-3" />
                Open Folder
              </>
            )}
          </Button>

          {metadata?.github_url && (
            <Button
              size="icon"
              variant="ghost"
              className="h-8 w-8"
              onClick={() => handleSecondaryLink(metadata.github_url!)}
              title="GitHub"
            >
              <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z" />
              </svg>
            </Button>
          )}

          {metadata?.app_store_url && (
            <Button
              size="icon"
              variant="ghost"
              className="h-8 w-8"
              onClick={() => handleSecondaryLink(metadata.app_store_url!)}
              title="App Store"
            >
              <ExternalLink className="h-3.5 w-3.5" />
            </Button>
          )}

          {metadata?.play_store_url && (
            <Button
              size="icon"
              variant="ghost"
              className="h-8 w-8"
              onClick={() => handleSecondaryLink(metadata.play_store_url!)}
              title="Play Store"
            >
              <ExternalLink className="h-3.5 w-3.5" />
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
