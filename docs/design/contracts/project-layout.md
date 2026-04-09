# KCS: Project layout contracts

## Symptom

A change is hard to place because ownership boundaries between reusable language logic and LSP runtime code are unclear.

## Operational context

This repository is split into a reusable language core and an LSP runtime,
plus a Rust workspace that is incrementally absorbing Python modules from
the bottom up:

- `core/` contains parser/compiler/analysis/registry/domain logic.
- `lsp/` contains pygls server wiring, feature handlers, and workspace orchestration.
- Lifted shared modules (formatting, semantic graph, package resolver) live in `core/`.
- `vm/` and `explorer/` consume `core/` as downstream clients.
- `rust/` is a two-crate workspace (`rust/tcl-lexer/` pure Rust and
  `rust/tcl-lsp-rust/` PyO3 bindings) — see
  [`../rust-rewrite.md`](../rust-rewrite.md) for the chunked migration
  plan and the "binding layer owns Python compatibility" rule.

## Decision rules / contracts

1. Put reusable Tcl language behaviour in `core/` (no `lsp/` imports).
2. Put protocol/server lifecycle/feature wiring in `lsp/`.
3. Keep dependency direction one-way: `lsp/` -> `core/`; never `core/` -> `lsp/`.
4. New compiler/parsing/analysis passes must expose stable, reusable facts from `core/` for all consumers.
5. Editor- or transport-specific adaptation belongs in `lsp/features/`, not in `core/`.
6. When behaviour is lifted from `lsp/` to `core/`, update all downstream consumers (`explorer/`, `ai/`, `lsp/`, tests) to import the new `core` module directly.
7. Remove legacy module paths in the same change; do not leave compatibility wrappers behind.
8. `core/*` Python modules may shim through to `tcl_lsp_rust` after a Rust port lands, but the reverse direction is forbidden: the pure Rust crates under `rust/tcl-lexer/` (and future siblings) never depend on `pyo3` and never import anything from `core/`, `lsp/`, or other Python packages.
9. `editors/zed/` is a standalone Rust cdylib and is **excluded** from the main Cargo workspace in the root `Cargo.toml`. Keep it that way.

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
- `Cargo.toml` (workspace manifest)
- `rust/tcl-lexer/`
- `rust/tcl-lsp-rust/`

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

- [Design docs index](../README.md)
- [Pipeline layering](pipeline-lsp-first.md)
- [Rust rewrite design doc](../../rust-rewrite.md)
