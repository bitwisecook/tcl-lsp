---
name: ai-help
description: >
  Show what features and AI tools are available in the tcl-lsp extension,
  how to use them in VS Code, other editors, Claude Code skills, and the
  MCP server.  Answers "what can you do?" questions.
allowed-tools: Bash, Read
---

# AI Help — tcl-lsp feature guide

Answer questions about what features the tcl-lsp extension provides and how
to use them across VS Code, other editors, Claude Code, and the MCP server.

## Steps

1. Read the feature catalogue built into the server:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py help
   ```
2. If the user asked about a **specific topic** (e.g. "how do I validate?",
   "what MCP tools exist?", "how do I set up Neovim?"), focus on that area
   and read the relevant editor README if needed:
   - VS Code: `editors/vscode/package.json` (commands, settings, chat participants)
   - Neovim: `editors/neovim/README.md`
   - Emacs: `editors/emacs/README.md`
   - Zed: `editors/zed/README.md`
   - Helix: `editors/helix/README.md`
   - Sublime Text: `editors/sublime-text/README.md`
   - JetBrains: `editors/jetbrains/README.md`
3. If the user asked a **general** question ("what can you do?", "help"),
   give an overview of all feature areas with brief descriptions.
4. Always mention that the same analysis engine powers all surfaces:
   the LSP server, the MCP tools, the Claude Code skills, and the VS Code
   chat participants.

## Output format

- Group features by surface area: **Editor (LSP)**, **AI Chat (VS Code)**,
  **Claude Code Skills**, **MCP Tools**
- Use concise bullet lists
- Highlight the most useful commands for the user's context
- If the user's question is about a specific editor, include setup
  instructions from the relevant README

$ARGUMENTS
