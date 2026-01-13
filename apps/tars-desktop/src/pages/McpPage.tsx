/**
 * MCP Servers management page
 *
 * Displays the McpPanel for managing MCP servers.
 */

import { HelpButton } from '../components/HelpButton';
import { McpPanel } from '../components/config/McpPanel';

export function McpPage() {
  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="h-14 border-b border-border px-6 flex items-center justify-between shrink-0 tars-header relative z-10">
        <div className="flex items-center gap-3">
          <div className="tars-indicator" />
          <h2 className="text-lg font-semibold tracking-wide">MCP Servers</h2>
          <HelpButton section="MCP" />
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        <McpPanel />
      </div>
    </div>
  );
}
