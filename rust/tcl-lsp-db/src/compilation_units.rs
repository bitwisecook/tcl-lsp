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

//! The shared compilation unit of a document, with the lexer policy and the registry it resolves under.

use super::declared_commands::declared_command_surface;
use super::document_analysis::{document_analyser, walked_document_input};
use super::function_lattices::FnLatticeKey;
use super::memoised_units::{build_unit_with_keys_and_input, memoised_compilation_unit};
use super::{AnalyserConfig, SourceFile, TclDb, overlay_epoch};
use std::sync::Arc;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::unit_scope::CallSiteEvidence;
use tcl_registry::CommandRegistry;
use tcl_registry::model::{DeclaredSurface, OverlayMiss};

/// Interned complete-document lexer policy for the resolved environment.
/// The environment owns every grammar axis and the file BOM boundary; strict
/// quoting and sub-lexing offsets retain their default entry-point values.
/// Supplied document builds compare this policy with their genuine complete
/// analysis input before issuing a compilation unit. Nested bodies receive
/// their separately retained child input and policy.
#[salsa::interned]
pub struct LexerCfgKey<'db> {
    /// The resolved environment id (`tcl8.6`, `f5-irules`, …) whose grammar
    /// this build lexes under.
    #[returns(ref)]
    pub environment: String,
}

impl LexerCfgKey<'_> {
    /// The full complete-document policy selected by this environment key.
    pub(super) fn to_config(self, db: &dyn TclDb) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::for_file_grammar(
            tcl_lsp_core::environment_for_dialect(self.environment(db)).grammar(),
        )
    }
}

/// Intern a [`LexerCfgKey`] for the environment `dialect` names.
///
/// The name is resolved through the one dialect-name ingress seam first, so
/// an alias (`irules`) and its canonical id (`f5-irules`) share one key and
/// one build rather than interning two.
pub(super) fn lexer_cfg_key<'db>(db: &'db dyn TclDb, dialect: &str) -> LexerCfgKey<'db> {
    LexerCfgKey::new(
        db,
        tcl_registry::model::ingress::resolve_environment(dialect)
            .id()
            .to_owned(),
    )
}

/// The registry a unit resolves against: the dialect's shared one, or its
/// generation carrying a workspace pack overlay.
pub(super) enum UnitRegistry {
    /// The dialect's process-wide registry ([`TclDb::registry`]).
    Shared(&'static CommandRegistry),
    /// The dialect's registry with the workspace's packs
    /// ([`TclDb::registry_with_overlay`]).
    Overlaid(Arc<CommandRegistry>),
}

impl std::ops::Deref for UnitRegistry {
    type Target = CommandRegistry;

    fn deref(&self) -> &CommandRegistry {
        match self {
            Self::Shared(registry) => registry,
            Self::Overlaid(registry) => registry,
        }
    }
}

/// The registry a unit for `dialect` under pack overlay `overlay` resolves
/// against: the shared one when there is no overlay (`0`), so a workspace
/// with no packs resolves exactly as before, and the overlaid generation
/// otherwise (`docs/design/compiler/value-transfers.md` § *One invocation,
/// one context*: a workspace pack's declarations reach the memoised
/// lattice).
///
/// # Errors
///
/// [`OverlayMiss`] when the overlay is non-zero and its packs are not
/// installed: the caller decides what it may do without them.
pub(super) fn unit_registry(
    db: &dyn TclDb,
    dialect: &str,
    overlay: u64,
) -> Result<UnitRegistry, OverlayMiss> {
    if overlay == 0 {
        return Ok(UnitRegistry::Shared(db.registry(dialect)));
    }
    // A tracked read: a moved overlay epoch asks the query again.
    let _epoch = overlay_epoch(db);
    db.registry_with_overlay(dialect, overlay)
        .map(UnitRegistry::Overlaid)
}

/// The exact shared registry retained by this lattice key's analysis snapshot.
pub(super) fn lattice_registry(db: &dyn TclDb, key: FnLatticeKey<'_>) -> Arc<CommandRegistry> {
    key.snapshot(db).registry.shared_registry()
}

/// A query that cannot be answered without packs nobody has installed
/// answers nothing. The answer depends on the overlay epoch, which
/// [`unit_registry`] read, so it is asked again when the host moves the epoch
/// after installing the packs.
pub(super) fn abstain<T>() -> Option<T> {
    None
}

/// The registry the token queries classify commands against: the overlaid
/// generation once the packs are installed, and the dialect's plain one until
/// then.
///
/// Highlighting advises and is asked for again once the packs arrive (a pack
/// reload has the client re-pull its tokens), so a window of pack commands
/// coloured as unknown ones costs nothing that outlives it; nothing here
/// builds a unit or offers a rewrite, which is why this is the one query
/// family that reads the plain registry for a miss. The miss is not
/// memoised past the overlay epoch that [`unit_registry`] read.
pub(super) fn token_registry(db: &dyn TclDb, dialect: &str, overlay: u64) -> UnitRegistry {
    unit_registry(db, dialect, overlay)
        .unwrap_or_else(|_| UnitRegistry::Shared(db.registry(dialect)))
}

/// The [`UnitBuildOptions`] every [`CompilationUnit`] built for `file` under
/// `cfg` shares — one place so the taint solve and the shared unit cannot
/// drift on the dialect, the cross-file view, or the document's own
/// declarations.
pub(super) fn unit_build_options<'a>(
    db: &dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'_>,
    registry: &'a CommandRegistry,
    external: Option<&'a CallSiteEvidence>,
    declared: &'a DeclaredSurface,
) -> UnitBuildOptions<'a> {
    UnitBuildOptions {
        registry,
        defer_top_level: false,
        config: cfg.to_config(db),
        dialect: tcl_lsp_core::optional_profile_for_dialect(file.dialect(db)),
        external_call_sites: external,
        declared_commands: Some(declared),
    }
}

