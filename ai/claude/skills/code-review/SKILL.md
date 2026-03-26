---
name: code-review
description: >
  Comprehensive code review of pending changes in the repository. Critiques
  quality, simplicity, correctness, dead code, LSP/editor integration
  completeness, KCS currency, AI/CLI tooling, compiler explorer usage,
  test coverage, and Tcl dialect compatibility (8.4-9.0).
allowed-tools: Bash, Read, Glob, Grep
---

# Code Review

Perform a thorough review of the current changes in the repository. Make no
modifications — only critique.

## Gather context

1. Ask the user what the changes are about — what problem they solve, what
   feature they add, or what refactoring they perform. Use this context to
   guide your review.
2. Identify the scope of the changes:
   ```bash
   git diff main --stat
   git diff main --name-only
   ```
3. Read every changed file. For large diffs, read the full diff:
   ```bash
   git diff main -- <file>
   ```

## Review checklist

Work through each area below. For each area, note findings as **pass**,
**concern**, or **action required**. Skip areas that are not relevant to the
changes.

### 1. Quality and simplicity

- Is the code as simple as it can reasonably be?
- Are there unnecessary abstractions, premature generalisations, or
  over-engineered helpers?
- Are there structures, shims, or legacy compatibility layers that are no
  longer necessary?
- Is there dead code, replaced code, or unreachable paths?
- Do names follow project conventions (UK spelling, explicit names, no
  ambiguous single-letter variables)?

### 2. Correctness

- Is the logic sound? Are edge cases handled?
- Are there off-by-one errors, missing bounds checks, or silent failures?
- For Tcl language features: consider namespaces, qualified names, aliases,
  `unknown` proc, shimmering, `package` loading, `source`, `auto_index`,
  `uplevel`/`upvar` scoping, `namespace eval`, `interp`, `rename`, and
  `trace` interactions.
- Is the code compatible with all supported Tcl dialects (8.4, 8.5, 8.6,
  9.0)? Are version-specific features gated appropriately?

### 3. Optimisations

- Are obvious optimisations done (algorithmic improvements, avoiding
  redundant work, caching where appropriate)?
- Are there hot paths doing unnecessary allocations or repeated lookups?

### 4. LSP and editor integration

- If the change affects diagnostics, completions, hovers, semantic tokens,
  code actions, or formatting: is the integration complete for **all**
  editors (VS Code, Neovim, Zed, Emacs, Helix, Sublime, JetBrains)?
- Were editor settings regenerated (`make gen-editor-settings`) if
  diagnostics or optimisations changed?
- Are new LSP capabilities registered in the server capabilities?

### 5. KCS documentation

- Are relevant KCS notes created or updated to reflect the changes?
- Is `docs/kcs/README.md` updated if new notes were added?
- Does the README reflect new or changed features?

### 6. AI tooling and CLI

- Are AI skills (`ai/claude/skills/`) updated if the change affects
  skill-relevant behaviour (new diagnostics, commands, features)?
- Is `ai/claude/tcl_ai.py` updated if new analysis capabilities were added?
- Is the command registry (`core/commands/registry/`) updated if commands
  were added or changed?
- Are MCP tools updated if applicable?

### 7. Compiler explorer

- If the change affects bytecode, IR, or compilation: does the compiler
  explorer (`explorer/`) reflect it?
- Are new passes or optimisations visible in the explorer output?

### 8. Tests

- Are there tests covering the positive case (expected behaviour)?
- Are there tests covering the negative case (error handling, invalid input)?
- Are edge cases tested (empty input, boundary values, dialect differences)?
- Do tests follow the xfail policy (no shipping xfails)?
- For iRule changes: are Event Orchestrator tests included?

### 9. Tcl language completeness

Think through whether the solution handles these Tcl-specific concerns:

- **Namespaces**: qualified names, `namespace eval`, `namespace import/export`,
  `namespace path`, nested namespaces, `::` resolution
- **Aliases and rename**: `interp alias`, `rename`, effect on call tracking
- **unknown proc**: fallback resolution, `namespace unknown`, auto-loading
- **Shimmering**: string/list/dict representation changes, `K` combinator
  idiom, performance implications
- **Package loading**: `package require`, `package ifneeded`, `pkgIndex.tcl`,
  version resolution, `package forget`
- **Source and auto_index**: `source` path resolution, `auto_path`,
  `auto_mkindex`, lazy loading
- **Variable scoping**: `uplevel`, `upvar`, `variable`, `global`,
  `namespace upvar`, call stack depth implications
- **Ensemble commands**: `namespace ensemble`, sub-command dispatch,
  `-map` and `-unknown` options
- **Coroutines** (8.6+): `coroutine`, `yield`, `yieldto`, stack behaviour
- **OO** (8.6+): `oo::class`, mixins, filters, method resolution order
- **Zipfs** (9.0): embedded filesystem, `zipfs mount`

## Output format

### Summary

One-paragraph overview of the change quality.

### Findings

Group findings by review area. For each finding:

- **Severity**: pass / concern / action required
- **Location**: file path and line number(s)
- **Description**: what the issue is
- **Suggestion**: how to address it (if applicable)

### Verdict

Overall assessment: **approve**, **approve with suggestions**, or
**request changes**. Summarise the key items that need attention.

$ARGUMENTS
