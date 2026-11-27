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

//! Taint analysis — data-flow from tainted sources to dangerous
//! sinks, tracked through a multi-colour lattice.
//!
//! This module provides:
//!
//! 1. **`TaintColour`** / **`TaintLattice`** — the colour lattice.
//! 2. **`propagate_taints`** — intra-procedural worklist that seeds
//!    taint from known source commands (`gets`, `read`, `exec`,
//!    `chan`, `encoding convertfrom`) and propagates through SSA phi
//!    nodes and variable copies. Optionally consumes a
//!    `rendered_props` map to enrich each lattice with colours
//!    derived from string content (`STARTS_WITH_SLASH` →
//!    `PATH_PREFIXED`, absence of `STARTS_WITH_DASH` →
//!    `NON_DASH_PREFIXED`), and an
//!    [`InterproceduralAnalysis`]
//!    to transfer taint across proc boundaries via passthrough
//!    parameters.
//! 3. **`find_taint_warnings`** — sink check: emits **T100** when a
//!    tainted value reaches a code-execution sink (`eval`, `exec`,
//!    `uplevel`, `subst`, `expr`) and **T101** when it reaches an
//!    output sink (`puts`).
//!
//! ## What is implemented
//!
//! - **T104 SSRF / T105 cross-interpreter injection** — registry-driven
//!   via `taint_network_sink_args` (`socket`) and
//!   `taint_interp_eval_subcommands` (`interp eval` / `invokehidden`);
//!   see [`classify_network_interp_sinks`].  Suppressed by a validated
//!   address colour (T104) or a proved literal command word in the constructed
//!   script list (T105).
//! - **T106 double-encoding** — [`transform_colour`] stamps a command's
//!   `taint_transform` colour onto its tainted result during
//!   propagation, and [`emit_double_encode_warnings`] flags a value that
//!   re-enters a command whose `taint_double_encode_colour` it already
//!   carries (e.g. `uri::encode [uri::encode $x]`).
//! - **W313 destructive-file-on-tainted-path** —
//!   [`find_destructive_file_warnings`] flags `file delete` / `rename` /
//!   `mkdir` (registry `destructive` subcommands) with a variable path,
//!   suppressed when the path is normalised *and* bounds-checked (the
//!   `[string match …]` branch-guard analysis in
//!   [`compute_branch_guard_map`]).
//!
//! - Path-concat heuristic (W201) — see [`crate::path_concat`].
//! - iRules-specific sink codes IRULE3001–3004 / 3101 / 3102 —
//!   dispatched from [`classify_sink`] / [`find_taint_warnings`] /
//!   [`find_setter_constraint_warnings`] when the dialect is
//!   `f5-irules` / `irules`. IRULE3102 (unnormalised getter) lives
//!   in the sibling [`crate::irules_checks`] module.
//! - **URI-split / IRULE3103** (`*::uri` getter + manual
//!   decomposition) — see [`crate::uri_split`].
//! - **Positioned command identity** — retained source bindings and the shared
//!   registry invocation adapters choose source/sink/sanitiser semantics,
//!   including alias prefixes and renames. Explicit uncertainty cannot recover
//!   implementation guarantees from a matching written name.
//! - **Namespace-scoped proc shadowing a builtin** — a call whose bare or
//!   canonical name is shadowed, from its own namespace, by a user `proc` of
//!   the same name is not misclassified as the builtin sink/source; see
//!   [`shadowed_builtin_names_for_module_function`].
//! - **Variable-trace-altered taint** — `trace add variable` / `trace
//!   variable` conservatively forces a traced name to fully tainted
//!   everywhere in the module, since a runtime handler can rewrite the
//!   value in ways static analysis can't see; see
//!   [`apply_module_variable_traces`].
//!
//! ## Source / sink / sanitiser facts live in the registry
//!
//! The source / sanitiser tables live in
//! [`tcl_registry::taint`], which this module queries rather than keeping
//! tables of its own:
//!
//! * `tcl_registry::taint::is_taint_source` covers the trait-driven
//!   sources (`gets`, `read`, `exec`, `socket`), the subcommand
//!   sources (`chan gets`, `chan read`, `encoding convertfrom`),
//!   the iRules `UNNORMALISED_HTTP_GETTER` trait, and the iRules
//!   namespace-prefix fallback.
//! * `tcl_registry::taint::is_sanitiser` covers fixed-numeric-return
//!   sanitisers (e.g. `string length`, `string is integer`).
//!
//! Sink classification itself is registry-driven too: [`classify_sink`]
//! asks [`tcl_registry::taint::classify_taint_sinks`] (which reads
//! [`Traits::TAINT_SINK`] / [`Traits::EVALUATES_CODE`] plus each spec's
//! declared output/log sink code) rather than matching command names.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use tcl_core_types::{DiagCode, DiagFamily};

use bitflags::bitflags;
use rustc_hash::FxHashSet;

use tcl_lexer::Span;
#[cfg(test)]
use tcl_lexer::backslash_subst_in;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::CommandTokens;
use crate::ir::Statement;
use crate::irules_checks::CodeFix;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use crate::value_shapes::{
    parse_command_substitution_with_config, parse_command_substitution_with_spans_and_config,
};
use tcl_dialect::model::SurfaceQuery;
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

// Colour lattice

