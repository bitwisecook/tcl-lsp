//! Parsing frontend: the concrete syntax tree and its derivations.
//!
//! Ports `compiler/parsing/` from the Python source of truth.  The
//! canonical, lossless **red-green concrete syntax tree** (CST) lives
//! under [`syntax`] — the single representation the segmenter, lowering,
//! formatter, minifier, and tooling are meant to share.  See
//! `docs/design/compiler/syntax-tree.md` and the `CST-PORT` row of
//! `docs/rust-rewrite.md`.

pub mod syntax;
