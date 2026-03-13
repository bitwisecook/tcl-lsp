# KCS index — user-facing features

Each file in this directory documents one tcl-lsp feature.  The `help`
subcommand (`tcl_ai.py help`, the MCP `help` tool, and the VS Code `/help`
chat command) reads these files at runtime to build the feature catalogue.

## Format convention

Every file must follow this structure so the help parser can extract metadata:

```markdown
# KCS: feature — <Feature Name>

## Summary

<one-line description>

## Surface

<comma-separated list: lsp, vscode-command, vscode-chat, mcp, claude-code, all-editors>

## How to use

<brief usage instructions — may differ per surface>

## Operational context / File-path anchors / ...
<standard KCS sections>
```

## LSP features

- [kcs-feature-diagnostics.md](kcs-feature-diagnostics.md)
- [kcs-feature-unused-variables.md](kcs-feature-unused-variables.md)
- [kcs-feature-completions.md](kcs-feature-completions.md)
- [kcs-feature-hover.md](kcs-feature-hover.md)
- [kcs-feature-definition.md](kcs-feature-definition.md)
- [kcs-feature-references.md](kcs-feature-references.md)
- [kcs-feature-document-symbols.md](kcs-feature-document-symbols.md)
- [kcs-feature-workspace-symbols.md](kcs-feature-workspace-symbols.md)
- [kcs-feature-formatting.md](kcs-feature-formatting.md)
- [kcs-feature-code-actions.md](kcs-feature-code-actions.md)
- [kcs-feature-refactorings.md](kcs-feature-refactorings.md)
  - [kcs-feature-refactor-extract-variable.md](kcs-feature-refactor-extract-variable.md)
  - [kcs-feature-refactor-inline-variable.md](kcs-feature-refactor-inline-variable.md)
  - [kcs-feature-refactor-if-to-switch.md](kcs-feature-refactor-if-to-switch.md)
  - [kcs-feature-refactor-switch-to-dict.md](kcs-feature-refactor-switch-to-dict.md)
  - [kcs-feature-refactor-brace-expr.md](kcs-feature-refactor-brace-expr.md)
  - [kcs-feature-refactor-extract-datagroup.md](kcs-feature-refactor-extract-datagroup.md)
- [kcs-feature-rename.md](kcs-feature-rename.md)
- [kcs-feature-signature-help.md](kcs-feature-signature-help.md)
- [kcs-feature-folding.md](kcs-feature-folding.md)
- [kcs-feature-inlay-hints.md](kcs-feature-inlay-hints.md)
- [kcs-feature-call-hierarchy.md](kcs-feature-call-hierarchy.md)
- [kcs-feature-semantic-tokens.md](kcs-feature-semantic-tokens.md)
- [kcs-feature-selection-range.md](kcs-feature-selection-range.md)
- [kcs-feature-document-links.md](kcs-feature-document-links.md)

## Editor commands

- [kcs-feature-optimiser.md](kcs-feature-optimiser.md)
- [kcs-feature-compiler-explorer.md](kcs-feature-compiler-explorer.md)
- [kcs-feature-tk-preview.md](kcs-feature-tk-preview.md)
- [kcs-feature-runtime-validation.md](kcs-feature-runtime-validation.md)
- [kcs-feature-dialect-selection.md](kcs-feature-dialect-selection.md)
- [kcs-feature-text-transforms.md](kcs-feature-text-transforms.md)
- [kcs-feature-irule-extraction.md](kcs-feature-irule-extraction.md)
- [kcs-feature-irule-skeleton.md](kcs-feature-irule-skeleton.md)
- [kcs-feature-template-snippets.md](kcs-feature-template-snippets.md)
- [kcs-feature-package-scaffolding.md](kcs-feature-package-scaffolding.md)
- [kcs-feature-xc-translation.md](kcs-feature-xc-translation.md)
- [kcs-feature-minifier.md](kcs-feature-minifier.md)
- [kcs-feature-unminify-error.md](kcs-feature-unminify-error.md)

## CLI tools

- [kcs-feature-tcl-verb-cli.md](kcs-feature-tcl-verb-cli.md)

## AI features

- [kcs-feature-ai-chat-irule.md](kcs-feature-ai-chat-irule.md)
- [kcs-feature-ai-chat-tcl.md](kcs-feature-ai-chat-tcl.md)
- [kcs-feature-ai-chat-tk.md](kcs-feature-ai-chat-tk.md)
- [kcs-feature-mcp-server.md](kcs-feature-mcp-server.md)
- [kcs-feature-claude-code-skills.md](kcs-feature-claude-code-skills.md)
