/**
 * MCP Servers management page
 *
 * Displays the McpPanel for managing MCP servers.
 */

import { McpPanel } from '../components/config/McpPanel';

export function McpPage() {
  return (
    <div className="h-full overflow-auto">
      <McpPanel />
    </div>
  );
}