bitflags! {
    /// A taint colour — each bit records one safety property or
    /// origin fact about a value.
    ///
    /// A value is "clean" when `TAINTED` is unset; otherwise one
    /// or more mitigating colours may prove it safe for specific
    /// sinks (see `T102_SAFE`, `CRLF_FREE`, `SHELL_ATOM`, …).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TaintColour: u32 {
        /// Value is known tainted (attacker-influenced).
        const TAINTED            = 1 << 0;
        /// Value has an absolute-path prefix (starts with `/`).
        const PATH_PREFIXED      = 1 << 1;
        /// Value cannot start with `-` (option-injection safe).
        const NON_DASH_PREFIXED  = 1 << 2;
        /// Value contains no CR / LF characters.
        const CRLF_FREE          = 1 << 3;
        /// Value is a shell atom (no unquoted whitespace).
        const SHELL_ATOM         = 1 << 4;
        /// Value is a canonical Tcl list with known structure.
        const LIST_CANONICAL     = 1 << 5;
        /// Value is a literal regex pattern.
        const REGEX_LITERAL      = 1 << 6;
        /// Value is a fully-normalised filesystem path.
        const PATH_NORMALISED    = 1 << 7;
        /// Value is bounded inside a known safe directory.
        const PATH_BOUNDED       = 1 << 8;
        /// Value is a header token (RFC 7230 tchar set).
        const HEADER_TOKEN_SAFE  = 1 << 9;
        /// Value has been HTML-escaped.
        const HTML_ESCAPED       = 1 << 10;
        /// Value has been URL-encoded.
        const URL_ENCODED        = 1 << 11;
        /// Value is a literal IP address.
        const IP_ADDRESS         = 1 << 12;
        /// Value is a literal TCP/UDP port number.
        const PORT               = 1 << 13;
        /// Value is a fully-qualified domain name.
        const FQDN               = 1 << 14;
        /// Value was assembled via `[file join]`.
        const PATH_JOINED        = 1 << 15;
        /// Value is an I/O channel handle.
        const CHANNEL            = 1 << 16;
    }
}

impl TaintColour {
    /// Every colour bit set. Used as the "definitely clean" mask
    /// in set-union lattices where adding a colour can only
    /// sharpen what we know.
    pub const ALL: Self = Self::from_bits_truncate(
        Self::TAINTED.bits()
            | Self::PATH_PREFIXED.bits()
            | Self::NON_DASH_PREFIXED.bits()
            | Self::CRLF_FREE.bits()
            | Self::SHELL_ATOM.bits()
            | Self::LIST_CANONICAL.bits()
            | Self::REGEX_LITERAL.bits()
            | Self::PATH_NORMALISED.bits()
            | Self::PATH_BOUNDED.bits()
            | Self::HEADER_TOKEN_SAFE.bits()
            | Self::HTML_ESCAPED.bits()
            | Self::URL_ENCODED.bits()
            | Self::IP_ADDRESS.bits()
            | Self::PORT.bits()
            | Self::FQDN.bits()
            | Self::PATH_JOINED.bits()
            | Self::CHANNEL.bits(),
    );

    /// Colours that prove a value cannot start with `-` and so
    /// is safe against option-injection sinks (T102).
    pub const T102_SAFE: Self = Self::from_bits_truncate(
        Self::PATH_PREFIXED.bits()
            | Self::NON_DASH_PREFIXED.bits()
            | Self::IP_ADDRESS.bits()
            | Self::PORT.bits()
            | Self::FQDN.bits(),
    );

    /// Colours that mitigate CRLF / header / log injection in the
    /// *value position* of an iRules header or log sink (IRULE3002 /
    /// IRULE3003).
    ///
    /// The CRLF-safe mask is `CRLF_FREE | IP_ADDRESS | PORT |
    /// FQDN`. `HEADER_TOKEN_SAFE` is deliberately **not** included —
    /// it only suppresses IRULE3002 in the header/cookie *name*
    /// position and is handled by the call-site-aware
    /// `irule3002_name_position_safe` helper. `HTML_ESCAPED` and
    /// `URL_ENCODED` are also excluded: both can still carry raw
    /// CR/LF octets (HTML-escape rewrites `<`/`>`/`&`; URL-encode
    /// rewrites `%`), so neither proves header-injection safety.
    pub const CRLF_SAFE: Self = Self::from_bits_truncate(
        Self::CRLF_FREE.bits() | Self::IP_ADDRESS.bits() | Self::PORT.bits() | Self::FQDN.bits(),
    );

    /// Colours that prove a redirect target is same-origin and so safe
    /// against open-redirect (IRULE3004). A value starting with `/` or
    /// one that has been through `[file normalize]` routes back to the
    /// current host.
    pub const REDIRECT_SAFE: Self =
        Self::from_bits_truncate(Self::PATH_PREFIXED.bits() | Self::PATH_NORMALISED.bits());
}

/// Provenance carried separately from Registry-owned mitigation colours.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaintProvenance {
    /// No tainted contributor.
    None,
    /// Taint exists only because of a conservative global-writer seed.
    ConservativeGlobalSeedOnly,
    /// At least one tainted contributor has an independent source.
    OtherTaint,
}

