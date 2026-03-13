# KCS: Project layout contracts

## Symptom

A change is hard to place because ownership boundaries between reusable language logic and LSP runtime code are unclear.

## Operational context

This repository is split into a reusable language core and an LSP runtime:

- `core/` contains parser/compiler/analysis/registry/domain logic.
- `lsp/` contains pygls server wiring, feature handlers, and workspace orchestration.
- Lifted shared modules (formatting, semantic graph, package resolver) live in `core/`.
- `vm/` and `explorer/` consume `core/` as downstream clients.

## Decision rules / contracts

1. Put reusable Tcl language behaviour in `core/` (no `lsp/` imports).
2. Put protocol/server lifecycle/feature wiring in `lsp/`.
3. Keep dependency direction one-way: `lsp/` -> `core/`; never `core/` -> `lsp/`.
4. New compiler/parsing/analysis passes must expose stable, reusable facts from `core/` for all consumers.
5. Editor- or transport-specific adaptation belongs in `lsp/features/`, not in `core/`.
6. When behaviour is lifted from `lsp/` to `core/`, update all downstream consumers (`explorer/`, `ai/`, `lsp/`, tests) to import the new `core` module directly.
7. Remove legacy module paths in the same change; do not leave compatibility wrappers behind.

## File-path anchors

- `core/parsing/`
- `core/compiler/`
- `core/analysis/`
- `core/commands/registry/`
- `core/packages/`
- `lsp/server.py`
- `lsp/features/`
- `lsp/workspace/`
- `core/formatting/`

## Failure modes

- Circular dependencies when `core/` imports `lsp/`.
- Duplicate logic when feature-specific behaviour is implemented in multiple `lsp/features/*` files instead of a shared `core` module.
- Regressions in VM/explorer behaviour when compiler logic is incorrectly added to `lsp/` only.

## Test anchors

- `tests/test_server_config.py`
- `tests/test_compilation_unit_parity.py`
- `tests/test_workspace_index.py`
- `tests/test_vm_basic.py`
- `tests/test_compiler_explorer.py`
- `tests/test_core_lift_consumers.py`

## Discoverability

- [KCS index](README.md)
- [Pipeline layering](kcs-pipeline-lsp-first.md)
- [Compiler KCS index](compiler/README.md)