/// The shared, memoised [`CompilationUnit`] for a document under a given lexer
/// config and workspace pack overlay (`0` for none) — built via
/// `memoised_compilation_unit` (per-procedure lattices on the salsa-native
/// [`function_lattice`] graph) against the overlay's registry, so a pack's
/// declarations reach every lattice of the file and a pack edit, which
/// changes the overlay, invalidates them.  Tracked + keyed on
/// `(file, cfg, overlay)` so the analyser tail ([`file_analysis_incremental`]) and the
/// compiler-checks pass share the same complete-document config key when
/// their supplied source inputs agree. Original full-input guards remain
/// independent of memo sharing.
///
/// **`None` is an abstention, not an error page.** A non-zero `overlay` names a
/// pack generation only the pack loader can build, so when nothing has
/// installed it for the document's dialect ([`TclDb::registry_with_overlay`]
/// answers an [`OverlayMiss`]) there is no unit: a unit built against the
/// plain registry would resolve every pack command as an unknown one and
/// would be memoised under the pack's key, and the rewrites the optimiser
/// and the checks derive from it could be wrong for the workspace. The miss
/// is recorded once for the host ([`take_overlay_misses`]), nothing is
/// published in its place, and the abstention reads the overlay epoch
/// ([`set_overlay_epoch`]), so it runs again — and so does everything that
/// read it — once the host installs the packs and moves the epoch.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn compilation_unit<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    overlay: u64,
) -> Option<Arc<CompilationUnit>> {
    let dialect = file.dialect(db).clone();
    let Ok(registry) = unit_registry(db, &dialect, overlay) else {
        return abstain();
    };
    let external = file.external_call_sites(db).clone();
    let declared = declared_command_surface(db, file, &registry);
    Some(Arc::new(memoised_compilation_unit(
        db,
        file.text(db),
        unit_build_options(db, file, cfg, &registry, external.as_deref(), &declared),
    )))
}

/// Memoised source compilation under the document's checked complete input.
/// Missing context or a stale requested grammar withholds the unit.
#[salsa::tracked(lru = 512, returns(clone))]
pub fn compilation_unit_for_config<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    config: AnalyserConfig,
) -> Option<Arc<CompilationUnit>> {
    let input = walked_document_input(db, file, config).ok()?;
    let registry = input.borrowed_context_registry().commands();
    let external = file.external_call_sites(db).clone();
    let declared = declared_command_surface(db, file, registry);
    let options = supplied_unit_build_options(
        db,
        file,
        cfg,
        registry,
        external.as_deref(),
        &declared,
        &input,
    )?;
    let settings = document_analyser(db, file, config).source_compilation_settings();
    Some(Arc::new(
        build_unit_with_keys_and_input(db, file.text(db), options, Some(&input), Some(&settings)).0,
    ))
}

pub(super) fn supplied_unit_build_options<'a>(
    db: &dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'_>,
    registry: &'a CommandRegistry,
    external: Option<&'a CallSiteEvidence>,
    declared: &'a DeclaredSurface,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> Option<UnitBuildOptions<'a>> {
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        input,
        cfg.to_config(db),
        Some(input.unit_profile()),
    )?;
    Some(UnitBuildOptions {
        dialect: Some(input.unit_profile()),
        config: input.lexer_config(),
        ..unit_build_options(db, file, cfg, registry, external, declared)
    })
}

/// The document's [`CompilationUnit`] under the default lexer config and no
/// pack overlay — a thin wrapper over [`compilation_unit`] that interns the
/// `LexerCfgKey` from the file's dialect, for the server-side accessors that
/// only have `(db, file)`. It shares the diagnostics path's memoised build
/// when the workspace has no packs; a caller holding an [`AnalyserConfig`]
/// uses [`document_compilation_unit_for`] to share it with packs too.
// LRU-capped: per-file key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 64, returns(clone))]
pub fn document_compilation_unit(db: &dyn TclDb, file: SourceFile) -> Arc<CompilationUnit> {
    let cfg_key = lexer_cfg_key(db, file.dialect(db));
    compilation_unit(db, file, cfg_key, 0).expect("a unit with no pack overlay always builds")
}

/// The document's [`CompilationUnit`] under the default lexer config and
/// `config`'s pack overlay: the diagnostics path's memoised build, which a
/// consumer resolving against the overlaid registry reads so the unit and
/// the registry agree on which commands the workspace's packs declare.
///
/// `None` when the packs are not installed ([`compilation_unit`]).
#[must_use]
pub fn document_compilation_unit_for(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<CompilationUnit>> {
    let cfg_key = lexer_cfg_key(db, file.dialect(db));
    compilation_unit_for_config(db, file, cfg_key, config)
}