/// Per-SSA-value taint lattice element: Registry-owned colours plus taint
/// provenance tracking whether any incoming path may be tainted.
///
/// `colours` is the "must-have" intersection at joins — a colour
/// survives only when every incoming edge has it. Taint is a
/// "may-have" — once any path sets it, it sticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaintLattice {
    /// Bag of Registry-owned colours. `TAINTED` means "may be tainted".
    pub colours: TaintColour,
    /// Internal seed provenance, separate from the Registry colour domain.
    #[doc(hidden)]
    pub provenance: TaintProvenance,
}

impl TaintLattice {
    /// Fresh clean value — no taint, no mitigations proven.
    #[must_use]
    pub const fn clean() -> Self {
        Self {
            colours: TaintColour::empty(),
            provenance: TaintProvenance::None,
        }
    }

    /// Construct from Registry-owned colours with ordinary taint provenance.
    #[must_use]
    pub const fn from_colours(colours: TaintColour) -> Self {
        Self {
            colours,
            provenance: if colours.contains(TaintColour::TAINTED) {
                TaintProvenance::OtherTaint
            } else {
                TaintProvenance::None
            },
        }
    }

    /// Fully tainted with no mitigations.
    #[must_use]
    pub const fn tainted() -> Self {
        Self {
            colours: TaintColour::TAINTED,
            provenance: TaintProvenance::OtherTaint,
        }
    }

    /// Taint used only for the conservative cross-procedure global-write seed.
    #[must_use]
    pub const fn conservative_global_seed() -> Self {
        Self {
            colours: TaintColour::TAINTED,
            provenance: TaintProvenance::ConservativeGlobalSeedOnly,
        }
    }

    /// Whether every tainted contributor is still only a conservative global seed.
    #[must_use]
    pub const fn is_conservative_global_seed_only(self) -> bool {
        self.is_tainted() && matches!(self.provenance, TaintProvenance::ConservativeGlobalSeedOnly)
    }

    /// True when the value is known tainted.
    #[must_use]
    pub const fn is_tainted(self) -> bool {
        self.colours.contains(TaintColour::TAINTED)
    }

    /// Intersect mitigating colours (must-have) among the *tainted*
    /// contributors, union taint bits (may-have). This implements the
    /// standard lattice join for taint analysis.
    ///
    /// A clean (untainted) operand is the join **identity**: it contributes no
    /// taint, so it must not dilute the other operand's mitigation colours.
    /// Treating it as an annihilator (intersecting its empty colour set) wrongly
    /// strips proven-safe colours — e.g. `clean.join(tainted|PATH_PREFIXED)`
    /// would drop `PATH_PREFIXED` and re-fire T102. Mitigations are "must-have"
    /// only across operands that actually carry taint.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        if !self.is_tainted() {
            return other;
        }
        if !other.is_tainted() {
            return self;
        }
        let taint = (self.colours | other.colours) & TaintColour::TAINTED;
        let mitigations = (self.colours & other.colours) & !TaintColour::TAINTED;
        let provenance = if self.is_conservative_global_seed_only()
            && other.is_conservative_global_seed_only()
        {
            TaintProvenance::ConservativeGlobalSeedOnly
        } else {
            TaintProvenance::OtherTaint
        };
        Self {
            colours: taint | mitigations,
            provenance,
        }
    }

    /// Add a colour (typically a mitigation).
    #[must_use]
    pub fn with(self, c: TaintColour) -> Self {
        Self {
            colours: self.colours | c,
            provenance: if c.contains(TaintColour::TAINTED) && !self.is_tainted() {
                TaintProvenance::OtherTaint
            } else {
                self.provenance
            },
        }
    }

    /// Remove `TAINTED` — used by sanitisers.
    #[must_use]
    pub fn sanitised(self) -> Self {
        Self {
            colours: self.colours & !TaintColour::TAINTED,
            provenance: TaintProvenance::None,
        }
    }

    /// Drop every proof about the value, keeping only that it is tainted —
    /// used when it passes through a command with no registry classification
    /// (not a recognised source, sanitiser, transform, or passthrough).
    ///
    /// Every colour but `TAINTED` is a claim about the value's contents, and
    /// an unclassified command can invalidate any of them: `string range` can
    /// cut the leading `/` a `PATH_PREFIXED` proof rests on, and a `string
    /// map` that inserts a newline destroys `CRLF_FREE` just as surely. The
    /// taint itself must survive; the proofs must not. A command that really
    /// does preserve a proof earns a registry classification, whose
    /// `taint_transform` colour is re-applied on top of this.
    #[must_use]
    pub fn shape_unproven(self) -> Self {
        Self {
            colours: self.colours & TaintColour::TAINTED,
            provenance: self.provenance,
        }
    }
}

// Diagnostic type

/// Tainted data flowing into a dangerous sink.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaintWarning {
    /// Span of the sink use.
    pub span: Span,
    /// Variable name carrying the taint.
    pub variable: String,
    /// Command that acted as the sink.
    pub sink_command: String,
    /// Original source invocation selected by this warning, when independently retained.
    pub source_sink: Option<TaintSinkSource>,
    /// Diagnostic code (`"T100"` family).
    pub code: DiagCode,
    /// Formatted message.
    pub message: String,
    /// Optional replacement text for a code-action fix. Currently
    /// always `None` for taint diagnostics; see `fixes` for the
    /// insertion-style code actions taint diagnostics actually use.
    pub replacement: Option<String>,
    /// Suggested fixes whose range is independent of `span` (insertions /
    /// out-of-span replacements) — see `irules_checks::CodeFix`. Empty
    /// when the diagnostic has no such fix.
    pub fixes: Vec<CodeFix>,
}

