# Breaking up large Rust files

Goal: every Rust file that is not generated is at most 2,000 lines. Crates may be split or merged where that gives a cleaner design.

## Rules for every split

- A split is a move. No behaviour changes, no renamed public items, no logic edits. Every path that worked before (`crate::a::Item`) still works, through a re-export in the file that used to hold the item.
- A module is named for what it owns (`legend`, `receiver_dispatch`), never `part2`, `misc`, `helpers` or `utils`. Aim for 300 to 1,200 lines a file.
- `foo.rs` stays as the facade: module documentation, `mod` declarations, re-exports and the central type when it is small. The parts live in `foo/<topic>.rs`.
- A large `impl` block is split into several `impl` blocks by topic, one per file.
- Test modules live beside the code they test. A module declared `#[cfg(test)] mod name;` is test code to every gate (`rust/xtask/src/util/test_sources.rs`) and belongs to the file that declares it. A module that moves one directory deeper needs its `include_str!` paths adjusted.
- Visibility widens only as far as the split needs: `pub(super)` first.
- Verification for each split: `cargo check --workspace --all-targets` (about four minutes), `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings -A clippy::assert_is_empty`, `make xtask-gates`, and the crate's test list unchanged apart from module paths.

## Tools (`docs/work/file-split/tools`)

- `inventory.py` lists every file over the limit with its kind and how much of it is inline test modules.
- `hoist_inline_tests.py` moves top-level `#[cfg(test)] mod name { ... }` blocks of large files into their own files; `test_hoist_inline_tests.py` tests it (`uv run --with pytest python -m pytest -q docs/work/file-split/tools`).
- `outline.py FILE` prints each top-level item and impl method with its id, first line and length; plans refer to items by those ids.
- `apply_split.py PLAN.toml [--check]` moves the items a plan names into child modules; `test_apply_split.py` tests it. `--check` resolves the plan and prints the size of every resulting file, marking any still over the limit, without writing.
- `apply_plans.py PLANS_DIR` applies every plan under a directory, parents before the children they create, skipping what is already applied.
- `finish_split.sh PACKAGE CRATE_DIR`, `direct_imports.py` and `update_contract.py`: see *Applying the plans of one package* below.

## Plans (`docs/work/file-split/plans/<crate>/<path>.toml`)

One plan per file, named after the file (`rust/tcl-lsp-core/src/semantic_tokens.rs` has `plans/tcl-lsp-core/semantic_tokens.toml`; a nested file keeps its directories, `plans/tcl-compiler/analyser/handlers.toml`). A plan lists the child modules and the items each takes, by the ids `outline.py` prints. Whatever a plan does not name stays in the parent.

```toml
# One or two comment lines on what the facade keeps and why.
file = "rust/tcl-lsp-core/src/semantic_tokens.rs"

[[module]]
name = "legend"
doc = "The token kinds, the legend the server announces and the modifier bits."
items = ["enum:TokenKind..const:MOD_DECLARATION", "fn:legend_extra"]
```

- `a..b` takes every item from `a` to `b` in file order: both top-level, or both methods of one inherent impl (`impl:Walker::first..impl:Walker::last`). A whole impl block moves by its own id (`impl:fmt::Display for Walker`); a trait impl only moves whole.
- `use`, `mod` and macro items stay in the parent. Keep a type in the parent when its private fields are read in several modules: children see the parent's private items, the parent does not see theirs.
- `hoist = ["name"]` turns an inline `mod name { ... }` into `mod name;` with its own file before the modules are cut; an inline module can also be listed as an item and moves whole. A crate-root integration test (`tests/name.rs`) keeps its place: its modules go to `tests/name/` and are declared with `#[path]`, as `tcl-lsp-server/tests/e2e.rs` already does.
- A test file is planned the same way: helpers stay in the parent, tests move to modules named for what they test, matching the production module names where there are any.
- A good plan follows the seams already in the file: section comments, a type with its impls, a family of functions with one caller. `semantic_tokens.toml` is the worked example.

Applying the plans of one package:

1. `python3 docs/work/file-split/tools/apply_plans.py docs/work/file-split/plans/<crate>` and `cargo fmt`.
2. `docs/work/file-split/tools/finish_split.sh <package> <crate dir>`: narrows each child's `use super::*;`, points its imports at where each name is defined (`direct_imports.py`), drops what became unused, then checks and lints every target. It stops at the first failing step; what is left is what text cannot settle (a test naming an item that moved, a private field read across modules), and the compiler's `help:` lines name the fix. Fix and re-run until it prints `OK`.
3. `python3 docs/work/file-split/tools/update_contract.py`: the owner-resolution contract names the file and module of every owner entry point and route; this follows the declarations the split moved.
4. `cargo xtask file-sizes`, `cargo xtask registry-axes`, `cargo xtask value-transfers` to refresh the ledgers that list files, then `make xtask-gates`.

## What a split does to the gates

A gate that names files reads a module: `owner-resolution` checks written against a path read the file with the production text of its child modules; a `RATCHET` pin, a clean-file entry, a catalogue allowlist entry and a size-ledger entry each answer for the file's child modules (`util::listed_module_owner`). So a split edits no gate table. What does move is the owner-resolution contract's rows (file, module and visibility of each entry point and route), which `update_contract.py` rewrites from the tree.

## Status

Done, 16 November 2026: every hand-written Rust file under `rust/` and `runtime/rust/` is at most 2,000 lines, and `docs/generated/rust-file-sizes.md` is empty. 238 files were over the budget when the work began.

| Step | How |
| --- | --- |
| Inline test modules of large files moved to their own files (775 modules) | `hoist_inline_tests.py` |
| 287 plans, eleven groups, designed first | `plans/` |
| Plans applied crate by crate; imports settled from the compiler's diagnostics | `apply_plans.py`, `finish_split.sh`, `settle_imports.py` |
| The owner-resolution contract followed about 1,100 moved declarations | `update_contract.py`, then by hand where the gate ties a method to the file of its type |
| `tcl-lsp-server`'s `LanguageServer` impl (47 handlers) | each body is `Backend::serve_<request>` in its subsystem's `requests.rs` |
| `Vm::tick_instruction` (one 2,100-line match) | comparison and stack opcodes served by methods of their own |
| BIG-IP object-spec tables (nine files, up to 27,000 lines) | filed by tmsh word, `split_bigip_data.py`; the table's contents are byte-identical |
| mcpd service-name table | four key ranges |

Outside the budget, as for the licence-header check: generated files (first line `// @generated` or `// Generated from`) and recorded test data under `tests/data/` (five `software_controls.rs` row tables that two crates `include!`).

Kept in a facade because the owner-resolution contract ties them to the file that declares their type: `Lowerer::cmd_tokens`, `ModuleCommandBindings::analyse`, `Interp::synchronise_command_environment_guards` and `Interp::namespaces`.

Two sites carry `// dialect-drift-ok: unreviewed`: the dialect-drift gate stops reading a file at its first test-module line and had never seen them (`analyser/utils/stub_directives.rs`, `command_binding/module_bindings/namespace_transitions.rs`). They need the real grammar threaded, by the owning areas.

Crate boundaries the planners saw (none acted on): `tcl-registry/src/bigip` uses nothing else in its crate; `tcl-codegen/src/codegen/wasm`; the native tier of `tcl-compiler` (`executable_ir`, `dispatch_proof`, `world_state_ssa`, `common_aot_plan`, `native_lowering`, `native_byte_compilation`); the constant-folding core of `tcl_expr_eval`; the server's contention telemetry and its diagnostics engine; a shared home for the recorded `native_original_*` test data.
