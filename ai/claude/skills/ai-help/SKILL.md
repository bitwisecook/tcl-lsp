---
name: ai-help
description: "Show what features and AI tools are available in the tcl-lsp extension across VS Code, other editors, Claude Code skills, and the MCP server. Answers 'what can you do?' questions. Use when asking about tcl-lsp features, finding available AI tools, discovering Claude Code skills, or getting help with the tcl-lsp extension."
allowed-tools: mcp__tcl-lsp__help, Read
---

# AI Help -- tcl-lsp feature guide

## Steps

1. Call `mcp__tcl-lsp__help` (empty `topic` for the full catalogue; a
   `topic` to search). If it fails, read `README.md`.
2. For a specific topic ("how do I validate?", "what MCP tools exist?",
   "set up Neovim") focus there, reading the editor's README if needed
   (`editors/vscode/package.json` for commands, settings, and chat
   participants; `editors/<neovim|emacs|zed|helix|sublime-text|jetbrains>/README.md`).
   For "what can you do?" give an overview of every area.
3. Say that one analysis engine powers every surface: the LSP server, the
   MCP tools, the Claude Code skills, and the VS Code chat participants.

## Output

Concise bullets grouped as **Editor (LSP)**, **AI Chat (VS Code)**, **Claude
Code Skills**, **MCP Tools**; highlight what fits the user's context; include
setup steps from the README when the question is about one editor.

$ARGUMENTS