/// Why a required source-only nested replay could not be analysed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableTaintReplayReason {
    /// The original source owner is missing or no longer applicable.
    SourceMetadata,
    /// The selected procedure constructor cannot support source replay.
    ProcedureConstructor,
    /// The selected alias constructor cannot support source replay.
    AliasConstructor,
}

impl UnavailableTaintReplayReason {
    #[must_use]
    /// Stable reason label for reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceMetadata => "source-metadata",
            Self::ProcedureConstructor => "procedure-constructor",
            Self::AliasConstructor => "alias-constructor",
        }
    }
}

/// Original source correspondence, independently of execution permission.
#[derive(Debug, Clone, PartialEq)]
pub struct TaintReplaySourceSnapshot {
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    input: Option<crate::analyser::ResolvedAnalysisInput>,
    entry: crate::command_binding::SourceAnalysisEntry,
    owner: Option<std::sync::Arc<crate::command_binding::RetainedSourceModuleBindings>>,
    registry: Option<tcl_registry::RegistrySnapshot>,
}

impl TaintReplaySourceSnapshot {
    fn for_module(module: &crate::ir::Module) -> Self {
        Self {
            image: module.source.clone(),
            config: module.lexer_config,
            input: module.source_metadata_input.clone(),
            entry: module.source_entry.clone(),
            owner: module.retained_source_bindings.clone(),
            registry: module.registry_snapshot.clone(),
        }
    }

    #[must_use]
    /// Check the complete current source entry and retained Registry owner.
    pub fn matches_module(&self, module: &crate::ir::Module, registry: &CommandRegistry) -> bool {
        self == &Self::for_module(module)
            && self
                .owner
                .as_ref()
                .is_some_and(|owner| owner.matches_module(module, registry))
            && self.registry.as_ref().is_some_and(|captured| {
                captured.semantic_key() == registry.snapshot().semantic_key()
            })
    }
}

/// A required replay whose original source site remains unavailable.
#[derive(Debug, Clone, PartialEq)]
pub struct UnavailableTaintReplay {
    /// Replay span; original coordinates require a matching source snapshot.
    pub span: Span,
    /// Missing prerequisite for this replay.
    pub reason: UnavailableTaintReplayReason,
    /// Original source correspondence when it was retained.
    pub source: Option<TaintReplaySourceSnapshot>,
}

/// Availability of replays in the requested scope, separate from warning count.
/// Per-function reports cover command heads; whole-unit reports also cover stored callbacks.
#[derive(Debug, Clone, PartialEq)]
pub enum TaintReplayAnalysis {
    /// The requested scope could not be analysed.
    Unavailable {
        /// Known replay sites missing their prerequisites.
        unavailable: Vec<UnavailableTaintReplay>,
    },
    /// The requested scope was analysed, possibly with unavailable nested sites.
    Analysed {
        /// Nested replay sites missing their prerequisites.
        unavailable: Vec<UnavailableTaintReplay>,
    },
}

impl TaintReplayAnalysis {
    #[must_use]
    /// Whether the requested scope and every required nested replay were analysed.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Analysed { unavailable } if unavailable.is_empty())
    }
    #[must_use]
    /// Represent an unavailable scope with no known source sites.
    pub const fn unavailable() -> Self {
        Self::Unavailable {
            unavailable: Vec::new(),
        }
    }

    #[must_use]
    /// Retained sites whose replay prerequisites are unavailable.
    pub fn unavailable_replays(&self) -> &[UnavailableTaintReplay] {
        match self {
            Self::Unavailable { unavailable } | Self::Analysed { unavailable } => unavailable,
        }
    }

    /// Combine scopes while retaining every unavailable site.
    pub fn merge(&mut self, other: Self) {
        let was_available =
            matches!(self, Self::Analysed { .. }) && matches!(other, Self::Analysed { .. });
        let mut retained = self.unavailable_replays().to_vec();
        let (Self::Unavailable { unavailable } | Self::Analysed { unavailable }) = other;
        for replay in unavailable {
            if !retained.contains(&replay) {
                retained.push(replay);
            }
        }
        *self = if was_available {
            Self::Analysed {
                unavailable: retained,
            }
        } else {
            Self::Unavailable {
                unavailable: retained,
            }
        };
    }
}

/// Proven warnings plus the availability of every required nested replay.
#[derive(Debug, Clone, PartialEq)]
pub struct TaintWarningReport {
    /// Warnings established by the completed analyses.
    pub warnings: Vec<TaintWarning>,
    /// Availability of replays in this report's scope.
    pub replays: TaintReplayAnalysis,
}

impl TaintWarningReport {
    fn analysed(warnings: Vec<TaintWarning>, unavailable: Vec<UnavailableTaintReplay>) -> Self {
        Self {
            warnings,
            replays: TaintReplayAnalysis::Analysed { unavailable },
        }
    }
    fn merge(&mut self, other: Self) {
        self.warnings.extend(other.warnings);
        self.replays.merge(other.replays);
    }
}

/// Build the local receiver → class facts needed by object-method taint
/// dispatch.
///
/// This is the same generic registry contract as the object-type pass, kept
/// local to a function so the taint lattice does not inherit the object pass's
/// intentionally scope-blind widening.  It recognises a literal naming
/// factory (`ttk::entry .e`) and an assigned factory result (`set w
/// [ttk::entry .e]`).  Conflicting bindings are retained as a set and the
/// consumer above abstains unless that set is a singleton.
pub(crate) type InstanceClassState = HashMap<String, HashSet<String>>;

