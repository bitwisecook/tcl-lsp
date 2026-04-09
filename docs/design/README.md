# Design documentation

This folder is the home for technical documentation about how tcl-lsp is
built: architecture, contracts, interfaces, data-structure references,
pipeline internals, and pass/fact ownership. Technical jargon and
specialist terms are allowed here — design docs describe how the system
is structured and why, and assume the reader can read the code.

If you are writing a user-facing answer, a how-to, a Q&A, or a feature
description, it belongs in [`docs/kcs/`](../kcs/README.md) instead. The
rules for the KCS/documentation split live in
[`AGENTS.md`](../../AGENTS.md) under "Knowledge base and documentation".

## Architecture and walkthroughs

- [compiler-architecture.md](compiler-architecture.md) — high-level map of
  the multi-pass compiler pipeline with diagrams and cross-links.
- [example-script-walkthroughs.md](example-script-walkthroughs.md) — full
  pipeline traces for progressively complex Tcl scripts.
- [code-importing-examples.md](code-importing-examples.md) — reference
  patterns for Tcl code importing (package require, sourcing).

## Planning documents

- [kcs-completeness-plan.md](kcs-completeness-plan.md) — the phased plan
  to bring the knowledge base to 100% coverage of diagnostic,
  warning, and optimisation codes, with compiler-pass tagging and
  strong cross-linking between KCS pages, the glossary, and the
  design docs. Tracks scope, naming, templates, and the quality bar.

## Compiler internals

See [compiler/README.md](compiler/README.md) for the compiler design-doc
index — pipeline stages, analyses, codegen, optimisation passes, and
ownership matrices.

## Contracts and interfaces

See [contracts/](contracts/) for focused notes on module contracts and
cross-module interfaces. Each file answers "who owns this surface, what
are its rules, and what are the failure modes". One contract per file.

- [command-registry-event-model.md](contracts/command-registry-event-model.md)
  — command and event registry ownership rules.
- [core-lsp-shared-utility.md](contracts/core-lsp-shared-utility.md) —
  shared helper ownership across core and LSP.
- [formatter-engine.md](contracts/formatter-engine.md) — formatter
  idempotency and rewrite contracts.
- [project-layout.md](contracts/project-layout.md) — repository layout
  and dependency direction.
- [lsp-feature-providers.md](contracts/lsp-feature-providers.md) —
  non-diagnostics LSP provider contracts and failure modes.
- [workspace-indexing.md](contracts/workspace-indexing.md) — workspace
  cache, index, and scanner contracts.
- [package-loading.md](contracts/package-loading.md) — stdlib, tcllib,
  Tk, and iRules cross-file package loading.
- [parsing.md](contracts/parsing.md) — segmentation and recovery
  contracts.
- [lexing.md](contracts/lexing.md) — token and range fidelity rules.
- [lsp-diagnostics-publication.md](contracts/lsp-diagnostics-publication.md)
  — LSP diagnostics publication and suppression model.
- [vm-bytecode-test-boundary.md](contracts/vm-bytecode-test-boundary.md)
  — VM and bytecode identity and fixture boundary guidance.
- [vscode-extension.md](contracts/vscode-extension.md) — VS Code
  extension integration contracts.
- [differential-fuzzing.md](contracts/differential-fuzzing.md) —
  differential fuzzing oracle and coverage-guided mutation contracts.
- [pipeline-lsp-first.md](contracts/pipeline-lsp-first.md) — pipeline
  layering for LSP use.
- [command-alias-resolution.md](contracts/command-alias-resolution.md)
  — `interp alias` resolution and argument role inheritance.
- [docstring-handling.md](contracts/docstring-handling.md) — proc
  docstring extraction, parsing, and formatting.
- [dialect-stubs.md](contracts/dialect-stubs.md) — dialect command stubs
  and inline stub blocks.
- [proc-arg-traits.md](contracts/proc-arg-traits.md) — proc argument
  trait inference.
- [variable-case-mismatch-suggestions.md](contracts/variable-case-mismatch-suggestions.md)
  — case-mismatch suggestion diagnostics.
- [shimmer-reference-behaviour.md](contracts/shimmer-reference-behaviour.md)
  — shimmer expectations and validation strategy.
- [tcloo-implementation.md](contracts/tcloo-implementation.md) — TclOO
  class hierarchy, VM runtime, and MRO.
- [irule-test-framework.md](contracts/irule-test-framework.md) — iRule
  event orchestrator and TMM simulation.
- [namespace-model.md](contracts/namespace-model.md) — unified
  namespace model across dialects.
- [irule4005-racy-static-cross-event.md](contracts/irule4005-racy-static-cross-event.md)
  — IRULE4005 racy `static::` cross-event contract.
- [dialect-detection.md](contracts/dialect-detection.md) — dialect
  detection priority chain.
- [xdg-config.md](contracts/xdg-config.md) — XDG configuration file
  format reference.

## Templates

Templates for new design docs live at
[templates/](templates/):

- [template-contract.md](templates/template-contract.md) — ownership,
  contracts, and integration boundaries.
- [template-reference.md](templates/template-reference.md) — compact
  reference or decision pages.
- [template-matrix.md](templates/template-matrix.md) — producer/consumer
  ownership matrices.
