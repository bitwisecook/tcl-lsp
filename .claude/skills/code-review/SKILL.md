---
name: code-review
description: >
  Comprehensive code review of pending changes in the repository. Critiques
  quality, simplicity, correctness, dead code, the registry rule, LSP/editor
  integration completeness, KCS currency, AI/CLI tooling, compiler explorer
  visibility, test coverage, and Tcl dialect compatibility (8.4–9.1).
allowed-tools: Bash, Read, Glob, Grep
---

# Code Review

Review the current changes. Critique only; change nothing.

## Gather context

1. Ask what the change is for (problem, feature, refactor); review against that.
2. Scope: `git diff rust --stat` and `git diff rust --name-only` (`rust` is
   the active branch).
3. Read every changed file; for a large diff, `git diff rust -- <file>`.

## Checklist

Mark each area **pass**, **concern**, or **action required**; skip areas the
change does not touch.

1. **Quality and simplicity** — as simple as it reasonably can be; no
   premature abstraction, leftover shim, or compatibility layer; no dead,
   replaced, or unreachable code; names per `CONTRIBUTING.md` (UK spelling,
   explicit, no ambiguous single letters).
2. **Correctness** — sound logic, edge cases, off-by-one, bounds, silent
   failures. Tcl semantics: namespaces and qualified names, `interp alias` /
   `rename`, `unknown`, shimmering, `package` / `source` / `auto_index`,
   `uplevel` / `upvar`, `namespace eval`, `interp`, `trace`. Version-specific
   features gated per dialect (8.4–9.1).
3. **Registry rule** — no per-command `match` in a consumer; the fact lives on
   `CommandSpec` (`AGENTS.md`). A new command lands with its WASM backing.
4. **Performance** — obvious algorithmic wins taken; no needless allocation
   or repeated lookup on a hot path.
5. **LSP and editors** — a diagnostics / completion / hover / token /
   code-action / formatting change reaches every editor (VS Code, Neovim,
   Zed, Emacs, Helix, Sublime, JetBrains); `make codegen` re-run when a
   diagnostic or optimisation changed; new capabilities registered.
6. **Docs** — KCS note created or updated and indexed; README and the owning
   design doc current (`CONTRIBUTING.md` § *Documentation required for a
   PR*).
7. **AI and CLI surfaces** — `ai/claude/skills/`, the `.j2` prompt templates,
   MCP tools (`rust/tcl-mcp`), and the `tcl` / `f5-query` CLIs updated when a
   diagnostic, command, or feature they expose changed.
8. **Compiler explorer** — a bytecode / IR / pass change is visible in
   `tcl explore` (`rust/tcl-explorer`).
9. **Tests** — positive, negative, edge (empty, boundary, dialect); no
   shipped xfails; fuzz-shaped tests `#[ignore]`d; iRule changes carry Event
   Orchestrator tests.
10. **Tcl completeness** — think through: ensembles (`-map`, `-unknown`),
    aliases and `rename`, `unknown` and auto-loading, shimmering (the `K`
    idiom), package loading and version resolution, `source` / `auto_path`,
    scoping (`uplevel`, `upvar`, `variable`, `global`, `namespace upvar`),
    coroutines (8.6+), TclOO (mixins, filters, MRO), zipfs (9.0).

## Output

**Summary** — one paragraph. **Findings** — by area: severity, `file:line`,
description, suggestion. **Verdict** — approve / approve with suggestions /
request changes, naming the items that need attention.

$ARGUMENTS