/// Receiver classes reaching each statement entry, keyed by source offset.
///
/// Keeping the state at the program point (rather than an append-only history)
/// is load-bearing: a later `set w [label .l]` kills an earlier entry binding,
/// while two different assignments reaching a branch join widen to both
/// classes and make [`unique_instance_class`] abstain.
#[derive(Debug, Clone, Default)]
pub(crate) struct LocalInstanceClasses {
    at: HashMap<u32, InstanceClassState>,
}

#[cfg(test)]
pub(crate) fn local_instance_classes(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
) -> LocalInstanceClasses {
    local_instance_classes_with_source(
        cfg,
        registry,
        &InstanceClassState::new(),
        TaintSourceContext::standalone(registry, registry.profile()),
    )
}

mod instance_classes;
pub(crate) use instance_classes::transfer_instance_command;
use instance_classes::transfer_instance_statement;

fn join_instance_predecessors<'a>(
    states: impl Iterator<Item = &'a InstanceClassState>,
) -> Option<InstanceClassState> {
    let mut states = states;
    let mut joined = states.next()?.clone();
    for state in states {
        // A type is known after a join only when every incoming path binds the
        // receiver.  Classes from those paths are unioned; consumers abstain
        // unless the resulting set is a singleton.
        joined.retain(|receiver, classes| {
            let Some(incoming) = state.get(receiver) else {
                return false;
            };
            classes.extend(incoming.iter().cloned());
            true
        });
    }
    Some(joined)
}

fn local_instance_classes_with_source(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    initial: &InstanceClassState,
    source: TaintSourceContext<'_>,
) -> LocalInstanceClasses {
    if !source.allows_nominal_metadata() && source.metadata_context().is_none() {
        return LocalInstanceClasses::default();
    }
    let predecessors = cfg.predecessors();
    let mut outputs: HashMap<crate::cfg::BlockId, InstanceClassState> = HashMap::new();
    let mut changed = true;
    while changed {
        changed = false;
        for (&block_id, block) in &cfg.blocks {
            let mut state = if block_id == cfg.entry {
                initial.clone()
            } else {
                let Some(joined) = join_instance_predecessors(
                    predecessors
                        .get(&block_id)
                        .into_iter()
                        .flatten()
                        .filter_map(|pred| outputs.get(pred)),
                ) else {
                    continue;
                };
                joined
            };
            for statement in &block.statements {
                let tokens = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                    &cfg.command_binding_sites,
                    statement,
                );
                transfer_instance_statement(&mut state, statement, registry, source, tokens);
            }
            if outputs.get(&block_id) != Some(&state) {
                outputs.insert(block_id, state);
                changed = true;
            }
        }
    }

    let mut classes = LocalInstanceClasses::default();
    for (&block_id, block) in &cfg.blocks {
        let mut state = if block_id == cfg.entry {
            initial.clone()
        } else {
            join_instance_predecessors(
                predecessors
                    .get(&block_id)
                    .into_iter()
                    .flatten()
                    .filter_map(|pred| outputs.get(pred)),
            )
            .unwrap_or_default()
        };
        for statement in &block.statements {
            // Several statements share one command's span; a source lookup
            // at that command sees the state before its first statement.
            classes
                .at
                .entry(statement.span().start())
                .or_insert_with(|| state.clone());
            let tokens = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                &cfg.command_binding_sites,
                statement,
            );
            transfer_instance_statement(&mut state, statement, registry, source, tokens);
        }
        if let Some(span) = block
            .terminator
            .as_ref()
            .and_then(crate::cfg::Terminator::span)
        {
            // Return values and branch conditions are evaluated after every
            // statement in the block. Interprocedural return-summary inference
            // queries receiver facts at the terminator span, so it must see
            // the reaching state rather than an absent/exact-offset miss.
            classes.at.insert(span.start(), state);
        }
    }
    classes
}

#[cfg(test)]
std::thread_local! {
    static INSTANCE_CLASS_SOLVES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_instance_class_solve_count() {
    INSTANCE_CLASS_SOLVES.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn instance_class_solve_count() -> usize {
    INSTANCE_CLASS_SOLVES.with(std::cell::Cell::get)
}

/// Local conditional receiver candidates plus source setup advice from the
/// same complete Module interpretation. The top-level function keeps only
/// source-ordered local candidates; none proves an existing Native object.
pub(crate) fn instance_classes_for_function(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    interproc: Option<&InterproceduralAnalysis>,
    include_globals: bool,
    source: TaintSourceContext<'_>,
) -> LocalInstanceClasses {
    #[cfg(test)]
    INSTANCE_CLASS_SOLVES.with(|count| count.set(count.get() + 1));
    let initial = if include_globals {
        interproc
            .filter(|analysis| {
                let Some(owner) = source.module_owner() else {
                    return false;
                };
                let Some(producer) = analysis.global_instance_class_owner.as_deref() else {
                    return false;
                };
                let input = source.metadata_context().and_then(
                    crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
                );
                if producer != owner
                    || input.is_none()
                    || input != analysis.global_instance_class_input.as_ref()
                {
                    return false;
                }
                let mut observed = false;
                for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
                    if let Some(tokens) =
                        crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                            &cfg.command_binding_sites,
                            statement,
                        )
                    {
                        if tokens.synthetic.is_some() {
                            continue;
                        }
                        if !owner.owns_original_tokens(tokens) {
                            return false;
                        }
                        observed = true;
                    }
                }
                observed
            })
            .and_then(|analysis| analysis.global_instance_classes.get(&source.body_id()?))
            .cloned()
            .unwrap_or_default()
    } else {
        InstanceClassState::new()
    };
    local_instance_classes_with_source(cfg, registry, &initial, source)
}

