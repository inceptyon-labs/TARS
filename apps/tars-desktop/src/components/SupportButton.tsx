/**
 * SupportButton - Ko-fi and ETH donation links
 */

import { useState } from 'react';

const ETH_ADDRESS = '0x42353a7Fc70Eab5C0017733813805313B7b10b8B';

export function SupportButton() {
  const [copied, setCopied] = useState(false);

  const handleEthClick = () => {
    // Try to open wallet first, fallback to clipboard
    if (typeof window !== 'undefined' && typeof window.ethereum !== 'undefined') {
      window.location.href = `ethereum:${ETH_ADDRESS}`;
    } else {
      navigator.clipboard.writeText(ETH_ADDRESS).then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      });
    }
  };

  return (
    <div className="flex items-center gap-2">
      {/* Ko-fi Button */}
      <a
        href="https://ko-fi.com/P5P51QJAJ5"
        target="_blank"
        rel="noopener noreferrer"
        className="inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium transition-colors text-white hover:opacity-90"
        style={{ backgroundColor: '#72a4f2' }}
      >
        <svg viewBox="0 0 24 24" className="w-4 h-4 fill-current">
          <path d="M23.881 8.948c-.773-4.085-4.859-4.593-4.859-4.593H.723c-.604 0-.679.798-.679.798s-.082 7.324-.022 11.822c.164 2.424 2.586 2.672 2.586 2.672s8.267-.023 11.966-.049c2.438-.426 2.683-2.566 2.658-3.734 4.352.24 7.422-2.831 6.649-6.916zm-11.062 3.511c-1.246 1.453-4.011 3.976-4.011 3.976s-.121.119-.31.023c-.076-.057-.108-.09-.108-.09-.443-.441-3.368-3.049-4.034-3.954-.709-.965-1.041-2.7-.091-3.71.951-1.01 3.005-1.086 4.363.407 0 0 1.565-1.782 3.468-.963 1.904.82 1.832 3.011.723 4.311zm6.173.478c-.928.116-1.682.028-1.682.028V7.284h1.77s1.971.551 1.971 2.638c0 1.913-.985 2.667-2.059 3.015z" />
        </svg>
        <span>Ko-fi</span>
      </a>

      {/* ETH Button */}
      <button
        onClick={handleEthClick}
        className="inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium transition-colors border border-border hover:bg-muted/50"
        title={copied ? 'Copied!' : 'Send ETH or click to copy address'}
      >
        <svg viewBox="0 0 320 512" className="w-3.5 h-3.5 fill-current">
          <path d="M311.9 260.8L160 353.6 8 260.8 160 0l151.9 260.8zM160 383.4L8 290.6 160 512l152-221.4-152 92.8z" />
        </svg>
        <span className="font-mono text-xs">{copied ? 'Copied!' : '0x4235...0b8B'}</span>
      </button>
    </div>
  );
}

// Extend Window interface for ethereum
declare global {
  interface Window {
    ethereum?: {
      isMetaMask?: boolean;
      request?: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
    };
  }
}
