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
- A test file is planned the same way: helpers stay in the parent, tests move to modules named for what they test, matching the production module names where there are any.
- A good plan follows the seams already in the file: section comments, a type with its impls, a family of functions with one caller. `semantic_tokens.toml` is the worked example.

Applying a plan: `apply_split.py PLAN.toml`, `cargo fmt`, fix what the compiler reports (private fields read across modules, imports only tests use), then `cargo clippy --fix` to narrow each child's `use super::*;`, then the verification above.

## Status

| Step | State |
| --- | --- |
| Inline test modules of large files moved out (775 modules) | done, 16 November 2026 |
| Production files over the limit | 149 (252 before) |
| Test files over the limit | 80 |
| Hand-maintained data tables over the limit (`tcl-registry/src/bigip/data`) and generated files | 15, not started; generated files stay |