/// True when the supplied `dialect` enables iRules-specific taint rules.
///
/// Exposed so adjacent check modules (`compiler_checks`, `irules_checks`)
/// can gate their iRules-only diagnostics on the same predicate.
#[must_use]
pub fn is_irules_dialect(dialect: Option<&'static tcl_dialect::DialectProfile>) -> bool {
    dialect.is_some_and(tcl_dialect::DialectProfile::is_irules)
}

fn dialect_to_point(
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<SurfaceQuery<'static>> {
    // The profile's own point: the family alone for iRules (aliases
    // canonicalise), a release plus its package for the additive vendor
    // shells — so version-gated taint/side-effect hints reach a vendor
    // dialect's embedded Tcl core. An unknown / unset dialect stays
    // EMPTY (not the permissive fallback): taint hints gated to a specific
    // dialect must not fire when the dialect is unknown.
    dialect.map(tcl_dialect::DialectProfile::surface_query)
}

fn is_sanitiser(registry: &CommandRegistry, command: &str, args: &[&str]) -> bool {
    tcl_registry::taint::is_sanitiser(registry, command, args)
}

mod source_context;
mod source_warnings;
pub use source_context::TaintSinkSource;
pub(crate) use source_context::{TaintPropagationInputs, TaintSourceContext};
pub use source_warnings::{
    find_destructive_file_warnings_for_module_function,
    find_path_concat_warnings_for_module_function,
    find_setter_constraint_warnings_for_module_function,
};

// Taint propagation

/// Shared inputs for the per-statement taint helpers.
///
/// Bundles the registry, optional interprocedural analysis summary
/// (plus a precomputed set of its known procedure names so call-site
/// helpers don't rebuild it each invocation), and optional dialect so
/// helper functions don't need a five-argument signature. The
/// rendered-properties map is consumed only at the `propagate_taints`
/// outer level (to colour each SSA def) and is not referenced by the
/// nested helpers.
#[derive(Clone, Copy)]
pub(crate) struct TaintCtx<'a> {
    pub(crate) registry: &'a CommandRegistry,
    /// The SSA function whose taint is being computed, used to resolve a
    /// scanned variable name to its interned [`Symbol`] when indexing the
    /// [`Symbol`]-keyed `uses` / `taints` maps.
    pub(crate) ssa: &'a SsaFunction,
    pub(crate) interproc: Option<&'a InterproceduralAnalysis>,
    pub(crate) known_procs: Option<&'a HashSet<crate::ir::ProcedureId>>,
    pub(crate) dialect: Option<&'static tcl_dialect::DialectProfile>,
    pub(crate) source: TaintSourceContext<'a>,
    /// Colour-aware return summaries from the interprocedural taint
    /// solve (`taint_interproc::solve_interprocedural_taints`). When
    /// present, calls to a known proc apply the full
    /// [`crate::taint_interproc::apply_proc_return_summary`] transfer
    /// instead of the conservative single-passthrough rule.
    pub(crate) taint_summaries:
        Option<&'a HashMap<crate::ir::ProcedureId, crate::taint_interproc::ProcTaintSummary>>,
    /// Local, registry-derived object receiver classes used to resolve a
    /// `TAINT_SOURCE` instance method (`.entry get`, `$widget get`).
    pub(crate) instance_classes: Option<&'a LocalInstanceClasses>,
    /// Start offset of the statement currently being evaluated. Registry
    /// instance bindings created later in the function are not visible.
    pub(crate) source_position: Option<u32>,
    /// Retained source invocation carrying exact nested dispatch bindings.
    pub(crate) source_tokens: Option<&'a CommandTokens>,
    /// Absolute start of the word currently classified; never a text search.
    pub(crate) word_offset: Option<u32>,
    /// The exact source word; quoted whole substitutions retain their own span.
    pub(crate) source_word: Option<&'a WordExpr>,
}

impl TaintCtx<'_> {
    /// Grammar retained by this word's actual source context.
    pub(crate) fn lexer_config(self) -> tcl_lexer::LexerConfig {
        self.source.config
    }

    pub(crate) const fn at(self, source_position: u32) -> Self {
        Self {
            source_position: Some(source_position),
            ..self
        }
    }
}

/// Conditional names in argument text; syntax gaps cannot prove absence.
fn arg_var_names_with_config(
    arg: &str,
    config: tcl_lexer::LexerConfig,
) -> tcl_syntax::naming::SubstitutionVariableCandidates {
    tcl_syntax::naming::substitution_variable_candidates(arg.as_bytes(), config)
}

