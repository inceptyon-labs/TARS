import { useEffect, useRef, type ReactNode } from 'react';
import { toast } from 'sonner';

interface CodeBlockCopyButtonProps {
  children: ReactNode;
}

/**
 * Wrapper component that:
 * 1. Adds copy buttons to CodeMirror code blocks
 * 2. Makes inline code clickable to copy
 * 3. Removes visible backticks from inline code
 */
export function CodeBlockCopyButton({ children }: CodeBlockCopyButtonProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const processCodeElements = () => {
      // Process code blocks - add copy button
      const cmEditors = container.querySelectorAll('.cm-editor');
      cmEditors.forEach((cmEditor) => {
        if (cmEditor.querySelector('.code-copy-btn')) return;

        const button = document.createElement('button');
        button.className = 'code-copy-btn';
        button.setAttribute('aria-label', 'Copy code');
        button.setAttribute('title', 'Copy code');
        button.type = 'button';
        button.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>`;

        button.addEventListener('click', async (e) => {
          e.preventDefault();
          e.stopPropagation();
          const cmContent = cmEditor.querySelector('.cm-content');
          if (!cmContent) return;
          const code = cmContent.textContent || '';
          try {
            await navigator.clipboard.writeText(code);
            button.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>`;
            setTimeout(() => {
              button.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>`;
            }, 2000);
          } catch (err) {
            console.error('Failed to copy code:', err);
          }
        });

        cmEditor.appendChild(button);
      });

      // Process inline code - remove backticks and add click-to-copy
      const inlineCodes = container.querySelectorAll('code[data-lexical-text="true"]');
      inlineCodes.forEach((codeEl) => {
        if (codeEl.hasAttribute('data-processed')) return;
        codeEl.setAttribute('data-processed', 'true');

        // Remove backtick text nodes
        const childNodes = Array.from(codeEl.childNodes);
        childNodes.forEach((node) => {
          if (node.nodeType === Node.TEXT_NODE) {
            const text = node.textContent || '';
            // Remove backticks from text nodes
            if (text.includes('`')) {
              node.textContent = text.replace(/`/g, '');
              // If empty after removing backticks, remove the node
              if (!node.textContent?.trim()) {
                node.remove();
              }
            }
          }
        });

        // Add click-to-copy
        (codeEl as HTMLElement).style.cursor = 'pointer';
        codeEl.setAttribute('title', 'Click to copy');

        codeEl.addEventListener('click', async (e) => {
          e.preventDefault();
          e.stopPropagation();

          // Get the code text from the inner span
          const codeSpan = codeEl.querySelector('span[class*="_code_"]');
          const code = codeSpan?.textContent || codeEl.textContent || '';

          try {
            await navigator.clipboard.writeText(code);
            toast.success('Copied to clipboard');
          } catch (err) {
            console.error('Failed to copy:', err);
          }
        });
      });
    };

    // Initial processing with delay
    const initialTimeout = setTimeout(processCodeElements, 300);

    // Watch for changes
    const observer = new MutationObserver(() => {
      setTimeout(processCodeElements, 100);
    });

    observer.observe(container, {
      childList: true,
      subtree: true,
      characterData: true,
    });

    return () => {
      clearTimeout(initialTimeout);
      observer.disconnect();
    };
  }, []);

  return <div ref={containerRef}>{children}</div>;
}
