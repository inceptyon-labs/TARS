/**
 * Custom MDXEditor plugin to add triple-backtick (```) shortcut for code blocks.
 *
 * When user types ``` at the start of a paragraph and presses Enter,
 * it converts to a code block.
 */

import { realmPlugin, insertCodeBlock$, activeEditor$ } from '@mdxeditor/editor';
import {
  $getSelection,
  $isRangeSelection,
  TextNode,
  COMMAND_PRIORITY_HIGH,
  KEY_ENTER_COMMAND,
  type LexicalEditor,
} from 'lexical';
import { $isCodeNode } from '@lexical/code';

/**
 * Plugin that adds markdown-style triple backtick shortcut for code blocks.
 * Type ``` and press Enter to create a code block.
 */
export const codeBlockShortcutPlugin = realmPlugin({
  init(realm) {
    // Subscribe to activeEditor changes
    realm.sub(activeEditor$, (editor: LexicalEditor | null) => {
      if (!editor) return;

      // Register transform for text nodes to detect ```
      const removeTransform = editor.registerNodeTransform(TextNode, (textNode: TextNode) => {
        const text = textNode.getTextContent();

        // Check if text starts with ``` (with optional language after)
        const match = text.match(/^```(\w*)$/);
        if (!match) return;

        // Get the parent - should be a paragraph
        const parent = textNode.getParent();
        if (!parent || $isCodeNode(parent)) return;

        // Only trigger if this is the only content in the paragraph
        if (parent.getChildrenSize() !== 1) return;

        // Get the language (if specified)
        const language = match[1] || '';

        // Remove the paragraph with backticks and insert code block
        editor.update(() => {
          parent.remove();
          realm.pub(insertCodeBlock$, { language, code: '' });
        });
      });

      // Also handle Enter key after typing ```
      const removeCommand = editor.registerCommand(
        KEY_ENTER_COMMAND,
        (event: KeyboardEvent | null) => {
          const selection = $getSelection();
          if (!$isRangeSelection(selection)) return false;

          const anchorNode = selection.anchor.getNode();
          if (!(anchorNode instanceof TextNode)) return false;

          const text = anchorNode.getTextContent();
          const match = text.match(/^```(\w*)$/);
          if (!match) return false;

          // Get the parent
          const parent = anchorNode.getParent();
          if (!parent || parent.getChildrenSize() !== 1) return false;

          event?.preventDefault();

          const language = match[1] || '';

          // Remove the paragraph and insert code block
          parent.remove();
          realm.pub(insertCodeBlock$, { language, code: '' });

          return true;
        },
        COMMAND_PRIORITY_HIGH
      );

      // Cleanup on editor change (return cleanup function)
      return () => {
        removeTransform();
        removeCommand();
      };
    });
  },
});