fn ordered_argument_candidates(
    arg: &str,
    config: tcl_lexer::LexerConfig,
    ssa: &SsaFunction,
    uses: &HashMap<Symbol, Version>,
) -> Vec<String> {
    let candidates = arg_var_names_with_config(arg, config);
    let mut names = candidates.names().to_vec();
    if candidates.has_unknown_residual() {
        let mut possible = uses
            .keys()
            .map(|&symbol| ssa.var_name(symbol).to_owned())
            .collect::<Vec<_>>();
        possible.sort_unstable();
        names.extend(possible);
    }
    names
}

fn scan_argument_names<S>(env: &TaintScan<'_, S>, arg: &str) -> Vec<String> {
    ordered_argument_candidates(arg, env.source.config, env.ssa, env.uses)
}

fn all_argument_candidates(
    arg: &str,
    config: tcl_lexer::LexerConfig,
    ssa: &SsaFunction,
) -> Vec<String> {
    let candidates = arg_var_names_with_config(arg, config);
    let mut names = candidates.names().to_vec();
    if candidates.has_unknown_residual() {
        names.extend_from_slice(ssa.var_names());
    }
    names
}

#[cfg(test)]
fn arg_var_names(arg: &str, braced_var: tcl_dialect::BracedVarStyle) -> HashSet<String> {
    let config = tcl_lexer::LexerConfig {
        braced_var,
        // dialect-drift-ok: a #[cfg(test)] helper
        ..tcl_lexer::LexerConfig::default()
    };
    arg_var_names_with_config(arg, config)
        .names()
        .iter()
        .cloned()
        .collect()
}

#[cfg(test)]
std::thread_local! {
    /// Number of synthetic compilation units built by one warning pass.
    static CALLBACK_REPLAY_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Number of source scans used to reserve synthetic replay identifiers.
    static CALLBACK_REPLAY_NAME_SEARCHES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

mod callback_replay;
mod callback_replay_source;
mod definition_taint;
mod destructive_files;
mod propagation;
mod registry_colours;
mod setter_constraints;
mod sink_warnings;
mod substitution_taint;
mod unit_report;
mod word_taint;

#[cfg(test)]
use callback_replay::{
    CALLBACK_REPLAY_INPUT_VAR_BASE, CALLBACK_REPLAY_PROC_BASE, CallbackReplayInsertions,
    MAX_CALLBACK_TAINT_REPLAYS, callback_replay_unit, collect_callback_replay_candidates,
    find_callback_substitution_warnings, insert_callback_replays,
    original_callback_replay_residuals,
};
#[cfg(test)]
use callback_replay_source::callback_replay_script;
#[cfg(test)]
use callback_replay_source::callback_replay_source;
use callback_replay_source::{callback_static_word_value, decoded_text_fragment};
use definition_taint::var_taint;
#[cfg(test)]
use definition_taint::{evaluate_taint_def, join_uses};
pub use destructive_files::find_destructive_file_warnings;
#[cfg(test)]
use destructive_files::{extract_guard_var, extract_var_name, guard_var_from_string_command};
#[cfg(test)]
use propagation::propagate_statement_taints;
pub(crate) use propagation::{TaintGraph, propagate_taints};
#[cfg(test)]
use propagation::{is_actual_formal_entry, seed_entry_taints};
#[cfg(test)]
use registry_colours::compiler_colour;
#[cfg(test)]
pub(crate) use registry_colours::unique_instance_class;
use registry_colours::{double_encode_label, reg_colour, transform_colour};
pub use setter_constraints::find_setter_constraint_warnings;
use sink_warnings::SourceWarningMetadata;
#[cfg(test)]
use sink_warnings::{
    ScaffoldConstructors, collect_coercion_operands, command_substitutions_in_word,
    emit_dynamic_command_head_warning, emit_isolated_script_warnings, emit_regexp_pattern_warnings,
    global_heads_name_one_command, holds_one_whole_var_ref, list_wrapped_arg_command_is_literal,
    operand_read_taint, option_injection_arguments, parse_taint_command_substitution,
    reference_root_with_config, resolve_taint_command_heads, selected_sink_schema,
    shadowed_builtin_names, sink_var_position_safe, standalone_list_wrapper_head,
    var_consumed_by_sanitiser, var_wrapper_colours,
};
#[cfg(test)]
use sink_warnings::{
    classify_network_interp_sinks, classify_sink, irule3002_name_position_safe,
    original_scalar_reference_name, sink_arg_span, sink_colour_mitigated,
};
pub use sink_warnings::{
    find_taint_warnings, find_taint_warnings_for_function, find_taint_warnings_for_module_function,
    shadowed_builtin_names_for_module_function, taint_warning_report_for_module_function,
};
#[cfg(test)]
use substitution_taint::expr_command_taint;
#[cfg(test)]
use unit_report::find_taint_warnings_for_cu_base;
use unit_report::taint_warning_report_for_cu_base_with_external_variable_seeds;
pub use unit_report::{
    apply_module_variable_traces, find_taint_warnings_for_cu, taint_warning_report_for_cu,
};
pub(crate) use word_taint::word_taint;
#[cfg(test)]
use word_taint::{
    interpolation_carve_out, leading_literal_prefix_char, literal_contains_crlf,
    original_component_read_taint, retained_word_source,
};

#[cfg(test)]
fn reset_callback_replay_build_count() {
    CALLBACK_REPLAY_BUILDS.with(|count| count.set(0));
}

#[cfg(test)]
fn callback_replay_build_count() -> usize {
    CALLBACK_REPLAY_BUILDS.with(std::cell::Cell::get)
}

#[cfg(test)]
fn reset_callback_replay_name_search_count() {
    CALLBACK_REPLAY_NAME_SEARCHES.with(|count| count.set(0));
}

#[cfg(test)]
fn callback_replay_name_search_count() -> usize {
    CALLBACK_REPLAY_NAME_SEARCHES.with(std::cell::Cell::get)
}

/// Statically recoverable callback registration, collected before allocating
/// replay identifiers. Keeping the registry query separate from rendering
/// makes the no-callback path allocation-free apart from ordinary CFG facts.
struct CallbackReplayCandidate {
    callback: String,
    inputs: &'static [tcl_registry::CallbackTaintInput],
    source_is_command_substitution: bool,
    callback_span: Span,
    callback_input_label: String,
    selected_builder: Option<CallbackListBuilder>,
}

/// The actual selected registry owner for one whole source callback builder.
/// This authorizes only list-result materialization, not callback execution.
#[derive(Clone)]
struct CallbackListBuilder {
    source: String,
    invocation: crate::registry_invocation::ResolvedStatementInvocation,
}

/// Each callback is an independently retained future procedure body.
/// Its formal input is seeded by the taint solver, without sequencing
/// hypothetical callback executions through the document world.
struct CallbackReplaySource {
    definition: String,
}

#[derive(Clone, Copy)]
struct CommandHeadContext<'a> {
    fu: &'a crate::compilation_unit::FunctionUnit,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'a>,
    identities: &'a crate::realm::CommandBindingRealm,
}

