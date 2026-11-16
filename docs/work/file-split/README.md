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

## Status

| Step | State |
| --- | --- |
| Inline test modules of large files moved out (775 modules) | done, 16 November 2026 |
| Production files over the limit | 149 (252 before) |
| Test files over the limit | 80 |
| Hand-maintained data tables over the limit (`tcl-registry/src/bigip/data`) and generated files | 15, not started; generated files stay |
