// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `SpecTcl` — runtime spec packs, from a `.tclspec` file on disk to a
//! [`CommandSpec`](tcl_registry::spec::CommandSpec) in a live registry.
//!
//! The design is `docs/design/spec-packs.md`; the frozen DSL syntax is
//! `docs/design/spec-dsl-examples/README.md`. This crate is the runtime half
//! of it, in four layers that stack but do not depend upwards:
//!
//! | layer | module | question it answers |
//! |---|---|---|
//! | parse | [`loader`] | what does *one* `.tclspec` file declare? |
//! | discover | [`discovery`] | which files are this workspace's packs? |
//! | merge | [`pack`] | which files make up one pack, and who wins? |
//! | install | [`install`] | how does a pack reach the cached registry? |
//!
//! [`cache`] sits beside them: a disposable, hash-keyed compiled-pack cache in
//! the OS cache directory that makes a reload of an unchanged pack cheap.
//! [`bundled`] sits beside them as well: the **bundled** tier on its own, for
//! the consumers that have no workspace to discover — the `tcl` CLI, the MCP
//! server, a test harness — since the EDA vendor libraries are now loadables
//! and reach a registry only through the loader.
//! [`hooks`] sits beside them too: the seam that turns a pack's declared hook
//! *bodies* into live function pointers by way of the `tcl-spec-hooks` host —
//! it is here, and not in either neighbour, because this is the only crate
//! that owns both the loader's `HookDecl` and the host's `HookProgram`.
//!
//! ## Why this is not in `tcl-spec-studio`
//!
//! The loader was written in the studio, where the DSL was designed. It lives
//! here because **`tcl-registry` must not depend on the studio** and the LSP
//! server must not either: the server needs pack loading, and the studio is a
//! draft model, a `.rs` renderer, and a schema-coverage gate it has no use
//! for. The studio re-exports [`loader`] as `tcl_spec_studio::spectcl` and
//! [`catalogue`], so its own surface is unchanged — it is now a *consumer* of
//! this crate rather than its owner, which is also what lets the equivalence
//! gate in `tcl-spec-studio/tests/spectcl_ports.rs` keep testing exactly the
//! loader the server runs.
//!
//! ## Scope layering — packs are a *workspace* fact
//!
//! Packs are inserted into the per-profile cached `CommandRegistry` at
//! workspace scope, never into the per-document overlay path that
//! `.tcl.stubs` sidecars use. A pack is parsed per edit **of the pack**, never
//! per edit of the code that uses it, which is what makes a loaded pack cost
//! exactly what a compiled-in spec costs at query time
//! (`docs/design/spec-packs.md`, "Performance: the format does not decide
//! it").

pub mod bundled;
pub mod cache;
pub mod catalogue;
pub mod discovery;
pub mod hooks;
pub mod install;
pub mod loader;
pub mod pack;
pub mod upgrade;

pub use discovery::{DiscoveryOptions, PackFile, Tier, discover};
pub use install::registry_with_packs;
pub use loader::{
    AmbientPackage, ClauseGrammar, HookDecl, HookFamily, HookOwner, HookSource,
    KNOWN_VOCABULARY_VERSIONS, LoadError, NEWEST_VOCABULARY_VERSION, Notice, Pack, PackCommand,
    PackDialect, PackDialectAxis, PackEnvironment, PackEnvironmentTier, VocabularyClass, load_pack,
    roles_from_manufacturers, speclib_version_span,
};
pub use pack::{MergedPack, PackNotice, PackSet};
pub use upgrade::{
    OLDEST_VOCABULARY_VERSION, UpgradeOptions, UpgradeOutcome, UpgradeStatus, upgrade_source,
};

/// The `.tclspec` file extension, without the dot.
pub const PACK_EXTENSION: &str = "tclspec";

/// The **`SpecTcl` vocabulary version** — bumped when the meaning of a DSL
/// word changes, never when one is added (`docs/design/spec-packs.md`,
/// "Compatibility policy").
///
/// Part of the compiled-cache key, so a vocabulary bump invalidates every
/// cached pack exactly once.
///
/// `2` for `SpecTcl` 2.0. The redesign's §6.1 requires exactly one bump
/// here for 2.0: the legacy `dialects` word keeps loading forever, but its
/// *translation output* is now the same internal value the new `available`
/// word produces, so every cached pack must be rebuilt once against the
/// translating loader rather than served from a pre-2.0 entry.
pub const VOCABULARY_VERSION: &str = "2";