#[derive(Clone, Copy)]
struct SinkWarningContext<'a, H> {
    registry: &'a CommandRegistry,
    source: TaintSourceContext<'a>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    shadowed_builtins: &'a HashSet<String, H>,
    original_module: Option<&'a crate::ir::Module>,
    unavailable_replays: Option<&'a std::cell::RefCell<Vec<UnavailableTaintReplay>>>,
}

/// Conditional original source reads retain exact positioned value authority.
#[derive(Clone, Copy)]
struct SourceWarningRead<'a> {
    metadata: &'a SourceWarningMetadata,
    point: crate::ssa::SsaSourceView<'a>,
}

/// Per-statement read context for the taint-warning emitters: the SSA
/// versions reaching the statement (`uses`), the taint lattice, and the SSA
/// function they resolve against. Bundled so the emitters stay within the
/// argument limit.
struct TaintScan<'a, S> {
    uses: &'a HashMap<Symbol, u32>,
    taints: &'a HashMap<ValueKey, TaintLattice, S>,
    ssa: &'a SsaFunction,
    /// Uses mentioned only in brace-quoted words at this statement.
    quoted_uses: Option<&'a HashSet<Symbol>>,
    /// Current source metadata cannot turn an unrepresented read into clean SSA.
    source_read: Option<SourceWarningRead<'a>>,
    /// Retained grammar and metadata for wrapper mitigation candidates.
    source: TaintSourceContext<'a>,
}

/// Emit one warning per tainted use flowing into a classified sink.
///
/// Deduplicates on variable name so the same variable appearing multiple
/// times in `uses` only produces one warning. For iRules sinks
/// (`IRULE3001` / `IRULE3002` / `IRULE3003` / `IRULE3004`), applies the
/// per-code mitigation masks via [`sink_colour_mitigated`] plus the
/// name-position `HEADER_TOKEN_SAFE` carve-out for IRULE3002.
/// Context for a classified sink call, bundled so
/// [`emit_sink_warnings`] stays under the 7-argument clippy limit
/// while still carrying the command name + arg slice needed for the
/// IRULE3002 name-position mitigation.
#[derive(Clone, Copy)]
struct SinkCall<'a> {
    source: TaintSourceContext<'a>,
    /// Raw command name (e.g. `"HTTP::header"`).
    command: &'a str,
    /// Argument vector as seen by the sink.
    args: &'a [String],
    /// Command registry — drives the position-aware sink filters
    /// (network-address slots, `[list]`-head recognition).
    registry: &'a CommandRegistry,
    /// Per-word source spans for this call, when available (`Statement::Call`
    /// / `Statement::Barrier` carry them; the `AssignValue` "owned fallback"
    /// path — a nested `[cmd …]` re-parsed out of a `set` RHS — does not, so
    /// `None` there, and the synthetic sink call built for a nested `[cmd
    /// …]` inside a branch condition also passes `None`). Drives
    /// [`sink_arg_span`]'s per-argument highlighting; `None` falls every hit
    /// back to the whole-statement span.
    tokens: Option<&'a CommandTokens>,
    /// Candidate-specific lexical origins, including aliases; no effect/result proof.
    effective: Option<&'a crate::registry_invocation::EffectiveCommandWords>,
    /// Sealed conditional source read advice, independently of SSA cell authority.
    source_read: Option<SourceWarningRead<'a>>,
    /// First argument in the retained effective invocation for a sliced sink region.
    argument_offset: usize,
}

// Tests

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_taint_local_parser_purpose_tests;

#[cfg(test)]
mod original_taint_word_purpose_tests;

#[cfg(test)]
mod authored_script_read_ownership_tests;

#[cfg(test)]
mod authored_profileless_sink_owner_tests;

#[cfg(test)]
mod original_script_contents_coverage_tests;

#[cfg(test)]
mod boolean_result_controls;
