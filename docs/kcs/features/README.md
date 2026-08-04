# KCS index — user-facing features

Each file in this directory documents one tcl-lsp feature.  The `help`
subcommand (`tcl help`, the MCP `help` tool, and the VS Code `/help`
chat command) reads these files to build the feature catalogue.

## Format convention

Every file in this directory is a KCS note of type **Functionality**.
Use the [functionality template](../templates/kcs-template-functionality.md)
and follow the [KCS style guide](../STYLE.md) — British English,
Oxford comma, short plain sentences, one core question per note, no
inline contracts or file-path anchors (those belong in a design doc
under [`docs/design/`](../../design/README.md)).

The `help` tool parses the first few sections at runtime, so their
names and order must match the template exactly:

```markdown
# KCS: feature — <Feature Name>

> **Audience:** User
> **Type:** Functionality

## Summary

<one-line description>

## Applies to

<comma-separated plain-text tags — see docs/kcs/STYLE.md rule 11>

## How to use

<plain instructions; one sub-heading per editor or tool when they differ>
```

### Examples are mandatory

Every feature note must include at least one concrete example under an
`## Example` section. Pick whichever form shows the feature best, and
combine them when more than one form helps:

- **Before / after code** — for transforms (refactor, format, minify,
  optimise, unminify).
- **Where it appears** — for analysers (diagnostics, hover,
  completions, inlay hints, signature help, semantic tokens). Show a
  short snippet and say what the user sees on which token or line.
- **Screenshot** — for panels and visual features (compiler explorer,
  call hierarchy, debugger, document symbols). Reference an image
  from `../screenshots/` with a short caption.

## LSP features

- [kcs-feature-diagnostics.md](kcs-feature-diagnostics.md)
- [kcs-feature-byte-array-corruption.md](kcs-feature-byte-array-corruption.md)
- [kcs-feature-unknown-command-resolution.md](kcs-feature-unknown-command-resolution.md)
- [kcs-feature-unused-variables.md](kcs-feature-unused-variables.md)
- [kcs-feature-special-variables.md](kcs-feature-special-variables.md)
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
- [kcs-feature-command-option-highlighting.md](kcs-feature-command-option-highlighting.md)
- [kcs-feature-apl-language.md](kcs-feature-apl-language.md)
- [kcs-feature-selection-range.md](kcs-feature-selection-range.md)
- [kcs-feature-document-links.md](kcs-feature-document-links.md)
- [kcs-feature-code-lens.md](kcs-feature-code-lens.md)
- [kcs-feature-document-highlight.md](kcs-feature-document-highlight.md)
- [kcs-feature-type-navigation.md](kcs-feature-type-navigation.md)

## Editor commands

- [kcs-feature-optimiser.md](kcs-feature-optimiser.md)
- [kcs-feature-compiler-explorer.md](kcs-feature-compiler-explorer.md)
- [kcs-feature-var-escape-analysis.md](kcs-feature-var-escape-analysis.md)
- [kcs-feature-tk-preview.md](kcs-feature-tk-preview.md)
- [kcs-feature-runtime-validation.md](kcs-feature-runtime-validation.md)
- [kcs-feature-dialect-selection.md](kcs-feature-dialect-selection.md)
- [kcs-feature-text-transforms.md](kcs-feature-text-transforms.md)
- [kcs-feature-irule-extraction.md](kcs-feature-irule-extraction.md)
- [kcs-feature-irule-skeleton.md](kcs-feature-irule-skeleton.md)
- [kcs-feature-template-snippets.md](kcs-feature-template-snippets.md)
- [kcs-feature-package-scaffolding.md](kcs-feature-package-scaffolding.md)
- [kcs-feature-package-management.md](kcs-feature-package-management.md)
- [kcs-feature-extension-settings.md](kcs-feature-extension-settings.md)
- [kcs-feature-xc-translation.md](kcs-feature-xc-translation.md)
- [kcs-feature-minifier.md](kcs-feature-minifier.md)
- [kcs-feature-unminify-error.md](kcs-feature-unminify-error.md)

## CLI tools

- [kcs-feature-tcl-verb-cli.md](kcs-feature-tcl-verb-cli.md)
- [kcs-feature-debugger.md](kcs-feature-debugger.md)
- [kcs-feature-compilation-tools.md](kcs-feature-compilation-tools.md)
- [kcs-feature-command-info.md](kcs-feature-command-info.md)
- [kcs-feature-spec-studio.md](kcs-feature-spec-studio.md)
- [kcs-feature-tcllib-package-coverage.md](kcs-feature-tcllib-package-coverage.md)
- [kcs-feature-event-registry.md](kcs-feature-event-registry.md)
- [kcs-feature-semantic-graphs.md](kcs-feature-semantic-graphs.md)
- [kcs-feature-control-flow-diagrams.md](kcs-feature-control-flow-diagrams.md)
- [kcs-feature-irule-review.md](kcs-feature-irule-review.md)
- [kcs-feature-bigip-cleanup.md](kcs-feature-bigip-cleanup.md)
- [kcs-feature-bigip-grep.md](kcs-feature-bigip-grep.md)
- [kcs-feature-bigip-query.md](kcs-feature-bigip-query.md)
- [kcs-feature-bigip-registry.md](kcs-feature-bigip-registry.md)
- [kcs-feature-bigip-report-apm-tab.md](kcs-feature-bigip-report-apm-tab.md)
- [kcs-feature-bigip-report-profile-defaults.md](kcs-feature-bigip-report-profile-defaults.md)
- [kcs-feature-f5-cli.md](kcs-feature-f5-cli.md)
- [kcs-feature-f5-secret-crypto.md](kcs-feature-f5-secret-crypto.md)
- [kcs-feature-f5-query-renderers.md](kcs-feature-f5-query-renderers.md)
- [kcs-feature-f5-secret-crypto.md](kcs-feature-f5-secret-crypto.md)

## AI features

- [kcs-feature-ai-chat-irule.md](kcs-feature-ai-chat-irule.md)
- [kcs-feature-ai-chat-tcl.md](kcs-feature-ai-chat-tcl.md)
- [kcs-feature-ai-chat-tk.md](kcs-feature-ai-chat-tk.md)
- [kcs-feature-chat-slash-commands.md](kcs-feature-chat-slash-commands.md)
- [kcs-feature-ai-help.md](kcs-feature-ai-help.md)
- [kcs-feature-documentation-generation.md](kcs-feature-documentation-generation.md)
- [kcs-feature-diff.md](kcs-feature-diff.md)
- [kcs-feature-code-generation.md](kcs-feature-code-generation.md)
- [kcs-feature-fakecmp-tools.md](kcs-feature-fakecmp-tools.md)
- [kcs-feature-modernisation-tools.md](kcs-feature-modernisation-tools.md)
- [kcs-feature-mcp-server.md](kcs-feature-mcp-server.md)
- [kcs-feature-claude-code-skills.md](kcs-feature-claude-code-skills.md)
- [kcs-feature-tcl-pkg.md](kcs-feature-tcl-pkg.md)
- [kcs-feature-tcl-venv.md](kcs-feature-tcl-venv.md)
- [kcs-feature-bpf-tcl.md](kcs-feature-bpf-tcl.md)
