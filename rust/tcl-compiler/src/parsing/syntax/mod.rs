//! The canonical red-green concrete syntax tree (CST).
//!
//! Ports `compiler/parsing/syntax/` from the Python source of truth
//! (landed on `main` by #533 / `SYNC-JUN06`).  The split mirrors
//! Roslyn / rust-analyzer:
//!
//! - [`green`] — the **position-independent** layer: a node knows only
//!   its *width* and its children, never an absolute offset.  Trivia is
//!   attached to the adjacent token, so a command is pure syntax while
//!   every inter-word byte still round-trips.
//! - [`red`] — overlays a green tree with an anchoring and resolves
//!   absolute positions lazily, reproducing the lexer's exact `Token`
//!   offsets / lines / columns.
//! - [`build`] — re-shapes the existing lexer stream into the tree (no
//!   second parser) via start-to-start tiling.
//! - `segment` (strip 5) — derives `SegmentedCommand` from the tree.
//! - `descend` (strip 6) — lazy descent into braced bodies / `[…]` subs.
//!
//! See `docs/design/compiler/syntax-tree.md`.

pub mod build;
pub mod green;
pub mod red;
