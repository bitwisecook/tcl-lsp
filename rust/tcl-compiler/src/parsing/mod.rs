//! Parsing frontend: the concrete syntax tree and its derivations.
//!
//! The
//! canonical, lossless **red-green concrete syntax tree** (CST) lives
//! under [`syntax`] — the single representation the segmenter, lowering,
//! formatter, minifier, and tooling are meant to share.  See
//! `docs/design/compiler/syntax-tree.md`.

pub mod syntax;
