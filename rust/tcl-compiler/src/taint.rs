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
//!   address colour (T104) / `LIST_CANONICAL` (T105).
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
//! - **`rename` command-name aliasing** — sink/source classification keys
//!   off [`Statement::canonical_command_or_source`], and the lowerer feeds a
//!   static `rename old new` into the same alias table `interp alias`
//!   already populates, so a call through a renamed name resolves to
//!   whatever it now denotes (see `crate::alias::detect_rename`).
//! - **Namespace-scoped proc shadowing a builtin** — a call whose bare or
//!   canonical name is shadowed, from its own namespace, by a user `proc` of
//!   the same name is not misclassified as the builtin sink/source; see
//!   [`shadowed_builtin_names`].
//! - **Variable-trace-altered taint** — `trace add variable` / `trace
//!   variable` conservatively forces a traced name to fully tainted
//!   everywhere in the module, since a runtime handler can rewrite the
//!   value in ways static analysis can't see; see
//!   [`apply_module_variable_traces`].
//!
//! ## Source / sink / sanitiser facts live in the registry
//!
//! The source / sanitiser tables live in
//! [`tcl_registry::taint`]. This module asks the registry the
//! questions it used to answer locally:
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
use rustc_hash::{FxHashMap, FxHashSet};

use tcl_dialect::DialectSet;
use tcl_lexer::{Lexer, SourceMap, Span, TokenType, backslash_subst};
use tcl_registry::{ArgRole, CommandRegistry, TaintColourAtom, Traits};

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::depth_guard::{MAX_BRACKET_TEXT_DEPTH, MAX_EXPR_NODE_DEPTH};
use crate::expr_ast::ExprNode;
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::{CommandTokens, Statement, WordExpr, WordPart};
use crate::irules_checks::CodeFix;
use crate::naming::{normalise_qualified_name, normalise_var_name};
use crate::regex_source::regexp_pattern_index;
use crate::rendered_properties::{RenderedProperties, RenderedValueProps};
use crate::sccp::{SccpResult, cfg_order};
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use crate::value_shapes::{
    is_pure_var_ref, parse_command_substitution, parse_command_substitution_with_spans,
};

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

/// Per-SSA-value taint lattice element: a bag of colours plus a
/// flag tracking whether any incoming path definitely set
/// `TAINTED`.
///
/// `colours` is the "must-have" intersection at joins — a colour
/// survives only when every incoming edge has it. Taint is a
/// "may-have" — once any path sets it, it sticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaintLattice {
    /// Bag of colours. `TAINTED` membership means "may be tainted".
    pub colours: TaintColour,
}

impl TaintLattice {
    /// Fresh clean value — no taint, no mitigations proven.
    #[must_use]
    pub const fn clean() -> Self {
        Self {
            colours: TaintColour::empty(),
        }
    }

    /// Fully tainted with no mitigations.
    #[must_use]
    pub const fn tainted() -> Self {
        Self {
            colours: TaintColour::TAINTED,
        }
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
        Self {
            colours: taint | mitigations,
        }
    }

    /// Add a colour (typically a mitigation).
    #[must_use]
    pub fn with(self, c: TaintColour) -> Self {
        Self {
            colours: self.colours | c,
        }
    }

    /// Remove `TAINTED` — used by sanitisers.
    #[must_use]
    pub fn sanitised(self) -> Self {
        Self {
            colours: self.colours & !TaintColour::TAINTED,
        }
    }

    /// Strip the shape-dependent mitigation colours (`T102_SAFE`:
    /// `PATH_PREFIXED` / `NON_DASH_PREFIXED` / `IP_ADDRESS` / `PORT` /
    /// `FQDN`) — used when a value passes through a command with no
    /// registry classification (not a recognised source, sanitiser,
    /// transform, or passthrough), so its effect on the value's string
    /// shape is unknown. A "cannot start with `-`"-style proof cannot be
    /// assumed to survive an arbitrary unclassified transformation (e.g.
    /// `string range`, `lindex [split ...]`) even though the underlying
    /// taint itself must. Commands specifically known to preserve shape
    /// should get a registry classification instead of relying on this
    /// (conservative, over-approximating) default.
    #[must_use]
    pub fn shape_unproven(self) -> Self {
        Self {
            colours: self.colours & !TaintColour::T102_SAFE,
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

// Source-command classification

/// Return `true` when `command` is a known taint source — i.e. its
/// return value may carry attacker-influenced data.
///
/// Commands with `UNNORMALISED_HTTP_GETTER` trait (iRules dialect) are
/// also included once the registry carries that flag on actual specs.
/// When `dialect` is `"f5-irules"` / `"irules"`, iRules namespace
/// prefixes (`HTTP::`, `URI::`, `IP::`, …) are also treated as
/// attacker-controlled sources.
/// The taint lattice a source `command` produces, or `None` when the
/// call is not a taint source. Wraps the registry's
/// `taint_source_colour` (which carries the per-command source colour
/// and the derived-safety augmentation) into the compiler's mirror
/// lattice. Replaces the old "any source ⇒ bare `TAINTED`" rule so
/// path/IP/port/FQDN getters propagate their option-injection-safe
/// colours.
fn source_colour(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: Option<&tcl_dialect::DialectProfile>,
    instance_classes: Option<&LocalInstanceClasses>,
    source_position: Option<u32>,
) -> Option<TaintLattice> {
    let dialect_set = dialect_to_set(dialect);
    let direct = tcl_registry::taint::taint_source_colour(registry, command, args, dialect_set);
    let colour = direct.or_else(|| {
        instance_taint_source_colour(
            registry,
            command,
            args,
            dialect_set,
            instance_classes,
            source_position,
        )
    })?;
    Some(TaintLattice {
        colours: reg_colour(colour),
    })
}

/// Resolve a registry-declared source on a statically-known object instance.
///
/// Tcl widget and package-object APIs are invoked through a command created at
/// runtime (`.entry get`, `$widget get`) rather than through the constructor's
/// registered command name.  The object-class and `creates_instance_at`
/// descriptors already model that dispatch; this is the taint counterpart.
/// It deliberately knows neither Tk widget names nor method spellings: a
/// source is recognised only when the registry says the receiver's class has
/// an instance method carrying [`Traits::TAINT_SOURCE`].
///
/// A receiver with multiple possible classes is not classified.  A source is
/// a security fact, so guessing a method from a widened object flow would turn
/// an unrelated dynamic object call into a false positive.
fn instance_taint_source_colour(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: tcl_dialect::DialectSet,
    instance_classes: Option<&LocalInstanceClasses>,
    source_position: Option<u32>,
) -> Option<tcl_registry::TaintColour> {
    let class = unique_instance_class(command, instance_classes, source_position)?;
    registry
        .resolve_instance_invocation(class, command, args, dialect)
        .filter(|invocation| {
            invocation.semantics.traits.contains(Traits::TAINT_SOURCE)
                || invocation
                    .semantics
                    .traits
                    .contains(Traits::TAINT_SOURCE_ZERO_ARGS)
                    && args.len() == 1
        })
        .map(|_| tcl_registry::TaintColour::TAINTED)
}

/// The receiver's single registry class at `source_position`.
///
/// Shared by source-return and linked-variable instance semantics so both
/// abstain identically on an untyped or widened receiver.
pub(crate) fn unique_instance_class<'a>(
    command: &str,
    instance_classes: Option<&'a LocalInstanceClasses>,
    source_position: Option<u32>,
) -> Option<&'a str> {
    let key = normalise_var_name(command);
    let position = source_position?;
    let classes = instance_classes?.at.get(&position)?.get(key)?;
    if classes.len() != 1 {
        return None;
    }
    classes.iter().next().map(String::as_str)
}

/// Whether a registry-typed instance call exposes `variable` to external
/// input through one of its class/factory options.
fn instance_taints_var_write(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: Option<&tcl_dialect::DialectProfile>,
    instance_classes: Option<&LocalInstanceClasses>,
    source_position: Option<u32>,
    variable: &str,
) -> bool {
    if args.is_empty() {
        return false;
    }
    let Some(class) = unique_instance_class(command, instance_classes, source_position) else {
        return false;
    };
    let dialect = dialect_to_set(dialect);
    let Some(invocation) = registry.resolve_instance_invocation(class, command, args, dialect)
    else {
        return false;
    };
    invocation
        .semantics
        .traits
        .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
        && tcl_registry::taint::taints_var_write(registry, class, &args[1..], dialect, variable)
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
    local_instance_classes_with_initial(cfg, registry, &InstanceClassState::new())
}

fn factory_class<'a>(
    registry: &'a CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<&'a str> {
    if let Some(method) = args.first()
        && registry
            .exported_manufacturer_method(head, method)
            .is_some()
    {
        return registry.object_class(head).map(|class| class.class_name);
    }
    registry
        .get(head)
        .filter(|spec| spec.creates_instance_at.is_some())
        .and_then(|spec| spec.object_class)
        .map(|class| class.class_name)
}

fn literal_receiver(name: &str) -> Option<&str> {
    (!name.is_empty() && !name.starts_with(['$', '[', '{'])).then_some(name)
}

fn transfer_instance_statement(
    state: &mut InstanceClassState,
    statement: &Statement,
    registry: &CommandRegistry,
) {
    match statement {
        Statement::Call {
            command,
            args,
            defs,
            ..
        } => {
            transfer_instance_lifecycle(state, command, args, registry);
            // A registry-known write to a handle variable invalidates the old
            // type unless this very statement installs a fresh factory fact.
            for name in defs {
                state.remove(normalise_var_name(name));
            }
            if let Some(spec) = registry.get(command)
                && let Some(index) = spec.creates_instance_at
                && let Some(name) = args.get(usize::from(index))
                && let Some(name) = literal_receiver(name)
            {
                // Every naming factory replaces the command at this literal
                // receiver, even when its registry row does not expose an
                // object class. In that case the sound reaching fact is
                // "untyped", not the previous constructor's stale class.
                state.remove(name);
                if let Some(class) = spec.object_class {
                    state.insert(
                        name.to_owned(),
                        HashSet::from([class.class_name.to_owned()]),
                    );
                }
            }
        }
        Statement::AssignValue { name, value, .. } => {
            let name = normalise_var_name(name);
            state.remove(name);
            if let Some((head, args)) = parse_command_substitution(value.trim())
                && let Some(class) = factory_class(registry, &head, &args)
                && let Some(name) = literal_receiver(name)
            {
                state.insert(name.to_owned(), HashSet::from([class.to_owned()]));
            }
        }
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => {
            state.remove(normalise_var_name(name));
        }
        _ => {}
    }
}

/// Apply registry-declared command/object lifecycle changes to the local
/// receiver-class map.  This is deliberately driven by the registry's
/// command-table and teardown descriptors rather than by command-name
/// switches: a rename moves the class fact, an alias/dynamic mutation clears
/// it, and a Tk teardown removes the target window and its descendants.
pub(crate) fn transfer_instance_lifecycle(
    state: &mut InstanceClassState,
    command: &str,
    args: &[String],
    registry: &CommandRegistry,
) {
    if let Some(effect) = registry.command_table_effect(command, args.first().map(String::as_str)) {
        match effect {
            tcl_registry::CommandTableEffect::RenamesCommands => {
                let (Some(old), Some(new)) = (args.first(), args.get(1)) else {
                    state.clear();
                    return;
                };
                let Some(old) = literal_receiver(old) else {
                    state.clear();
                    return;
                };
                if new.starts_with(['$', '[', '{']) {
                    state.clear();
                    return;
                }
                let class = state.remove(old);
                if !new.is_empty()
                    && let Some(class) = class
                {
                    state.insert(new.to_owned(), class);
                }
            }
            tcl_registry::CommandTableEffect::CreatesAliases => {
                // An alias may target or replace any command, including a
                // registry-modelled instance command.  Without a precise
                // target proof all receiver identities become unknown.
                state.clear();
            }
            tcl_registry::CommandTableEffect::DefinesProcedure => {}
        }
        return;
    }

    // Tk's `destroy` is a registry-declared fire-and-forget teardown.  Use the
    // package declaration to distinguish it from unrelated teardown commands
    // such as `unset` and `after cancel`, while keeping the consumer generic.
    let Some(spec) = registry.get(command) else {
        return;
    };
    if !spec.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN)
        || spec.required_package != Some("Tk")
    {
        return;
    }
    for target in args {
        let Some(target) = literal_receiver(target) else {
            state.clear();
            return;
        };
        state.retain(|name, _| !tcl_registry::tk_geometry::widget_path_is_within(name, target));
    }
}

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

pub(crate) fn local_instance_classes_with_initial(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    initial: &InstanceClassState,
) -> LocalInstanceClasses {
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
                transfer_instance_statement(&mut state, statement, registry);
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
            classes.at.insert(statement.span().start(), state.clone());
            transfer_instance_statement(&mut state, statement, registry);
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

/// Function-local receiver facts plus interpreter-global instance commands
/// proven to exist at this procedure's earliest execution phase. The top-level
/// function keeps only its local, source-ordered facts to avoid retroactively
/// typing an earlier call.
pub(crate) fn instance_classes_for_function(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    interproc: Option<&InterproceduralAnalysis>,
    include_globals: bool,
) -> LocalInstanceClasses {
    #[cfg(test)]
    INSTANCE_CLASS_SOLVES.with(|count| count.set(count.get() + 1));
    let initial = if include_globals {
        interproc
            .and_then(|analysis| analysis.global_instance_classes.get(&cfg.name))
            .cloned()
            .unwrap_or_default()
    } else {
        InstanceClassState::new()
    };
    local_instance_classes_with_initial(cfg, registry, &initial)
}

/// Bridge a registry colour to the compiler's mirror enum.
///
/// This must remain a total conversion: a registry bit must never disappear
/// while crossing into the compiler's lattice. `from_bits` makes a future
/// mismatch fail closed during development instead of silently truncating a
/// sanitiser declaration (issue #1410).
const fn compiler_colour_atom(atom: TaintColourAtom) -> TaintColour {
    match atom {
        TaintColourAtom::Tainted => TaintColour::TAINTED,
        TaintColourAtom::PathPrefixed => TaintColour::PATH_PREFIXED,
        TaintColourAtom::NonDashPrefixed => TaintColour::NON_DASH_PREFIXED,
        TaintColourAtom::CrlfFree => TaintColour::CRLF_FREE,
        TaintColourAtom::ShellAtom => TaintColour::SHELL_ATOM,
        TaintColourAtom::ListCanonical => TaintColour::LIST_CANONICAL,
        TaintColourAtom::RegexLiteral => TaintColour::REGEX_LITERAL,
        TaintColourAtom::PathNormalised => TaintColour::PATH_NORMALISED,
        TaintColourAtom::PathBounded => TaintColour::PATH_BOUNDED,
        TaintColourAtom::HeaderTokenSafe => TaintColour::HEADER_TOKEN_SAFE,
        TaintColourAtom::HtmlEscaped => TaintColour::HTML_ESCAPED,
        TaintColourAtom::UrlEncoded => TaintColour::URL_ENCODED,
        TaintColourAtom::IpAddress => TaintColour::IP_ADDRESS,
        TaintColourAtom::Port => TaintColour::PORT,
        TaintColourAtom::Fqdn => TaintColour::FQDN,
        TaintColourAtom::PathJoined => TaintColour::PATH_JOINED,
        TaintColourAtom::Channel => TaintColour::CHANNEL,
    }
}

fn reg_colour(c: tcl_registry::TaintColour) -> TaintColour {
    let mut out = TaintColour::empty();
    for atom in TaintColourAtom::ALL {
        if c.contains(atom.colour()) {
            out |= compiler_colour_atom(atom);
        }
    }
    assert_eq!(
        out.bits(),
        c.bits(),
        "registry and compiler TaintColour definitions must stay aligned"
    );
    out
}

/// The inverse bridge, kept beside [`reg_colour`] so conversions stay audited
/// in both directions whenever the registry adds a colour.
#[cfg(test)]
fn compiler_colour(c: TaintColour) -> tcl_registry::TaintColour {
    let mut out = tcl_registry::TaintColour::empty();
    for atom in TaintColourAtom::ALL {
        if c.contains(compiler_colour_atom(atom)) {
            out |= atom.colour();
        }
    }
    assert_eq!(
        out.bits(),
        c.bits(),
        "compiler and registry TaintColour definitions must stay aligned"
    );
    out
}

/// The colour a command stamps on a tainted value it returns — its
/// `taint_transform` (e.g. `uri::encode` ⇒ `URL_ENCODED`,
/// `file normalize` ⇒ `PATH_NORMALISED`).  Subcommand transforms take
/// precedence over the bare-command form.
fn transform_colour(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
) -> Option<TaintColour> {
    let spec = registry.get(command)?;
    if let Some(sub_name) = args.first()
        && let Some(sub) = spec.resolve_subcommand(sub_name)
        && let Some(colour) = sub.taint_transform
    {
        return Some(reg_colour(colour));
    }
    spec.taint_transform.map(reg_colour)
}

/// Human-readable label for a double-encode colour, for the T106
/// message.
fn double_encode_label(colour: TaintColour) -> &'static str {
    if colour.contains(TaintColour::URL_ENCODED) {
        "URL-encoded"
    } else if colour.contains(TaintColour::HTML_ESCAPED) {
        "HTML-escaped"
    } else if colour.contains(TaintColour::REGEX_LITERAL) {
        "regex-escaped"
    } else {
        "encoded"
    }
}

/// True when the supplied `dialect` enables iRules-specific taint rules.
///
/// Exposed so adjacent check modules (`compiler_checks`, `irules_checks`)
/// can gate their iRules-only diagnostics on the same predicate.
#[must_use]
pub fn is_irules_dialect(dialect: Option<&tcl_dialect::DialectProfile>) -> bool {
    dialect.is_some_and(tcl_dialect::DialectProfile::is_irules)
}

fn dialect_to_set(dialect: Option<&tcl_dialect::DialectProfile>) -> DialectSet {
    // The profile's availability mask: bare IRULES for iRules (aliases
    // canonicalise), the composed (version|vendor) mask for the additive
    // vendor shells — so version-gated taint/side-effect hints reach a
    // vendor dialect's embedded Tcl core. An unknown / unset dialect stays
    // EMPTY (not the permissive fallback): taint hints gated to a specific
    // dialect must not fire when the dialect is unknown.
    dialect.map_or(DialectSet::empty(), |profile| profile.availability_mask)
}

fn is_sanitiser(registry: &CommandRegistry, command: &str, args: &[&str]) -> bool {
    tcl_registry::taint::is_sanitiser(registry, command, args)
}

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
    pub(crate) known_procs: Option<&'a HashSet<String>>,
    pub(crate) caller_qname: Option<&'a str>,
    pub(crate) dialect: Option<&'a tcl_dialect::DialectProfile>,
    /// Colour-aware return summaries from the interprocedural taint
    /// solve (`taint_interproc::solve_interprocedural_taints`). When
    /// present, calls to a known proc apply the full
    /// [`crate::taint_interproc::apply_proc_return_summary`] transfer
    /// instead of the conservative single-passthrough rule.
    pub(crate) taint_summaries:
        Option<&'a HashMap<String, crate::taint_interproc::ProcTaintSummary>>,
    /// Local, registry-derived object receiver classes used to resolve a
    /// `TAINT_SOURCE` instance method (`.entry get`, `$widget get`).
    pub(crate) instance_classes: Option<&'a LocalInstanceClasses>,
    /// Start offset of the statement currently being evaluated. Registry
    /// instance bindings created later in the function are not visible.
    pub(crate) source_position: Option<u32>,
}

impl TaintCtx<'_> {
    /// This document's `${…}` close rule, for the shared owner
    /// [`tcl_lexer::braced_var_name_end`].
    ///
    /// Taint is keyed by variable *name*, so a scan that recovers the wrong
    /// name tracks the wrong cell: the tainted value's colour is dropped and
    /// the security diagnostics that depend on it (T1xx, W201) go quiet on a
    /// sink they should flag. With no explicit dialect the document was lexed
    /// under [`tcl_dialect::BracedVarStyle::default`] (issue #1604).
    pub(crate) fn braced_var(self) -> tcl_dialect::BracedVarStyle {
        tcl_dialect::BracedVarStyle::of_profile(self.dialect)
    }

    pub(crate) const fn at(self, source_position: u32) -> Self {
        Self {
            source_position: Some(source_position),
            ..self
        }
    }
}

/// Infer the taint of an argument word from already-known per-variable
/// taint values.
///
/// Handles pure variable references (`$x`), bracketed command
/// substitutions (`[cmd ...]`), and interpolated strings.
pub(crate) fn word_taint<S: std::hash::BuildHasher>(
    word: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
) -> TaintLattice {
    // Public entry: a word's raw text is bracket-nesting depth 0 (issue
    // #996 — the recursion cap lives in [`word_taint_at`]).
    word_taint_at(word, uses, taints, ctx, 0)
}

/// Depth-carrying core of [`word_taint`]. `depth` counts nested `[cmd …]`
/// command substitutions *within this word's raw text* — a genuinely
/// unbounded recursion axis (issue #996), independent of any expression- or
/// statement-tree cap. `[a [b [c …]]]` nested N deep recurses N frames here.
fn word_taint_at<S: std::hash::BuildHasher>(
    word: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> TaintLattice {
    // Native-stack safety net. Past the cap, assume the word is tainted with
    // no proven mitigations — the conservative direction for a taint
    // analysis, so a source buried in nesting we stopped scanning can never
    // be laundered into a false-negative security result.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return TaintLattice::tainted();
    }
    let stripped = word.trim();

    // Pure variable reference — inherit taint directly.
    if is_pure_var_ref(stripped) {
        let name = normalise_var_name(stripped);
        return var_taint(name, uses, taints, ctx.ssa);
    }

    // Bracketed command substitution.
    if let Some((cmd, args)) = parse_command_substitution(stripped) {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        if is_sanitiser(ctx.registry, &cmd, &arg_refs) {
            return TaintLattice::clean();
        }
        if let Some(t) = source_colour(
            ctx.registry,
            &cmd,
            &arg_refs,
            ctx.dialect,
            ctx.instance_classes,
            ctx.source_position,
        ) {
            return t;
        }
        // Interprocedural: if `cmd` resolves to a known proc with a
        // passthrough parameter, propagate the taint of the matching
        // actual.
        if let Some(t) = interproc_call_taint(&cmd, &args, uses, taints, ctx) {
            return t;
        }
        // Propagate from the arguments inside the command sub. `cmd` has
        // no registry classification (source / sanitiser / passthrough all
        // missed above), so its effect on argument shape is unknown —
        // strip shape-dependent mitigations (`shape_unproven`) rather than
        // optimistically assuming e.g. `PATH_PREFIXED` survives an
        // arbitrary `string range` / `lindex [split ...]` transform.
        let mut t = TaintLattice::clean();
        for arg in &args {
            t = t.join(word_taint_at(arg, uses, taints, ctx, depth + 1));
        }
        t = t.shape_unproven();
        // Stamp the encoder/transform colour the command adds to its result
        // (e.g. `uri::encode` → `URL_ENCODED`, `file normalize` →
        // `PATH_NORMALISED`), so a later pass through the same encoder is
        // detectable as a double-encode (T106) and a consumer asking "has
        // this value been through a normalising sanitiser" gets a yes.
        //
        // Stamped regardless of taint (issue #1391).  The colour describes
        // what the *command* guarantees about its result, not what its input
        // was, so gating it on `is_tainted()` made the fact unobservable for
        // exactly the values a hygiene check like W201 asks about — a path
        // built from clean local variables and then normalised.  Nothing
        // widens as a result: every consumer that acts on a colour
        // ([`emit_double_encode_warnings`], the sink mitigation checks)
        // requires `is_tainted()` first, and a clean lattice is `join`'s
        // identity, so a clean colour never dilutes a tainted operand's
        // must-have mitigations.
        if let Some(colour) = transform_colour(ctx.registry, &cmd, &arg_refs) {
            t = t.with(colour);
        }
        return t;
    }

    // Interpolated string: scan for $var references and [cmd] substitutions.
    if stripped.contains('$') || stripped.contains('[') {
        let mut t = TaintLattice::clean();

        // Scan for [cmd ...] command substitutions (non-nested).
        let mut rest = stripped;
        while let Some(open) = rest.find('[') {
            rest = &rest[open..];
            if let Some(close) = rest.find(']') {
                let sub = &rest[..=close];
                // Only recurse when the bracketed slice is strictly
                // smaller than the word being analysed — i.e. it's a
                // substitution *embedded* in surrounding text. When the
                // whole word is itself the bracketed region it was
                // already handled by the command-substitution branch
                // above; recursing on it again (e.g. the empty `[]`
                // inside `{[]}`) makes no progress and would recurse
                // until the stack overflows.
                if sub.len() < stripped.len() {
                    t = t.join(word_taint_at(sub, uses, taints, ctx, depth + 1));
                }
                rest = &rest[close + 1..];
            } else {
                break;
            }
        }

        // Scan for $var references.
        let mut rest = stripped;
        while let Some(pos) = rest.find('$') {
            rest = &rest[pos + 1..];
            // ${name} form.
            let raw_name = if rest.starts_with('{') {
                // The name starts just past the `${`; an unterminated
                // reference names nothing, so stop scanning this word.
                match tcl_lexer::braced_var_name_end(rest.as_bytes(), 1, ctx.braced_var()) {
                    tcl_lexer::BracedVarEnd::Closed(end) => {
                        let n = &rest[1..end];
                        rest = &rest[end + 1..];
                        n
                    }
                    tcl_lexer::BracedVarEnd::Unterminated => break,
                }
            } else {
                // $name — grab identifier chars (including :: for namespaces).
                let end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_' && c != ':')
                    .unwrap_or(rest.len());
                let n = &rest[..end];
                rest = &rest[end..];
                n
            };
            if !raw_name.is_empty() {
                let name = normalise_var_name(raw_name);
                t = t.join(var_taint(name, uses, taints, ctx.ssa));
            }
        }
        return interpolation_carve_out(word, t);
    }

    TaintLattice::clean()
}

/// Re-derive the structural / option-prefix colours of an interpolated
/// (concatenated) word from its literal fragments.
///
/// Interpolation invalidates every structural guarantee unless the
/// literal text re-establishes it: the canonical-list / normalised-path /
/// escaped colours are cleared, `CRLF_FREE` is cleared when a literal
/// fragment contains CR/LF, and the leading literal character controls
/// option-prefix safety (`PATH_PREFIXED` / `NON_DASH_PREFIXED`). A clean
/// join is returned unchanged.
fn interpolation_carve_out(value: &str, joined: TaintLattice) -> TaintLattice {
    if !joined.is_tainted() {
        return TaintLattice::clean();
    }
    let mut colour = joined.colours;
    colour &= !(TaintColour::LIST_CANONICAL
        | TaintColour::PATH_NORMALISED
        | TaintColour::PATH_BOUNDED
        | TaintColour::HEADER_TOKEN_SAFE
        | TaintColour::HTML_ESCAPED
        | TaintColour::URL_ENCODED
        | TaintColour::REGEX_LITERAL
        | TaintColour::SHELL_ATOM);
    if literal_contains_crlf(value) {
        colour &= !TaintColour::CRLF_FREE;
    }
    match leading_literal_prefix_char(value) {
        Some('/') => colour |= TaintColour::PATH_PREFIXED | TaintColour::NON_DASH_PREFIXED,
        Some('-') => colour &= !(TaintColour::NON_DASH_PREFIXED | TaintColour::PATH_PREFIXED),
        Some(_) => {
            colour |= TaintColour::NON_DASH_PREFIXED;
            colour &= !TaintColour::PATH_PREFIXED;
        }
        None => {}
    }
    TaintLattice {
        colours: colour | TaintColour::TAINTED,
    }
}

/// Return the leading literal character of `value`, or `None` when the
/// word starts with a variable/command substitution (dynamic prefix).
///
/// The first `Esc` token is rendered through `backslash_subst` (so
/// `\x2f` → `/`); the first `Str` (braced) token contributes its literal
/// first char; a leading `Var` or `Cmd` token means the prefix is
/// dynamic.
fn leading_literal_prefix_char(value: &str) -> Option<char> {
    let source_map = SourceMap::new(value);
    let tokens = Lexer::new(value).tokenise_all().ok()?;
    for tok in tokens {
        match tok.kind {
            // End of input, or a dynamic (variable/command) prefix — no
            // literal leading character to report.
            TokenType::Eol | TokenType::Eof | TokenType::Var | TokenType::Cmd => return None,
            TokenType::Esc => {
                let text = source_map.text(tok.span);
                let rendered = if text.contains('\\') {
                    backslash_subst(text).into_owned()
                } else {
                    text.to_owned()
                };
                if let Some(c) = rendered.chars().next() {
                    return Some(c);
                }
            }
            TokenType::Str => {
                if let Some(c) = source_map.text(tok.span).chars().next() {
                    return Some(c);
                }
            }
            _ => {}
        }
    }
    None
}

/// Return `true` when any rendered literal fragment of `value` contains a
/// CR or LF.: `Esc` fragments are
/// `backslash_subst`-rendered (so `\n` resolves to a real newline) before
/// the scan.
fn literal_contains_crlf(value: &str) -> bool {
    let source_map = SourceMap::new(value);
    let Ok(tokens) = Lexer::new(value).tokenise_all() else {
        return false;
    };
    for tok in tokens {
        match tok.kind {
            TokenType::Eol => return false,
            TokenType::Esc => {
                let text = source_map.text(tok.span);
                let rendered = if text.contains('\\') {
                    backslash_subst(text).into_owned()
                } else {
                    text.to_owned()
                };
                if rendered.contains('\r') || rendered.contains('\n') {
                    return true;
                }
            }
            TokenType::Str => {
                let text = source_map.text(tok.span);
                if text.contains('\r') || text.contains('\n') {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// When `command` resolves to an internal proc with a known
/// `return_passthrough_param`, return the taint of the corresponding
/// actual argument. Returns `None` when interprocedural summaries are
/// not available or the call doesn't resolve.
fn interproc_call_taint<S: std::hash::BuildHasher>(
    command: &str,
    args: &[String],
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
) -> Option<TaintLattice> {
    let known = ctx.known_procs?;
    let caller = ctx.caller_qname.unwrap_or("::top");

    // Colour-aware return-summary path: when the interprocedural taint
    // solve has computed per-proc summaries, apply the full transfer.
    // A resolved callee
    // always yields a result (untainted when the arity rejects the call),
    // so the bare argument join below is bypassed for internal calls.
    if let Some(summaries) = ctx.taint_summaries {
        let target = crate::interprocedural::resolve_call_target(command, args, caller, known)?;
        let summary = summaries.get(&target)?;
        let arg_taints: Vec<TaintLattice> = args
            .iter()
            .map(|a| word_taint(a, uses, taints, ctx))
            .collect();
        return Some(crate::taint_interproc::apply_proc_return_summary(
            summary,
            &arg_taints,
        ));
    }

    // Legacy single-passthrough rule (no solve summaries available).
    let interproc = ctx.interproc?;
    let target = crate::interprocedural::resolve_internal_call(command, caller, known)?;
    let summary = interproc.procedures.get(&target)?;
    let passthrough = summary.return_passthrough_param.as_ref()?;
    let idx = summary.params.iter().position(|p| p == passthrough)?;
    let actual = args.get(idx)?;
    Some(word_taint(actual, uses, taints, ctx))
}

/// Whether this call's callee exposes `defined_name` to external input via a
/// registry-typed instance configuration, including transitive callees.
fn interproc_call_taints_global(command: &str, defined_name: &str, ctx: TaintCtx<'_>) -> bool {
    let (Some(known), Some(interproc)) = (ctx.known_procs, ctx.interproc) else {
        return false;
    };
    let caller = ctx.caller_qname.unwrap_or("::top");
    crate::interprocedural::resolve_internal_call(command, caller, known)
        .and_then(|target| interproc.tainted_global_writes.get(&target))
        .is_some_and(|writes| writes.contains(defined_name))
}

/// Look up taint for a named variable at its current SSA version. A name not
/// interned in `ssa` is not a tracked SSA variable, so it is clean.
fn var_taint<S: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ssa: &SsaFunction,
) -> TaintLattice {
    let Some(sym) = ssa.var_symbol(name) else {
        return TaintLattice::clean();
    };
    // Version 0 means the variable may be read from enclosing scope.
    let ver = uses.get(&sym).copied().unwrap_or(0);
    taints
        .get(&(sym, ver))
        .copied()
        .unwrap_or(TaintLattice::clean())
}

/// Join the taint contributed by every command substitution embedded in an
/// expression AST (`[cmd …]` retained as [`ExprNode::Command`], or `[cmd …]`
/// inside a quoted `ExprNode::String`).
///
/// Variable references are intentionally *not* re-scanned here — they are
/// already covered by the `AssignExpr` statement's SSA `uses` join — so this
/// adds only the command-substitution taint that variable joining misses.
fn expr_command_taint<S: std::hash::BuildHasher>(
    node: &ExprNode,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> TaintLattice {
    // Native-stack safety net (issue #996): this walks the `ExprNode`
    // operator tree, one native frame per level. Past the cap, assume
    // tainted — the conservative direction, so a command substitution nested
    // deeper than we can walk can't launder taint into a false negative.
    // (Calls into `word_taint` below re-enter that walker at bracket-depth 0:
    // a leaf node's raw text is a separate, independent recursion axis.)
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return TaintLattice::tainted();
    }
    match node {
        // A bracketed command substitution is classified exactly as a word; an
        // unparseable `Raw` fallback is treated the same way, scanning its text
        // for `[cmd]` substitutions.
        ExprNode::Command { text, .. } | ExprNode::Raw { text } => {
            word_taint(text, uses, taints, ctx)
        }
        // A quoted string undergoes substitution, so an embedded `[cmd]`
        // counts; a braced `{…}` string is literal (no substitution) and is
        // skipped to avoid a false positive.
        ExprNode::String { text, .. } => {
            if text.trim_start().starts_with('"') {
                word_taint(text, uses, taints, ctx)
            } else {
                TaintLattice::clean()
            }
        }
        ExprNode::Binary { left, right, .. } => expr_command_taint(
            left,
            uses,
            taints,
            ctx,
            depth + 1,
        )
        .join(expr_command_taint(right, uses, taints, ctx, depth + 1)),
        ExprNode::Unary { operand, .. } => {
            expr_command_taint(operand, uses, taints, ctx, depth + 1)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => expr_command_taint(condition, uses, taints, ctx, depth + 1)
            .join(expr_command_taint(
                true_branch,
                uses,
                taints,
                ctx,
                depth + 1,
            ))
            .join(expr_command_taint(
                false_branch,
                uses,
                taints,
                ctx,
                depth + 1,
            )),
        ExprNode::Call { args, .. } => args.iter().fold(TaintLattice::clean(), |acc, a| {
            acc.join(expr_command_taint(a, uses, taints, ctx, depth + 1))
        }),
        // Literals and variable references carry no command substitution
        // (variables are covered by the statement's `uses` join).
        ExprNode::Literal { .. } | ExprNode::Var { .. } => TaintLattice::clean(),
    }
}

/// Determine the taint produced by a statement's definition(s).
fn evaluate_taint_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    defined_var: Symbol,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    ssa: &SsaFunction,
) -> TaintLattice {
    let ctx = ctx.at(stmt.span().start());
    match stmt {
        // Expression: join taint from all used variables AND from any command
        // substitution embedded in the expression AST. `join_uses` only sees
        // `$var` SSA uses, so a taint source nested in an `[expr {…}]` command
        // substitution (`set x [expr {[gets stdin] + 1}]`) would otherwise
        // launder the taint — a false-negative security diagnostic.
        Statement::AssignExpr { expr, .. } => {
            join_uses(uses, taints, ssa).join(expr_command_taint(expr, uses, taints, ctx, 0))
        }

        // Value assignment: evaluate the RHS word.
        Statement::AssignValue { value, .. } => word_taint(value, uses, taints, ctx),

        // incr propagates taint from the variable being incremented.
        Statement::Incr { name, .. } => {
            let base = normalise_var_name(name);
            var_taint(base, uses, taints, ssa)
        }

        // Generic call that defines variables.
        Statement::Call {
            command,
            args,
            defs,
            ..
        } if !defs.is_empty() => {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let defined_name = ssa.var_name(defined_var);
            if tcl_registry::taint::taints_var_write(
                ctx.registry,
                command,
                &arg_refs,
                dialect_to_set(ctx.dialect),
                defined_name,
            ) || instance_taints_var_write(
                ctx.registry,
                command,
                &arg_refs,
                ctx.dialect,
                ctx.instance_classes,
                ctx.source_position,
                defined_name,
            ) {
                return TaintLattice::tainted();
            }
            if interproc_call_taints_global(command, defined_name, ctx) {
                return TaintLattice::tainted();
            }
            if is_sanitiser(ctx.registry, command, &arg_refs) {
                return TaintLattice::clean();
            }
            if let Some(t) = source_colour(
                ctx.registry,
                command,
                &arg_refs,
                ctx.dialect,
                ctx.instance_classes,
                ctx.source_position,
            ) {
                return t;
            }
            if let Some(t) = interproc_call_taint(command, args, uses, taints, ctx) {
                return t;
            }
            // Propagate from arguments.
            let mut t = TaintLattice::clean();
            for arg in args {
                t = t.join(word_taint(arg, uses, taints, ctx));
            }
            t
        }

        // Barrier has unknown semantics — propagate taint conservatively
        // from its arguments so attacker-influenced data flowing through
        // an opaque command taints any output variables it defines.
        Statement::Barrier { args, .. } => {
            let mut t = TaintLattice::clean();
            for arg in args {
                t = t.join(word_taint(arg, uses, taints, ctx));
            }
            t
        }

        // `AssignConst` and any unhandled statement variant (e.g.
        // structured control flow that survives in some code paths) are
        // treated as clean: constants cannot introduce taint, and
        // opaque control-flow statements have no value definition to
        // propagate taint through.
        _ => TaintLattice::clean(),
    }
}

/// Apply rendered-property-derived colours to a taint lattice.
///
/// Only adds mitigating colours when the rendered-properties lattice
/// provides *positive* evidence for them. A leading `/` (must-bit
/// `STARTS_WITH_SLASH`) proves both `PATH_PREFIXED` and
/// `NON_DASH_PREFIXED`. We deliberately do *not* infer
/// `NON_DASH_PREFIXED` or `CRLF_FREE` from the absence of bits,
/// because phi-joins and unknown command substitutions lose those
/// facts without proving the value is safe — inferring from absence
/// would unsoundly suppress T102 / CRLF-injection warnings.
fn colour_from_rendered(lat: TaintLattice, props: RenderedValueProps) -> TaintLattice {
    let mut out = lat;
    if props.must.contains(RenderedProperties::STARTS_WITH_SLASH) {
        out = out.with(TaintColour::PATH_PREFIXED);
        out = out.with(TaintColour::NON_DASH_PREFIXED);
    }
    out
}

/// True when a version-0 SSA use should be skipped by the *reporting*
/// passes as the conservative global-write taint seeding rather
/// than a genuine taint.
///
/// Version-0 (`(name, 0)`) taints arise from two sources: the
/// interprocedural solve's parameter entry-taint (a genuine cross-proc
/// flow) and the conservative cross-procedure global-write seeding
/// ([`collect_global_reads`], which only ever seeds `::`-prefixed global
/// / namespace names). The latter is not a genuine taint, so a version-0
/// use is suppressed only when its name is global/namespace-scoped; a
/// non-`::` version-0 use is a real parameter entry-taint and is
/// reported.
fn is_seeded_global_v0(name: &str, ver: u32) -> bool {
    ver == 0 && name.starts_with("::")
}

/// Join taint from all SSA uses in a statement.
///
/// Version-0 uses contribute when they carry a genuine parameter
/// entry-taint (non-`::` name): the join takes `(name, 0)` when
/// present in the taint map. The conservative
/// global-write seeding (`::` names at version 0) is excluded so it does
/// not over-propagate into expression results.
fn join_uses<S: std::hash::BuildHasher>(
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ssa: &SsaFunction,
) -> TaintLattice {
    let mut t = TaintLattice::clean();
    for (&sym, &ver) in uses {
        if ver > 0 {
            t = t.join(
                taints
                    .get(&(sym, ver))
                    .copied()
                    .unwrap_or(TaintLattice::clean()),
            );
        } else if !ssa.var_name(sym).starts_with("::")
            && let Some(&v0) = taints.get(&(sym, 0))
        {
            t = t.join(v0);
        }
    }
    t
}

/// True when any procedure reachable from the current function (the
/// caller named `ssa.name`) has `writes_global = true`.
///
/// For a known proc, uses its summary's transitive `calls` list. For
/// `::top` or an unknown caller, enumerates direct callees from the
/// function's IR via `Statement::Call` commands that resolve to an
/// internal proc, then unions their transitive closures.
fn reachable_writes_global(
    ssa: &SsaFunction,
    cfg: &CfgFunction,
    ia: &InterproceduralAnalysis,
) -> bool {
    // If the function itself is in the summary, use its transitive
    // closure directly.
    if let Some(self_summary) = ia.procedures.get(ssa.name.as_str()) {
        if self_summary.writes_global {
            return true;
        }
        return self_summary
            .calls
            .iter()
            .any(|c| ia.procedures.get(c).is_some_and(|s| s.writes_global));
    }

    // Unknown / top-level caller: walk the CFG for direct Call
    // targets, resolve them, and union the transitive closures.
    let known: HashSet<String> = ia.procedures.keys().cloned().collect();
    let mut visited: FxHashSet<String> = FxHashSet::default();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { command, .. } = stmt
                && let Some(target) = crate::interprocedural::resolve_internal_call(
                    command,
                    ssa.name.as_str(),
                    &known,
                )
                && let Some(summary) = ia.procedures.get(&target)
            {
                if summary.writes_global {
                    return true;
                }
                if visited.insert(target) {
                    for c in &summary.calls {
                        if ia.procedures.get(c).is_some_and(|s| s.writes_global) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Collect the set of `::`-prefixed (global / namespace-scoped)
/// variable names that the function reads at SSA version 0.
///
/// Scans every block's `entry_versions`, every statement's `uses`
/// map, and every phi's incoming edges so we catch globals reached
/// from any predecessor — the entry block alone typically has no
/// seeded versions for globals.
fn collect_global_reads(ssa: &SsaFunction) -> FxHashSet<String> {
    let mut out: FxHashSet<String> = FxHashSet::default();
    let mut consider = |name: &str| {
        if name.starts_with("::") {
            out.insert(name.to_owned());
        }
    };
    for block in ssa.blocks.values() {
        for &sym in block.entry_versions.keys() {
            consider(ssa.var_name(sym));
        }
        for phi in &block.phis {
            if phi.incoming.values().any(|&v| v == 0) {
                consider(ssa.var_name(phi.name));
            }
        }
        for stmt in &block.statements {
            for (&sym, &ver) in &stmt.uses {
                if ver == 0 {
                    consider(ssa.var_name(sym));
                }
            }
        }
    }
    out
}

/// One function's CFG/SSA analyses plus the CFG-derived indices
/// [`propagate_taints`] walks, built once and reused across every
/// propagation over that function.
///
/// Both indices are pure functions of the CFG, so rebuilding them per
/// propagation is wasted work — and it was not a small amount of it:
/// [`crate::taint_interproc::infer_proc_summary`] runs one propagation for the
/// baseline plus one per (parameter, basis) scenario, and the summary
/// worklist re-infers a procedure whenever a callee's summary moves, so an
/// O(V+E) predecessor map and an O(V+E) reverse-postorder were being rebuilt
/// dozens of times per function (issue #1251).
pub(crate) struct TaintGraph<'a> {
    /// The function's CFG.
    pub cfg: &'a CfgFunction,
    /// Its SSA form.
    pub ssa: &'a SsaFunction,
    /// The SCCP result gating block/edge executability.
    pub sccp: &'a SccpResult,
    /// Predecessor map for the phi joins, `rustc-hash`-keyed.
    preds: FxHashMap<BlockId, FxHashSet<BlockId>>,
    /// Reverse-postorder block visit order for the fixpoint sweep.
    order: Vec<BlockId>,
}

impl<'a> TaintGraph<'a> {
    /// Build the per-function indices once.
    pub fn new(cfg: &'a CfgFunction, ssa: &'a SsaFunction, sccp: &'a SccpResult) -> Self {
        Self {
            cfg,
            ssa,
            sccp,
            preds: cfg.predecessors_fx(),
            order: cfg_order(cfg),
        }
    }
}

/// Run intra-procedural taint propagation over one SSA function.
///
/// Returns a map from `(variable_name, ssa_version)` to its taint
/// lattice value. Entries absent from the map are implicitly clean.
///
/// Sources are identified by [`is_taint_source`]. Propagation follows
/// SSA phi-join semantics with SCCP edge-level reachability: a phi
/// predecessor only contributes if its incoming CFG edge is executable,
/// preventing taint from propagating through SCCP-proven dead branches.
///
/// Optional inputs:
/// * `rendered_props` — when present, values are coloured using
///   rendered-string properties (`PATH_PREFIXED`, `NON_DASH_PREFIXED`,
///   `CRLF_FREE`).
/// * `interproc` — when present, calls to known user procs apply the
///   proc summary's passthrough-parameter rule, and procs marked
///   `writes_global` taint the version-0 entry of every global in the
///   map (conservative).
/// * `dialect` — when `"f5-irules"` / `"irules"`, iRules-specific
///   namespace-prefixed commands (`HTTP::`, `URI::`, …) are treated
///   as taint sources.
/// * `param_taints` — entry taints seeded by the interprocedural
///   solve: each tainted entry seeds the version-0 slot of the named
///   parameter.
/// * `taint_summaries` — colour-aware return summaries from the
///   interprocedural solve; when present, internal calls apply the
///   full return-summary transfer (`apply_proc_return_summary`)
///   instead of the conservative single-passthrough rule.
/// * `instance_classes` — precomputed function-local receiver facts. The
///   summary solver reuses this across its clean run and every parameter ×
///   colour scenario instead of repeating the same receiver CFG fixpoint.
#[must_use]
// These inputs are the independently reusable analysis products; wrapping
// them in an options bag would hide, rather than reduce, the dependency set.
#[allow(clippy::too_many_arguments)]
pub(crate) fn propagate_taints(
    graph: &TaintGraph<'_>,
    registry: &CommandRegistry,
    rendered_props: Option<&HashMap<ValueKey, RenderedValueProps>>,
    interproc: Option<&InterproceduralAnalysis>,
    dialect: Option<&tcl_dialect::DialectProfile>,
    param_taints: Option<&HashMap<String, TaintLattice>>,
    taint_summaries: Option<&HashMap<String, crate::taint_interproc::ProcTaintSummary>>,
    instance_classes: &LocalInstanceClasses,
) -> HashMap<ValueKey, TaintLattice> {
    let TaintGraph {
        cfg,
        ssa,
        sccp,
        preds,
        order,
    } = graph;

    // Precompute the set of known procedure names once so per-call
    // resolution in `interproc_call_taint` is O(1) rather than
    // O(procedures) per call site. The solve's summaries take
    // precedence (their key set).
    let known_procs: Option<HashSet<String>> = match (taint_summaries, interproc) {
        (Some(s), _) => Some(s.keys().cloned().collect()),
        (None, Some(ia)) => Some(ia.procedures.keys().cloned().collect()),
        (None, None) => None,
    };
    let ctx = TaintCtx {
        registry,
        ssa,
        interproc,
        known_procs: known_procs.as_ref(),
        caller_qname: Some(ssa.name.as_str()),
        dialect,
        taint_summaries,
        instance_classes: Some(instance_classes),
        source_position: None,
    };

    let mut taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
    seed_entry_taints(&mut taints, ssa, cfg, interproc, param_taints, dialect);

    let mut changed = true;
    while changed {
        changed = false;
        for bn in order {
            if !sccp.executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };
            changed |= propagate_phi_taints(&mut taints, ssa_block, *bn, preds, sccp);
            changed |= propagate_statement_taints(&mut taints, ssa_block, ctx, ssa, rendered_props);
        }
    }

    taints
}

/// Seed the initial taint map: tainted interprocedural parameters, plus
/// version-0 global reads when a reachable callee writes globals.
fn seed_entry_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    ssa: &SsaFunction,
    cfg: &CfgFunction,
    interproc: Option<&InterproceduralAnalysis>,
    param_taints: Option<&HashMap<String, TaintLattice>>,
    dialect: Option<&tcl_dialect::DialectProfile>,
) {
    // Seed entry taints for tainted parameters (interprocedural solve).
    // Only tainted params seed a slot; clean params leave the version-0
    // slot absent (implicitly clean).
    if let Some(pt) = param_taints {
        for (name, t) in pt {
            // A tainted param only matters if the body reads it, and any read
            // interns the name; a never-read param's seed slot would never be
            // consulted, so skipping an un-interned name is behaviour-neutral.
            if t.is_tainted()
                && let Some(sym) = ssa.var_symbol(name)
            {
                taints.insert((sym, 0), *t);
            }
        }
    }

    // Seed: when a callee reachable from the current function writes
    // to global scope, taint version-0 reads of global/namespace
    // variables that this function actually touches. Scoping to
    // reachable callees prevents an unrelated helper proc's global
    // writes from polluting functions that never invoke it; scanning
    // *every* block's entry_versions (plus statement uses / phi
    // incomings) ensures we discover globals even when the entry
    // block has no seeded versions.
    if let Some(ia) = interproc
        && reachable_writes_global(ssa, cfg, ia)
    {
        let globals = collect_global_reads(ssa);
        for name in globals {
            // `collect_global_reads` only returns names that appear in the SSA
            // (entry versions / phi names / statement uses), so each is interned.
            if let Some(sym) = ssa.var_symbol(&name) {
                taints.entry((sym, 0)).or_insert(TaintLattice::tainted());
            }
        }
    }

    // Interpreter-provided external-input globals (`env`, `argv`, `argv0`) are
    // taint sources: their version-0 (external) read is attacker-influenced.
    // The set is dialect-aware — the restricted iRules interpreter provides
    // none of them — and sourced from the special-variable registry. A later
    // local `set env …` writes a higher SSA version, so a read that resolves
    // to the local (version > 0) is unaffected; shadowing falls out of the SSA
    // versioning, not a check here.
    for spec in tcl_registry::special_vars::special_vars_for_dialect(
        tcl_registry::special_vars::dialect_set_for_profile(dialect),
    ) {
        let Some(colour) = spec.read_taint else {
            continue;
        };
        if let Some(sym) = ssa.var_symbol(spec.name) {
            taints.entry((sym, 0)).or_insert(TaintLattice {
                colours: reg_colour(colour),
            });
        }
        // Per-element SSA: `$env(PATH)` reads its own `env(PATH)` symbol —
        // every constant-keyed element of a tainted special array carries
        // the source colour at version 0 too.
        for name in ssa.var_names() {
            if name.len() > spec.name.len()
                && name.starts_with(spec.name)
                && name.as_bytes()[spec.name.len()] == b'('
                && let Some(sym) = ssa.var_symbol(name)
            {
                taints.entry((sym, 0)).or_insert(TaintLattice {
                    colours: reg_colour(colour),
                });
            }
        }
    }
}

/// Join phi taints from edge-executable predecessors into `taints` for one
/// block. Returns `true` if any lattice value changed.
fn propagate_phi_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    ssa_block: &crate::ssa::SsaBlock,
    bn: BlockId,
    preds: &FxHashMap<BlockId, FxHashSet<BlockId>>,
    sccp: &SccpResult,
) -> bool {
    let mut changed = false;
    // Phi nodes: join taint from edge-executable predecessors only.
    // Using executable_edges (not just executable_blocks) ensures
    // taint does not flow through SCCP-proven dead branches.
    for phi in &ssa_block.phis {
        let exec_preds = preds
            .get(&bn)
            .map(|ps| {
                ps.iter()
                    .filter(|p| sccp.executable_edges.contains(&(**p, bn)))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if exec_preds.is_empty() {
            continue;
        }

        // Use `Option` to distinguish "no predecessor seen yet"
        // from "clean with no mitigations". The join's intersect
        // semantics for mitigations needs an identity of
        // "all mitigations set" that we don't have a sentinel
        // for; instead, the first incoming value becomes the
        // accumulator directly and subsequent ones join into it.
        let mut phi_taint: Option<TaintLattice> = None;
        for pred in exec_preds {
            let ver = phi.incoming.get(pred).copied().unwrap_or(0);
            // Include version-0 incomings: they represent
            // enclosing-scope reads (possibly pre-seeded with
            // taint when a reachable callee writes globals).
            let incoming = taints
                .get(&(phi.name, ver))
                .copied()
                .unwrap_or(TaintLattice::clean());
            phi_taint = Some(match phi_taint {
                Some(existing) => existing.join(incoming),
                None => incoming,
            });
        }

        let Some(phi_taint) = phi_taint else { continue };
        let key = (phi.name, phi.version);
        let merged = match taints.get(&key) {
            Some(&old) => old.join(phi_taint),
            None => phi_taint,
        };
        if taints.get(&key) != Some(&merged) {
            taints.insert(key, merged);
            changed = true;
        }
    }
    changed
}

/// Apply the per-statement taint transfer for one block's statements,
/// merging results into `taints`. Returns `true` if any value changed.
fn propagate_statement_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    ssa_block: &crate::ssa::SsaBlock,
    ctx: TaintCtx<'_>,
    ssa: &SsaFunction,
    rendered_props: Option<&HashMap<ValueKey, RenderedValueProps>>,
) -> bool {
    let mut changed = false;
    for ssa_stmt in &ssa_block.statements {
        let stmt = &ssa_stmt.statement;
        for (&var, &ver) in &ssa_stmt.defs {
            let mut inferred = evaluate_taint_def(stmt, var, &ssa_stmt.uses, &*taints, ctx, ssa);
            // Enrich the inferred taint with rendered-property
            // colours when available.
            if let Some(rp) = rendered_props
                && let Some(p) = rp.get(&(var, ver))
            {
                inferred = colour_from_rendered(inferred, *p);
            }
            let key = (var, ver);
            let merged = match taints.get(&key) {
                Some(&old) => old.join(inferred),
                None => inferred,
            };
            if taints.get(&key) != Some(&merged) {
                taints.insert(key, merged);
                changed = true;
            }
        }
    }
    changed
}

// Sink detection

/// Return the diagnostic code and human-readable sink label for a
/// statement that acts as a taint sink, or `None` if the statement is
/// not a sink.
///
/// Single registry-driven classification via
/// [`tcl_registry::taint::classify_taint_sinks`] — no command name is
/// matched here. The registry's most specific fact wins: a command that
/// declares an explicit output/log sink code (`puts` → `taint_output_sink
/// = "T101"`, `HTTP::respond` → `"IRULE3001"`, `log` → `taint_log_sink =
/// "IRULE3003"`) is classified as that sink, even when it also carries the
/// generic `TAINT_SINK` trait; only a command with *no* declared sink code
/// falls back to the trait-driven **T100** code-execution classification
/// (`EVALUATES_CODE` / `TAINT_SINK` — `eval`, `exec`, `uplevel`, `subst`,
/// `expr`).
///
/// `classify_taint_sinks` is asked with an empty dialect (no
/// `supports_dialect` filtering) so a sink the active profile *bans*
/// (`exec` under f5-irules — excluded by the §9 disable list, not by any
/// mask) keeps its
/// T100 classification: a banned command that appears in source anyway is
/// still a code-execution sink. The iRules-only sink codes (`IRULE…`, from
/// `taint_output_sink` / `taint_log_sink` on iRules specs) are instead
/// gated explicitly on the active document dialect via
/// [`DiagCode::family`] — the same explicit `is_irules_dialect` gate
/// [`find_setter_constraint_warnings`] uses for IRULE3101, rather than
/// relying on whether the shared registry happens to have those specs
/// loaded.
///
/// Before any of that, a command declaring
/// [`tcl_registry::CommandSpec::taint_sink_gate`] gets one more check: when
/// this call's own option flags disable its hazard (`subst -nocommands`),
/// the call is not a sink at all, regardless of which code it would
/// otherwise have classified as.
fn classify_sink(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Option<(DiagCode, String)> {
    let canonical_command = registry.get(command).map_or(command, |spec| spec.name);
    // A call whose own option flags disable this command's hazard for
    // *this* invocation (e.g. `subst -nocommands $tainted`) is not a sink
    // at all — checked before any classification below.
    if let Some(spec) = registry.get(command)
        && let Some(gate) = spec.taint_sink_gate
    {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        if !gate(&arg_refs) {
            return None;
        }
    }

    let subcommand = args.first().map(String::as_str);
    let info = tcl_registry::taint::classify_taint_sinks(
        registry,
        command,
        subcommand,
        DialectSet::empty(),
    );

    if let Some(code_str) = info.output_sink.or(info.log_sink) {
        let code: DiagCode = code_str.parse().unwrap_or_else(|_| {
            panic!("registry declared unknown sink code {code_str:?} for {command}")
        });
        if code.family() == DiagFamily::IRule && !is_irules_dialect(dialect) {
            return None;
        }
        let label = if info.output_sink_is_subcommand_qualified {
            let canonical = subcommand
                .and_then(|s| {
                    registry
                        .get(command)
                        .and_then(|spec| spec.resolve_subcommand(s))
                })
                .map_or_else(|| subcommand.unwrap_or_default(), |sub| sub.name);
            format!("{canonical_command} {canonical}")
        } else {
            canonical_command.to_owned()
        };
        return Some((code, label));
    }

    if info.is_code_sink {
        return Some((DiagCode::T100, canonical_command.to_owned()));
    }

    None
}

/// Registry-driven SSRF / cross-interpreter sinks (T104 / T105),
/// returned in addition to the primary [`classify_sink`] match so a
/// single statement can trip multiple categories:
///
/// * **T104** — the command's spec carries `taint_network_sink_args`
///   (a network-address argument → SSRF risk; e.g. `socket`).
/// * **T105** — the first argument names a subcommand in the spec's
///   `taint_interp_eval_subcommands` (cross-interpreter code execution;
///   e.g. `interp eval` / `interp invokehidden`).
fn classify_network_interp_sinks(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<(DiagCode, String)> {
    let mut out = Vec::new();
    let Some(spec) = registry.get(command) else {
        return out;
    };
    if spec.taint_network_sink_args.is_some() {
        out.push((DiagCode::T104, spec.name.to_owned()));
    }
    // Resolve the subcommand prefix-aware (`interp ev` ⇒ `eval`) before testing
    // membership, so a legal abbreviation does not dodge the cross-interpreter
    // eval classification.
    if let Some(sub) = args.first() {
        let canonical = spec
            .resolve_subcommand(sub)
            .map_or(sub.as_str(), |s| s.name);
        if spec.taint_interp_eval_subcommands.contains(&canonical) {
            out.push((DiagCode::T105, format!("{} {canonical}", spec.name)));
        }
    }
    out
}

/// **W313.** Flag destructive `file` operations (`file delete` /
/// `rename` / `mkdir` — the registry's `destructive` subcommands) whose
/// path argument is a variable, since the path may carry user-controlled
/// content (path-traversal).  Suppressed when the path variable is both
/// normalised (`PATH_NORMALISED` colour, or its SSA def is
/// `[file normalize …]`) *and* bounds-checked — the latter via
/// branch-guard analysis ([`compute_branch_guard_map`]) or a
/// `PATH_BOUNDED` colour.  The message is softened (not suppressed) for a
/// normalised-but-unguarded path.
#[must_use]
pub fn find_destructive_file_warnings<S: std::hash::BuildHasher, E: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    let destructive = destructive_file_subs(registry);
    if destructive.is_empty() {
        return Vec::new();
    }
    // The document's `${…}` close rule, threaded into the path-variable scan
    // so the name W313 anchors on is the one the lexer spanned (issue #1604).
    let braced_var = tcl_dialect::BracedVarStyle::of_profile(dialect);
    let guard_map = compute_branch_guard_map(cfg, registry);

    let mut warnings: Vec<TaintWarning> = Vec::new();
    for bn in cfg_order(cfg) {
        if !executable_blocks.contains(&bn) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&bn) else {
            continue;
        };
        let empty = HashSet::new();
        let block_guarded = guard_map.get(&bn).unwrap_or(&empty);
        for ssa_stmt in &ssa_block.statements {
            let Statement::Call {
                command,
                canonical_command,
                args,
                span,
                ..
            } = &ssa_stmt.statement
            else {
                continue;
            };
            if canonical_command.as_deref() != Some("::file") && command != "file" {
                continue;
            }
            let Some(sub) = args.first() else {
                continue;
            };
            if !destructive.contains(sub.as_str()) {
                continue;
            }
            // Skip `-force` / `--` to find the path arguments.
            let mut path_start = 1;
            while path_start < args.len() && matches!(args[path_start].as_str(), "-force" | "--") {
                path_start += 1;
            }
            // One W313 per statement (first offending path variable).
            // Collect candidate path variables in argument (source) order —
            // tagged with their argument index so the warning can anchor at
            // the offending *path argument*, not the whole command:
            // `ssa_stmt.uses` is a `HashMap`, so iterating it would pick a
            // nondeterministic variable for a multi-path sink like
            // `file rename $a $b` — making the warning's message (and the memo
            // vs whole-module builds) differ run-to-run.
            let mut seen: FxHashSet<String> = FxHashSet::default();
            let mut ordered: Vec<(usize, String)> = Vec::new();
            for (idx, a) in args.iter().enumerate().skip(path_start) {
                for name in arg_var_names_ordered(a, braced_var) {
                    if seen.insert(name.clone()) {
                        ordered.push((idx, name));
                    }
                }
            }
            for (arg_idx, name) in &ordered {
                let Some(sym) = ssa.var_symbol(name) else {
                    continue;
                };
                let Some(&ver) = ssa_stmt.uses.get(&sym) else {
                    continue;
                };
                let t = taints
                    .get(&(sym, ver))
                    .copied()
                    .unwrap_or(TaintLattice::clean());
                let is_normalised = (t.is_tainted()
                    && t.colours.contains(TaintColour::PATH_NORMALISED))
                    || is_normalised_def(name, ver, ssa);
                let is_bounded = (t.is_tainted() && t.colours.contains(TaintColour::PATH_BOUNDED))
                    || (is_normalised && block_guarded.contains(name));
                if is_bounded {
                    continue;
                }
                let message = if is_normalised {
                    format!(
                        "file {sub} with normalised path (${name}) — verify it stays \
                         within the intended directory (e.g. [string match \"$base/*\" \
                         ${{{name}}}])."
                    )
                } else {
                    format!(
                        "file {sub} with a variable path (${name}) risks path-traversal. \
                         Normalise with [file normalize] and verify it stays within the \
                         intended directory."
                    )
                };
                warnings.push(TaintWarning {
                    // Anchor at the offending path argument's own token span
                    // (mirroring T102's per-argument targeting); the whole
                    // statement span is the tokens-unavailable fallback.
                    span: arg_index_span(&ssa_stmt.statement, *arg_idx).unwrap_or(*span),
                    variable: name.clone(),
                    sink_command: format!("file {sub}"),
                    code: DiagCode::W313,
                    message,
                    replacement: None,
                    fixes: Vec::new(),
                });
                break;
            }
        }
    }
    warnings
}

/// The `file` subcommands flagged `destructive` in the registry
/// (`delete` / `rename` / `mkdir`).
fn destructive_file_subs(registry: &CommandRegistry) -> HashSet<&'static str> {
    registry.get("file").map_or_else(HashSet::new, |spec| {
        spec.subcommands
            .iter()
            .filter(|s| s.destructive)
            .map(|s| s.name)
            .collect()
    })
}

/// Variable names referenced (via `$name` / `${name}`) anywhere in `arg`.
/// A lightweight VAR-token scan covering the path-argument shapes W313
/// cares about (`$p`, `${p}`, `"$d/$f"`).
///
/// Deliberately **not** `crate::var_refs::vars_in_word` — the two disagree on
/// a variable nested inside another's array index (`$arr($cmd)`): the real
/// lexer treats the whole thing as one opaque `Var` token, so `vars_in_word`
/// only ever surfaces `arr`, while this byte-scan (blind to token structure)
/// finds `cmd` too. Centralising on `vars_in_word` would drop that name from
/// W313's variable set — a false-negative regression — so this stays a
/// bespoke scanner; see the `arg_var_names_vs_vars_in_word` test module for
/// the full comparison.
fn arg_var_names(arg: &str, braced_var: tcl_dialect::BracedVarStyle) -> HashSet<String> {
    arg_var_names_ordered(arg, braced_var).into_iter().collect()
}

/// Variable names referenced in `arg`, in left-to-right source order with
/// duplicates preserved.  Callers that need a deterministic *first* variable
/// (e.g. W313's "first offending path variable") iterate this rather than the
/// `HashSet` from [`arg_var_names`], whose order is nondeterministic.
fn arg_var_names_ordered(arg: &str, braced_var: tcl_dialect::BracedVarStyle) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = arg.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // A backslash escapes the next character (`\$notavar` is the
            // literal text `$notavar`, not a variable reference) — mirrors
            // the lexer's `\X` handling for un-braced words. Skipping the
            // pair keeps an escaped `\$` from being misread as the start of
            // a substitution.
            let esc_len = arg[i + 1..].chars().next().map_or(0, char::len_utf8);
            i += 1 + esc_len;
            continue;
        }
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        // The `${…}` name starts just past the `${`; its closer comes from the
        // shared owner under this document's release rule. Taint is keyed by
        // name, so recovering `a{b` where the lexer spanned `a{b}c` tracks a
        // cell nothing writes — the tainted value's colour is lost and the
        // sink goes unflagged (issue #1604).
        if bytes.get(i + 1) == Some(&b'{')
            && let tcl_lexer::BracedVarEnd::Closed(end) =
                tcl_lexer::braced_var_name_end(bytes, i + 2, braced_var)
        {
            names.push(
                tcl_syntax::naming::normalise_var_name_for_style(&arg[i + 2..end], braced_var)
                    .to_owned(),
            );
            i = end + 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len()
            && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b':')
        {
            j += 1;
        }
        if j > start {
            names.push(normalise_var_name(&arg[start..j]).to_owned());
            i = j;
        } else {
            i += 1;
        }
    }
    names
}

/// True when the SSA def of `name`@`ver` is a `[file normalize …]`
/// command substitution.
fn is_normalised_def(name: &str, ver: u32, ssa: &SsaFunction) -> bool {
    let Some(sym) = ssa.var_symbol(name) else {
        return false;
    };
    for ssa_block in ssa.blocks.values() {
        for ssa_stmt in &ssa_block.statements {
            if ssa_stmt.defs.get(&sym) == Some(&ver) {
                if let Statement::AssignValue { value, .. } = &ssa_stmt.statement {
                    return value.trim().starts_with("[file normalize ");
                }
                return false;
            }
        }
    }
    false
}

/// Build a `block → {bounds-checked var}` map: a `Branch` whose condition
/// is `[string match|first|equal … $var]` marks `$var` as `PATH_BOUNDED`
/// in the guarded successor (and its exclusive successors).
fn compute_branch_guard_map(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
) -> HashMap<BlockId, HashSet<String>> {
    let mut guarded: HashMap<BlockId, HashSet<String>> = HashMap::new();
    for block in cfg.blocks.values() {
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let (negated, var) = extract_guard_var(condition, false, 0);
        let Some(var) = var else {
            continue;
        };
        let (guarded_target, other_target) = if negated {
            (*false_target, *true_target)
        } else {
            (*true_target, *false_target)
        };
        propagate_guard(
            cfg,
            guarded_target,
            other_target,
            &var,
            registry,
            &mut guarded,
        );
    }
    guarded
}

/// Extract `(negated, path-var)` from a branch condition: a unary
/// operator flips negation; a binary operator checks both sides; a
/// `[string …]` command sub yields the bounds-checked variable.
fn extract_guard_var(expr: &ExprNode, negated: bool, depth: u32) -> (bool, Option<String>) {
    // Native-stack safety net (issue #996): this walks the `ExprNode` tree,
    // one native frame per level. Past the cap, report "no guard variable
    // found" — the same conservative result as the leaf `_` arm, so no
    // read-narrowing is applied where the condition was too deep to analyse.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return (negated, None);
    }
    match expr {
        ExprNode::Unary { operand, .. } => extract_guard_var(operand, !negated, depth + 1),
        ExprNode::Binary { left, right, .. } => {
            let (n, r) = extract_guard_var(left, negated, depth + 1);
            if r.is_some() {
                return (n, r);
            }
            extract_guard_var(right, negated, depth + 1)
        }
        ExprNode::Command { text, .. } => (negated, guard_var_from_string_command(text)),
        _ => (negated, None),
    }
}

/// Parse `[string match|first|equal … $var]` and return the path
/// variable it bounds-checks.
fn guard_var_from_string_command(text: &str) -> Option<String> {
    let (command, args) = parse_command_substitution(text.trim())?;
    if command != "string" || args.is_empty() {
        return None;
    }
    match args[0].as_str() {
        "match" if args.len() >= 3 => extract_var_name(args.last().unwrap()),
        "first" if args.len() >= 3 => extract_var_name(&args[2]),
        "equal" => {
            let mut skip_next = false;
            for arg in &args[1..] {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if arg == "-length" {
                    skip_next = true;
                    continue;
                }
                if arg.starts_with('-') {
                    continue;
                }
                if let Some(name) = extract_var_name(arg) {
                    return Some(name);
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract a variable name from a `$name` / `${name}` word (identifier
/// chars only — no `::` qualification).
fn extract_var_name(arg: &str) -> Option<String> {
    let text = arg.trim();
    if let Some(inner) = text.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return Some(inner.to_owned());
    }
    if let Some(name) = text.strip_prefix('$')
        && !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Some(name.to_owned());
    }
    None
}

/// True when block `id` executes a block-terminating command
/// (`error` / `return` / `exit` — the `TERMINATES_BLOCK` trait) so its
/// successors are unreachable from this path.
fn is_dead_end_block(cfg: &CfgFunction, id: BlockId, registry: &CommandRegistry) -> bool {
    let Some(block) = cfg.blocks.get(&id) else {
        return false;
    };
    block.statements.iter().any(|stmt| {
        matches!(stmt, Statement::Call { command, .. }
            if registry.get(command).is_some_and(|s| s.traits.contains(Traits::TERMINATES_BLOCK)))
    })
}

/// Mark `guarded_target` and its successors that aren't also reachable
/// from `other_target` (i.e. before the merge point) as guarding `var`.
/// When `other_target` is a dead-end (error/return), the merge is only
/// reachable through the guard, so the guard extends through it.
fn propagate_guard(
    cfg: &CfgFunction,
    guarded_target: BlockId,
    other_target: BlockId,
    var: &str,
    registry: &CommandRegistry,
    guarded: &mut HashMap<BlockId, HashSet<String>>,
) {
    let mut other_reachable: HashSet<BlockId> = HashSet::new();
    if !is_dead_end_block(cfg, other_target, registry) {
        let mut stack = vec![other_target];
        while let Some(b) = stack.pop() {
            if other_reachable.contains(&b) {
                continue;
            }
            let Some(block) = cfg.blocks.get(&b) else {
                continue;
            };
            other_reachable.insert(b);
            for succ in block.terminator.iter().flat_map(Terminator::successors) {
                stack.push(succ);
            }
        }
    }
    let mut visit = vec![guarded_target];
    let mut visited: HashSet<BlockId> = HashSet::new();
    while let Some(b) = visit.pop() {
        if visited.contains(&b) {
            continue;
        }
        if other_reachable.contains(&b) && b != guarded_target {
            continue;
        }
        let Some(block) = cfg.blocks.get(&b) else {
            continue;
        };
        visited.insert(b);
        guarded.entry(b).or_default().insert(var.to_owned());
        for succ in block.terminator.iter().flat_map(Terminator::successors) {
            visit.push(succ);
        }
    }
}

/// **T106.** Emit a double-encoding warning when a tainted value that
/// already carries a command's `taint_double_encode_colour` is passed
/// through that command again (e.g. `uri::encode [uri::encode $x]`).
/// Emit T106 (double-encode) warnings — one warning per variable.
#[allow(clippy::too_many_arguments)]
fn emit_double_encode_warnings<S: std::hash::BuildHasher>(
    command: &str,
    call_args: &[String],
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    span: Span,
    warnings: &mut Vec<TaintWarning>,
    ctx: TaintCtx<'_>,
    quoted_uses: &HashSet<Symbol>,
) {
    let Some(dup_colour) = ctx
        .registry
        .get(command)
        .and_then(|s| s.taint_double_encode_colour)
        .map(reg_colour)
    else {
        return;
    };
    let label = double_encode_label(dup_colour);
    let mut emitted: FxHashSet<Symbol> = FxHashSet::default();
    // Named-variable double-encode: `set e [enc $x]; enc $e`.
    for (&sym, &ver) in uses {
        let name = ctx.ssa.var_name(sym);
        if is_seeded_global_v0(name, ver) || emitted.contains(&sym) || quoted_uses.contains(&sym) {
            continue;
        }
        let t = taints
            .get(&(sym, ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if t.is_tainted() && t.colours.intersects(dup_colour) {
            warnings.push(TaintWarning {
                span,
                variable: name.to_owned(),
                sink_command: command.to_owned(),
                code: DiagCode::T106,
                message: format!(
                    "Variable ${name} is already {label}; passing through {command} \
                     double-encodes the value"
                ),
                replacement: None,
                fixes: Vec::new(),
            });
            emitted.insert(sym);
        }
    }
    // Nested double-encode with no named intermediate: `enc [enc $x]`. The
    // inner encoder's result colour never lands on a named variable, so scan
    // each argument *word* — a command substitution whose value already carries
    // the colour is the same hazard. Emit once for the statement, and skip pure
    // `$var` words (already covered by the named-variable loop above).
    if emitted.is_empty() {
        for arg in call_args {
            if is_pure_var_ref(arg.trim()) {
                continue;
            }
            let t = word_taint(arg, uses, taints, ctx);
            if t.is_tainted() && t.colours.intersects(dup_colour) {
                warnings.push(TaintWarning {
                    span,
                    variable: String::new(),
                    sink_command: command.to_owned(),
                    code: DiagCode::T106,
                    message: format!(
                        "The value passed to {command} is already {label}; passing it \
                         through {command} again double-encodes it"
                    ),
                    replacement: None,
                    fixes: Vec::new(),
                });
                break;
            }
        }
    }
}

/// Return `true` when a tainted value `lat` is mitigated for the given
/// iRules sink code (the IRULE3001/3002/3003/3004 branches).
///
/// For IRULE3002 in the name-position (arg-index 1 of
/// `HTTP::header`/`HTTP::cookie` `insert`/`replace`), the
/// `HEADER_TOKEN_SAFE` colour is an additional mitigation. That extra
/// check is handled at the call site because it needs the per-use arg
/// index; the function signature here is deliberately kept narrow.
fn irules_sink_suppressed(code: DiagCode, lat: TaintLattice) -> bool {
    if !lat.is_tainted() {
        return false;
    }
    match code {
        DiagCode::Irule3001 => lat.colours.intersects(TaintColour::HTML_ESCAPED),
        DiagCode::Irule3002 | DiagCode::Irule3003 => lat.colours.intersects(TaintColour::CRLF_SAFE),
        DiagCode::Irule3004 => lat.colours.intersects(TaintColour::REDIRECT_SAFE),
        _ => false,
    }
}

/// Return `true` when the tainted var `var_name` occupies a
/// header/cookie *name* position (arg index 1 after the `insert` /
/// `replace` subcommand) in `args` and carries the
/// `HEADER_TOKEN_SAFE` colour — the IRULE3002 extra mitigation.
fn irule3002_name_position_safe(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    var_name: &str,
    lat: TaintLattice,
) -> bool {
    if !lat.colours.contains(TaintColour::HEADER_TOKEN_SAFE) {
        return false;
    }
    // Registry-driven + prefix-aware: an output-sink subcommand
    // (`taint_output_sink_subcommands`, e.g. `insert`/`replace`) resolved from
    // whatever the caller wrote (`ins` ⇒ `insert`), so the name-position
    // safety check matches the same set the sink classifier uses.
    let Some(spec) = registry.get(command) else {
        return false;
    };
    let Some(sub_word) = args.first() else {
        return false;
    };
    let is_output_sink_sub = !spec.taint_output_sink_subcommands.is_empty()
        && spec
            .resolve_subcommand(sub_word)
            .is_some_and(|sub| spec.taint_output_sink_subcommands.contains(&sub.name));
    if !is_output_sink_sub {
        return false;
    }
    let Some(arg) = args.get(1) else { return false };
    let stripped = arg.trim();
    is_pure_var_ref(stripped) && normalise_var_name(stripped) == var_name
}

/// Conservatively force every taint value touching a module-traced variable
/// to fully tainted — the "unknown taint" fix for `trace add variable` /
/// `trace variable`.
///
/// `trace add variable x write {handler}` (or a `read` trace) lets a runtime
/// callback rewrite `x` on every future access in ways static analysis
/// cannot see: the handler might sanitise a value the analysis considers
/// tainted (so a warning is arguably unwarranted), or it might inject
/// attacker-controlled content into a value the source only ever assigns
/// literals (so a real sink would be silently missed). Rather than model
/// trace semantics, this takes the safe side of that ambiguity — treat a
/// traced name as tainted everywhere, so the analysis never launders a
/// value through an opaque handler it can't inspect. This also incidentally
/// fixes a sharper, pre-existing bug: `trace add variable x write …` is
/// itself registered as a *write* to `x` (`ArgRole::VarWrite`, so SSA gives
/// it a fresh version for soundness elsewhere), and the fallback "propagate
/// from arguments" rule in [`evaluate_taint_def`] computed that fresh
/// version's taint from the trace-add call's own *literal* argument words
/// (`add`, `variable`, `x`, `write`, `{handler}` — none of them a `$`
/// reference) — always clean, silently dropping any taint `x` carried
/// *before* the trace was installed.
///
/// Uses [`crate::ir::Module::traced_variables`] /
/// [`crate::ir::Module::has_dynamic_variable_trace`] — the same
/// flow-insensitive, whole-module over-approximation (the same "traced
/// anywhere ⇒ traced everywhere, for the module's lifetime" rule already
/// applied to the optimiser's constant-fold trust gate via
/// [`crate::compilation_unit::ModuleTraceFacts`]) rather than a bespoke
/// traced-name scan.
///
/// Two passes:
/// 1. Every `(symbol, version)` already in `taints` for a traced name is
///    forced to [`TaintLattice::tainted()`] — this is every non-zero SSA
///    version, since [`propagate_statement_taints`] always inserts an entry
///    for a def, tainted or not.
/// 2. A traced global/namespace name read at version 0 with **no** local
///    def in this function at all (a pure cross-proc read reusing
///    [`collect_global_reads`]) is seeded tainted too, so a function that
///    only reads a traced global still sees it as tainted even without the
///    interprocedural global-write seeding this mirrors.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn apply_module_variable_traces(
    mut taints: HashMap<ValueKey, TaintLattice>,
    ssa: &SsaFunction,
    traces: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> HashMap<ValueKey, TaintLattice> {
    let is_traced =
        |name: &str| traces.has_dynamic_variable_trace || traces.traced_variables.contains(name);
    for (key, lattice) in &mut taints {
        if is_traced(ssa.var_name(key.0)) {
            *lattice = TaintLattice::tainted();
        }
    }
    for name in collect_global_reads(ssa) {
        if is_traced(&name)
            && let Some(sym) = ssa.var_symbol(&name)
        {
            taints
                .entry((sym, 0))
                .and_modify(|t| *t = TaintLattice::tainted())
                .or_insert_with(TaintLattice::tainted);
        }
    }
    taints
}

/// Find every taint warning across a whole compilation unit.
///
/// Public `*_for_cu` entry point (mirroring
/// [`crate::shimmer::find_shimmer_warnings_for_cu`]) composing the
/// `TaintWarning`-producing passes — sink detection, setter-constraint
/// violations, iRules URI-split suggestions, and destructive-file warnings
/// — over each function in the unit, in `cu.functions()` order. The
/// path-concatenation pass is omitted: its lattice colours are not yet
/// assigned (latent, always empty today).
#[must_use]
pub fn find_taint_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    let mut out = find_taint_warnings_for_cu_base(cu, registry, dialect);
    let callback_warnings = find_callback_substitution_warnings(cu, registry, dialect, &out);
    out.extend(callback_warnings);
    out
}

/// The ordinary whole-unit taint pass, without a second pass through stored
/// callback text. Kept separate so callback replay can analyse a synthetic
/// invocation without recursively replaying every registration in its source
/// module.
fn find_taint_warnings_for_cu_base(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    find_taint_warnings_for_cu_base_with_external_variable_seeds(cu, registry, dialect, None)
}

/// The ordinary warning pass with explicitly seeded top-level external values.
///
/// Synthetic callback replays use this rather than spelling a source command
/// such as `[gets stdin]`: a registration's user input is independent of
/// source-level command shadowing and aliases.
fn find_taint_warnings_for_cu_base_with_external_variable_seeds(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    external_variable_seeds: Option<&HashMap<String, TaintLattice>>,
) -> Vec<TaintWarning> {
    let mut out = Vec::new();
    let solved = external_variable_seeds.map_or_else(
        || crate::taint_interproc::solve_interprocedural_taints(cu, registry, dialect),
        |seeds| {
            crate::taint_interproc::solve_interprocedural_taints_with_external_variable_seeds(
                cu, registry, dialect, seeds,
            )
        },
    );
    let proc_qnames: HashSet<&str> = cu.procedures.keys().map(String::as_str).collect();
    let module_traces = crate::compilation_unit::ModuleTraceFacts {
        traced_variables: &cu.ir_module.traced_variables,
        has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
    };
    let identities = crate::head_identity::command_head_identities_with_config(
        &cu.source,
        dialect.map_or_else(tcl_lexer::LexerConfig::default, |profile| {
            tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        }),
        registry,
    );
    // `analysable_body_function_units` (not `analysable_functions`) so a
    // sink inside a TclOO method body — or an `apply` lambda / `namespace
    // eval` body — is still checked; see its doc comment.
    for fu in cu.analysable_body_function_units() {
        let exec = &fu.sccp.executable_blocks;
        let taints = apply_module_variable_traces(
            solved.taints_for(&fu.name, &fu.taints).clone(),
            &fu.ssa,
            module_traces,
        );
        let namespace = crate::optimiser::helpers::naming::namespace_from_qualified(&fu.name);
        let shadowed = shadowed_builtin_names(&namespace, proc_qnames.iter().copied(), registry);
        out.extend(find_taint_warnings_for_function_with_identities(
            fu,
            &taints,
            exec,
            registry,
            dialect,
            &shadowed,
            module_traces,
            &identities,
        ));
        out.extend(find_setter_constraint_warnings(
            registry, &fu.cfg, &fu.ssa, &taints, exec, dialect,
        ));
        out.extend(crate::uri_split::find_uri_split_suggestions(
            &fu.cfg,
            &fu.ssa,
            Some(&fu.sccp.values),
            exec,
            registry,
            dialect,
        ));
        out.extend(find_destructive_file_warnings(
            &fu.cfg, &fu.ssa, &taints, exec, registry, dialect,
        ));
    }
    out
}

/// Maximum statically recoverable stored callbacks replayed in one batch.
///
/// A callback replay performs a normal, interprocedural compiler pass so it
/// can reuse the established SSA and proc-parameter taint transfer. Cap the
/// per-batch work rather than making one synthetic unit quadratic; ordinary
/// applications have a handful of handlers.
const MAX_CALLBACK_TAINT_REPLAYS: usize = 32;

#[cfg(test)]
std::thread_local! {
    /// Number of synthetic compilation units built by one warning pass.
    static CALLBACK_REPLAY_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Number of source scans used to reserve synthetic replay identifiers.
    static CALLBACK_REPLAY_NAME_SEARCHES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

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

/// Synthetic callback parameter used only inside replay procedures. Its
/// top-level source slot is seeded directly in SSA rather than assigned by a
/// source command, so user shadows cannot affect the external-input model.
const CALLBACK_REPLAY_INPUT_VAR_BASE: &str = "__tcl_lsp_callback_input";
const CALLBACK_REPLAY_PROC_BASE: &str = "__tcl_lsp_callback_replay";

/// Pick synthetic names that cannot collide with source/callback variables.
///
/// Every replayed callback came from `source`, so absence from the complete
/// source text also proves absence from every static callback body we append.
/// Procedure candidates reserve one unique wrapper per bounded replay.
/// Deterministic suffixes keep diagnostics/replays reproducible.
fn fresh_callback_replay_names(source: &str) -> (String, String) {
    #[cfg(test)]
    CALLBACK_REPLAY_NAME_SEARCHES.with(|count| count.set(count.get() + 1));

    // Collecting identifier-shaped matches is linear in the source. Unlike a
    // repeated `contains` loop, it does not turn a deliberately collision-rich
    // document into dozens of full scans before callback analysis even starts.
    let mut occupied = HashSet::new();
    for base in [CALLBACK_REPLAY_INPUT_VAR_BASE, CALLBACK_REPLAY_PROC_BASE] {
        for (start, _) in source.match_indices(base) {
            let suffix = &source[start + base.len()..];
            let identifier_len = suffix
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            occupied.insert(source[start..start + base.len() + identifier_len].to_owned());
        }
    }

    // Every rejected suffix consumes a distinct occupied spelling, so the
    // next `occupied.len() + 1` candidates must contain a collision-free one.
    for suffix in 0..=occupied.len() {
        let input = if suffix == 0 {
            CALLBACK_REPLAY_INPUT_VAR_BASE.to_owned()
        } else {
            format!("{CALLBACK_REPLAY_INPUT_VAR_BASE}_{suffix}")
        };
        let proc_prefix = if suffix == 0 {
            CALLBACK_REPLAY_PROC_BASE.to_owned()
        } else {
            format!("{CALLBACK_REPLAY_PROC_BASE}_{suffix}")
        };
        let proc_collides = (0..MAX_CALLBACK_TAINT_REPLAYS)
            .any(|index| occupied.contains(&format!("{proc_prefix}_{index}")));
        if !occupied.contains(&input) && !proc_collides {
            return (input, format!("::{proc_prefix}"));
        }
    }
    unreachable!("a bounded synthetic-name search must find an unoccupied suffix")
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
}

/// Query a callback argument through either a direct command or a resolved
/// instance method. The latter is driven solely by local receiver-class facts
/// and `CommandRegistry::instance_callback_taint_inputs`; widget names and
/// method spellings never enter the analyser as special cases.
fn callback_taint_inputs_at_call(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    index: usize,
    dialect: DialectSet,
    instance_classes: &LocalInstanceClasses,
    source_position: u32,
) -> &'static [tcl_registry::CallbackTaintInput] {
    let direct = registry.callback_taint_inputs(command, args, index, dialect);
    if !direct.is_empty() {
        return direct;
    }
    let Some(class) = unique_instance_class(command, Some(instance_classes), Some(source_position))
    else {
        return &[];
    };
    registry.instance_callback_taint_inputs(class, command, args, index, dialect)
}

/// Replay statically materialised stored callbacks with their registry-declared
/// external substitutions replaced by a directly seeded synthetic source.
///
/// The substitution descriptor is the only Tk knowledge here. The callback
/// body is then lowered and checked by the same generic CFG/SSA/interproc
/// taint pipeline as ordinary source, which covers both `set x %P; eval $x`
/// and a `[list helper %P]` prefix flowing into `proc helper {x} { eval $x }`.
/// Dynamic callback construction deliberately abstains.
// Candidate discovery, bounded materialisation, and source-span remapping are
// one safety transaction; keeping them adjacent makes the replay cap auditable.
#[allow(clippy::too_many_lines)]
fn find_callback_substitution_warnings(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    baseline: &[TaintWarning],
) -> Vec<TaintWarning> {
    let mut candidates = Vec::new();
    // Most files have no statically replayable command-substitution callback.
    // Build the source-position command table only when one actually needs its
    // builder head resolved, while still sharing the one map across candidates.
    let callback_head_identities = std::cell::OnceCell::new();
    let lexer_config = dialect.map_or_else(tcl_lexer::LexerConfig::default, |profile| {
        tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    });

    for fu in cu.all_body_function_units() {
        let instance_classes = instance_classes_for_function(
            &fu.cfg,
            registry,
            cu.interproc.as_ref(),
            fu.name != "::top",
        );
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let (command, args, tokens) = match stmt {
                    Statement::Call { args, tokens, .. }
                    | Statement::Barrier { args, tokens, .. } => (
                        stmt.canonical_command_or_source(),
                        args.as_slice(),
                        tokens.as_ref(),
                    ),
                    _ => continue,
                };
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                for (index, callback) in args.iter().enumerate() {
                    let inputs = callback_taint_inputs_at_call(
                        registry,
                        command,
                        &arg_refs,
                        index,
                        dialect_to_set(dialect),
                        &instance_classes,
                        stmt.span().start(),
                    );
                    if inputs.is_empty() {
                        continue;
                    }
                    let callback_word = tokens.and_then(|tokens| tokens.word_exprs.get(index + 1));
                    let command_substitution = callback_word.and_then(whole_command_substitution);
                    let source_is_command_substitution = command_substitution.is_some()
                        || tokens
                            .and_then(|tokens| tokens.argv_kinds.get(index + 1))
                            .is_some_and(|kind| *kind == tcl_lexer::TokenType::Cmd);
                    // The callback argument is evaluated once when registered.
                    // Reduce static brace/quote/bare/backslash forms to that
                    // runtime value before parsing it as the later callback
                    // script. A whole command substitution remains source for
                    // the specialised list-prefix builder below.
                    let materialised_callback = if source_is_command_substitution {
                        command_substitution.unwrap_or(callback).to_owned()
                    } else if let Some(word) = callback_word {
                        let Some(value) = static_word_expr_value(word, lexer_config.escapes) else {
                            continue;
                        };
                        value
                    } else {
                        callback.clone()
                    };
                    let Some(callback_input_label) =
                        callback_input_label(&materialised_callback, inputs)
                    else {
                        continue;
                    };
                    let callback_span = tokens
                        .and_then(|tokens| tokens.argv.get(index + 1).copied())
                        .unwrap_or_else(|| stmt.span());
                    let head_identities = source_is_command_substitution.then(|| {
                        callback_head_identities.get_or_init(|| {
                            crate::head_identity::command_head_identities_with_config(
                                &cu.source,
                                lexer_config,
                                registry,
                            )
                        })
                    });
                    // Prove the callback can be materialised before choosing
                    // synthetic names. The probe is never compiled or
                    // evaluated, so it need not be collision-free; it only
                    // exercises the parser/list-prefix policy. This prevents
                    // dynamic registrations from consuming the replay cap or
                    // making callback-free files scan their full source.
                    if callback_replay_source(
                        &materialised_callback,
                        inputs,
                        source_is_command_substitution,
                        "__tcl_lsp_callback_probe",
                        "::__tcl_lsp_callback_probe",
                        dialect,
                        head_identities.map(|identities| (identities, callback_span.start())),
                    )
                    .is_none()
                    {
                        continue;
                    }
                    candidates.push(CallbackReplayCandidate {
                        callback: materialised_callback,
                        inputs,
                        source_is_command_substitution,
                        callback_span,
                        callback_input_label,
                    });
                }
            }
        }
    }

    if candidates.is_empty() {
        return Vec::new();
    }

    let (input_var, proc_prefix) = fresh_callback_replay_names(&cu.source);
    let mut out = Vec::new();
    // All recoverable callbacks are replayed in bounded batches. The per-batch
    // ranges retain source mapping while bounding the amount of synthetic
    // callback code in each interprocedural solve. Every statically recoverable
    // callback is processed; batching is a work-unit bound, not a soundness cap.
    for candidates in candidates.chunks(MAX_CALLBACK_TAINT_REPLAYS) {
        let mut replays: Vec<(String, Span, String)> = Vec::with_capacity(candidates.len());
        for (index, candidate) in candidates.iter().enumerate() {
            let proc_name = format!("{proc_prefix}_{index}");
            let replay = callback_replay_source(
                &candidate.callback,
                candidate.inputs,
                candidate.source_is_command_substitution,
                &input_var,
                &proc_name,
                dialect,
                callback_head_identities
                    .get()
                    .map(|identities| (identities, candidate.callback_span.start())),
            )
            .expect("callback replay preflight proved this script is static");
            replays.push((
                replay,
                candidate.callback_span,
                candidate.callback_input_label.clone(),
            ));
        }

        let mut injected = String::with_capacity(
            cu.source.len()
                + replays
                    .iter()
                    .map(|(source, _, _)| source.len() + 1)
                    .sum::<usize>(),
        );
        injected.push_str(&cu.source);
        let mut replay_spans: Vec<(Span, Span, String)> = Vec::with_capacity(replays.len());
        for (replay, callback_span, callback_input_label) in replays {
            injected.push('\n');
            let start = u32::try_from(injected.len()).unwrap_or(u32::MAX);
            injected.push_str(&replay);
            let end = u32::try_from(injected.len()).unwrap_or(u32::MAX);
            replay_spans.push((Span::new(start, end), callback_span, callback_input_label));
        }

        #[cfg(test)]
        CALLBACK_REPLAY_BUILDS.with(|count| count.set(count.get() + 1));
        let replay_cu =
            crate::compilation_unit::CompilationUnit::build_for(&injected, registry, false)
                .with_interprocedural(registry, dialect);
        // `input_var` was chosen absent from the complete user source, so
        // seeding both scopes cannot influence the original program.
        let replay_seed = HashMap::from([
            (format!("::{input_var}"), TaintLattice::tainted()),
            (input_var.clone(), TaintLattice::tainted()),
        ]);
        for mut warning in find_taint_warnings_for_cu_base_with_external_variable_seeds(
            &replay_cu,
            registry,
            dialect,
            Some(&replay_seed),
        ) {
            // A prefix can call a local procedure. Its sink then lies in the
            // original procedure body even though the synthetic invocation
            // taints its parameter. Keep that new source span intact. Only a
            // sink in an appended callback script is retargeted.
            if baseline.iter().any(|existing| {
                existing.span == warning.span
                    && existing.variable == warning.variable
                    && existing.sink_command == warning.sink_command
                    && existing.code == warning.code
            }) {
                continue;
            }
            if let Some((_, registration, callback_input_label)) =
                replay_spans.iter().find(|(span, _, _)| {
                    warning.span.start() >= span.start() && warning.span.start() < span.end()
                })
            {
                if warning.variable == input_var {
                    let synthetic_name = format!("Tainted variable ${input_var}");
                    warning.variable.clone_from(callback_input_label);
                    warning.message =
                        warning
                            .message
                            .replacen(&synthetic_name, callback_input_label, 1);
                }
                warning.span = *registration;
                // A future sink-specific fix emitted for replay text must
                // never point at the appended synthetic source.
                warning.replacement = None;
                warning.fixes.clear();
            }
            out.push(warning);
        }
    }
    out
}

/// A word whose runtime value is exactly one command substitution. Quotes
/// contribute empty text fragments, so `"[list ...]"` is equivalent to the
/// unquoted form; any real prefix/suffix makes it a different value.
fn whole_command_substitution(word: &WordExpr) -> Option<&str> {
    match word {
        WordExpr::CommandSubstitution { spelling, .. } => Some(spelling),
        WordExpr::Template { parts, .. } => {
            let mut substitution = None;
            for part in parts {
                match part {
                    WordPart::Text { text, .. } if text.is_empty() => {}
                    WordPart::CommandSubstitution { spelling, .. } if substitution.is_none() => {
                        substitution = Some(spelling.as_str());
                    }
                    _ => return None,
                }
            }
            substitution
        }
        _ => None,
    }
}

/// Convert one statically materialised callback script into normal Tcl source
/// whose registry-declared external `%` fields are explicit taint sources.
///
/// A braced/literal script is already executable source. A whole `[list ...]`
/// callback is the idiomatic prefix form: `list` builds the script at
/// registration time, so its decoded elements become one invocation here.
/// Any other command substitution or a malformed list remains dynamic and is
/// intentionally not guessed.
fn callback_replay_source(
    callback: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    source_is_command_substitution: bool,
    input_var: &str,
    proc_name: &str,
    dialect: Option<&tcl_dialect::DialectProfile>,
    builder_context: Option<(&crate::head_identity::HeadIdentityMap, u32)>,
) -> Option<String> {
    let callback = callback.trim().strip_prefix('+').unwrap_or(callback.trim());
    // Current lowering normally preserves the brackets in `args`; older
    // compatibility IR can retain only the command content.  The token kind
    // tells us it was a substitution in both cases, while the spelling avoids
    // wrapping an already bracketed word a second time.
    let bracketed = (source_is_command_substitution && !callback.starts_with('['))
        .then(|| format!("[{callback}]"));
    let callback_command = bracketed.as_deref().unwrap_or(callback);
    if callback_command.starts_with('[') {
        let script = callback_replay_list_prefix(
            callback_command,
            inputs,
            input_var,
            dialect,
            builder_context,
        )?;
        return Some(callback_replay_with_input(&script, proc_name, input_var));
    }
    let script = callback_replay_script(callback, inputs, input_var, dialect)?;
    Some(callback_replay_with_input(&script, proc_name, input_var))
}

/// Materialise one static `[list COMMAND ARG ...]` callback prefix.
///
/// The list builder executes while the callback is registered, so each source
/// word is first reduced to its Tcl value under the document's release grammar.
/// Variable/command substitutions remain dynamic and make the replay abstain.
fn callback_replay_list_prefix(
    callback: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
    dialect: Option<&tcl_dialect::DialectProfile>,
    builder_context: Option<(&crate::head_identity::HeadIdentityMap, u32)>,
) -> Option<String> {
    let inner = callback.strip_prefix('[')?.strip_suffix(']')?;
    let config = dialect.map_or_else(tcl_lexer::LexerConfig::default, |profile| {
        tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    });
    let commands = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial || !command.word_views_aligned() {
        return None;
    }
    let expand = command.expand_word.as_deref().unwrap_or_default();
    let mut values = Vec::new();
    for (index, (word, fragments)) in command
        .texts
        .iter()
        .zip(&command.word_fragments)
        .enumerate()
    {
        let value = callback_static_word_value(word, fragments, config.escapes)?;
        if expand.get(index).copied().unwrap_or(false) {
            values.extend(
                tcl_syntax::list::split_list(&value)
                    .ok()?
                    .into_iter()
                    .map(std::borrow::Cow::into_owned),
            );
        } else {
            values.push(value);
        }
    }
    let written_builder = values.first()?;
    let resolved_builder =
        builder_context.map_or(written_builder.as_str(), |(identities, source_position)| {
            identities
                .resolve(written_builder, source_position)
                .spec_name()
        });
    if resolved_builder != "list" && resolved_builder != "::list" {
        return None;
    }
    let elements = &values[1..];
    if elements.is_empty() {
        return None;
    }
    Some(
        elements
            .iter()
            .map(|element| callback_replay_list_word(element, inputs, input_var))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Build a callback script that is taint-equivalent at Tcl word granularity.
///
/// Tk replaces its `%` fields *before* it evaluates the callback, after
/// quoting each replacement as one Tcl element.  Replacing raw text would get
/// a braced word wrong: `eval {%P}` would become `eval {$input}`, whose first
/// evaluation only sees a literal.  The segmenter gives us Tcl's actual word
/// boundaries, so a marked non-head word instead becomes the symbolic input
/// value itself (`eval ${input}`). This also keeps quoted/bare embedded
/// prefix/suffix as one tainted word. A marked head is preserved as the
/// seeded dynamic value and classified by the ordinary dynamic-dispatch sink.
fn callback_replay_script(
    callback: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Option<String> {
    let config = dialect.map_or_else(tcl_lexer::LexerConfig::default, |profile| {
        tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    });
    let commands = crate::segmenter::segment_commands_with_offset_and_config(callback, 0, config);
    if commands.is_empty() {
        return None;
    }
    let mut replayed = Vec::with_capacity(commands.len());
    for command in commands {
        if command.is_partial || command.texts.is_empty() || !command.word_views_aligned() {
            return None;
        }
        let mut words = Vec::with_capacity(command.texts.len());
        let expand = command.expand_word.as_deref().unwrap_or_default();
        for (index, (word, fragments)) in command
            .texts
            .iter()
            .zip(&command.word_fragments)
            .enumerate()
        {
            let marked = callback_word_has_input(word, inputs);
            let rendered = if marked {
                callback_replay_tainted_word(word, inputs, input_var)
            } else {
                // Preserve dynamic substitutions for the ordinary compiler
                // pipeline (including a phase-correct `$command` head), while
                // reducing static words to their exact runtime value.
                callback_replay_script_word(word, fragments, config.escapes)?
            };
            // `{*}` is an argv operation outside the word itself. Canonical
            // word rendering must retain that marker or a callback whose
            // command prefix is a list silently changes into one command name.
            words.push(if expand.get(index).copied().unwrap_or(false) {
                format!("{{*}}{rendered}")
            } else {
                rendered
            });
        }
        replayed.push(words.join(" "));
    }
    Some(replayed.join("\n"))
}

/// Render a list-prefix element back into a source word. A replacement in head
/// position remains a seeded dynamic value; the ordinary taint pass diagnoses
/// that runtime dispatch without inventing a concrete target.
fn callback_replay_list_word(
    word: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
) -> String {
    if callback_word_has_input(word, inputs) {
        return callback_replay_tainted_word(word, inputs, input_var);
    }
    // The list builder has already reduced each element to one Tcl value. The
    // command head still needs canonical one-word quoting: `{eval foo}` names
    // a single command containing a space and must never be replayed as the
    // builtin `eval` plus an argument.
    tcl_syntax::list::list_element(word)
}

/// Replay source contains only callback text; its input variable is seeded
/// directly in the taint solver, so user definitions of `gets`, `set`, or
/// aliases cannot influence the callback-source model. Every callback runs in
/// its own synthetic procedure: event handlers are independent invocations,
/// not a registration-ordered shared local scope. A direct invocation keeps
/// user rebindings of `catch` out of the scaffold; abnormal completion remains
/// inside the callback procedure's own CFG.
fn callback_replay_with_input(script: &str, proc_name: &str, input_var: &str) -> String {
    let invocation = format!("{proc_name} ${{::{input_var}}}");
    format!(
        "::proc {proc_name} {{{input_var}}} {}\n{invocation}",
        tcl_syntax::list::list_element(script),
    )
}

/// Whether a logical callback word contains an authorised, unescaped `%`
/// marker. Metadata replacements remain ordinary literal text.
fn callback_word_has_input(text: &str, inputs: &[tcl_registry::CallbackTaintInput]) -> bool {
    inputs.iter().copied().any(|input| input.occurs_in(text))
}

/// User-facing label for a callback replay source. A callback with exactly
/// one declared replacement keeps that useful spelling; a mixed callback is
/// deliberately described generically rather than pretending every value was
/// the first marker seen.
fn callback_input_label(text: &str, inputs: &[tcl_registry::CallbackTaintInput]) -> Option<String> {
    let markers: Vec<&str> = inputs
        .iter()
        .copied()
        .filter(|input| input.occurs_in(text))
        .map(tcl_registry::CallbackTaintInput::spelling)
        .collect();
    match markers.as_slice() {
        [] => None,
        [marker] => Some(format!("Tk callback input {marker}")),
        _ => Some("Tk callback input".to_owned()),
    }
}

/// Render one callback-script word without changing its substitution shape.
///
/// The segmenter's logical `word` value alone cannot distinguish a dynamic
/// quoted word (`"puts $x"`) from literal list data (`{puts $x}`).  Its
/// lossless fragments can: variable and command fragments remain executable,
/// while every literal byte is escaped inside one quoted Tcl word.  A word
/// with no substitution stays a canonical literal list element.
fn callback_replay_script_word(
    word: &str,
    fragments: &[crate::segmenter::WordFragment],
    escapes: tcl_dialect::EscapeSyntax,
) -> Option<String> {
    if !fragments
        .iter()
        .any(|fragment| matches!(fragment.token.kind, TokenType::Var | TokenType::Cmd))
    {
        return callback_static_word_value(word, fragments, escapes)
            .map(|value| tcl_syntax::list::list_element(&value));
    }

    let mut out = String::with_capacity(word.len() + 2);
    out.push('"');
    for fragment in fragments {
        match fragment.token.kind {
            TokenType::Var | TokenType::Cmd => out.push_str(&fragment.text),
            TokenType::Esc => {
                let decoded = tcl_lexer::backslash_subst_in(&fragment.text, escapes);
                for ch in decoded.chars() {
                    if matches!(ch, '\\' | '"' | '$' | '[') {
                        out.push('\\');
                    }
                    out.push(ch);
                }
            }
            // A braced literal cannot legally concatenate with a substitution
            // in one Tcl word; recovery-shaped input is not safe to replay.
            _ => return None,
        }
    }
    out.push('"');
    Some(out)
}

/// Exact runtime value of one word containing no variable/command
/// substitutions. Braced words preserve backslashes except Tcl's universal
/// backslash-newline continuation; bare/quoted words decode them under the
/// document release's escape grammar.
fn callback_static_word_value(
    word: &str,
    fragments: &[crate::segmenter::WordFragment],
    escapes: tcl_dialect::EscapeSyntax,
) -> Option<String> {
    match fragments {
        [fragment] if fragment.token.kind == TokenType::Str => {
            Some(tcl_syntax::backslash::collapse_brace_continuations_str(word).into_owned())
        }
        fragments
            if fragments
                .iter()
                .all(|fragment| fragment.token.kind == TokenType::Esc) =>
        {
            Some(tcl_lexer::backslash_subst_in(word, escapes).into_owned())
        }
        _ => None,
    }
}

/// Render one non-head word containing a source marker as an interpolated Tcl
/// word. Static bytes are quoted so braces and quotes in the original spelling
/// cannot suppress the symbolic source after Tk's pre-evaluation replacement.
fn callback_replay_tainted_word(
    text: &str,
    inputs: &[tcl_registry::CallbackTaintInput],
    input_var: &str,
) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + input_var.len() + 4);
    out.push('"');
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && bytes.get(index + 1) == Some(&b'%') {
            out.push_str("%%");
            index += 2;
            continue;
        }
        if bytes[index] == b'%'
            && let Some(input) = inputs
                .iter()
                .find(|input| bytes[index..].starts_with(input.spelling().as_bytes()))
        {
            out.push_str("${");
            out.push_str(input_var);
            out.push('}');
            index += input.spelling().len();
            continue;
        }
        let ch = text[index..]
            .chars()
            .next()
            .expect("index is a UTF-8 boundary while copying callback text");
        match ch {
            '\\' | '"' | '$' | '[' => out.push('\\'),
            _ => {}
        }
        out.push(ch);
        index += ch.len_utf8();
    }
    out.push('"');
    out
}

/// Bare registry command names shadowed, for a call inside `namespace`, by a
/// user proc of the same name.
///
/// Tcl resolves an unqualified command name in the *current* namespace,
/// falling back to the global namespace when not found there — a lookup
/// that always terminates at global, so a plain top-level `proc puts {args}
/// {…}` shadows every bare `puts` call in the *whole* module, while `proc
/// ::myns::puts {args} {…}` shadows it only for calls made from `::myns`
/// itself. Either way the call dispatches to the user's own proc, not the
/// output-sink builtin, even though the registry still has a `puts` entry.
///
/// Matches the same simplified, current-namespace-or-global qualification
/// the rest of the compiler already uses for `rename` / `interp alias`
/// resolution (`crate::alias::resolve_alias`) rather than walking the full
/// namespace ancestor chain — a proc sitting in an *intermediate* ancestor
/// namespace (neither the caller's own namespace nor global) is not
/// recognised as shadowing, matching that existing precedent's own gap.
///
/// `proc_qnames` is every procedure's fully-qualified name in the module
/// (`CompilationUnit::procedures` keys); a name only counts as shadowing
/// when it sits *directly* in `namespace` or in the global namespace
/// (`{namespace}::{bare}` / `::{bare}`) — a proc in a nested child namespace
/// does not shadow its parent's calls, and vice versa.
#[must_use]
pub fn shadowed_builtin_names<'a>(
    namespace: &str,
    proc_qnames: impl Iterator<Item = &'a str>,
    registry: &CommandRegistry,
) -> HashSet<String> {
    /// `qname`'s bare tail when it sits directly under `ns_prefix` (e.g.
    /// `"::foo::"` or the global `"::"`) — `None` for a name that's either
    /// outside `ns_prefix` or in one of ITS OWN nested children.
    fn direct_member<'a>(qname: &'a str, ns_prefix: &str) -> Option<&'a str> {
        let bare = qname.strip_prefix(ns_prefix)?;
        (!bare.is_empty() && !bare.contains("::")).then_some(bare)
    }

    let ns_norm = normalise_qualified_name(namespace);
    let own_prefix = if ns_norm == "::" {
        "::".to_owned()
    } else {
        format!("{ns_norm}::")
    };
    let is_global = own_prefix == "::";

    let mut out = HashSet::new();
    for q in proc_qnames {
        let bare = match direct_member(q, &own_prefix) {
            Some(bare) => Some(bare),
            None if is_global => None,
            None => direct_member(q, "::"),
        };
        if let Some(bare) = bare
            && registry.get(bare).is_some()
        {
            out.insert(bare.to_owned());
        }
    }
    out
}

/// Emit taint sink warnings (`T100` family) for one function.
///
/// For each SSA use of a tainted variable in a sink statement, emits
/// one `TaintWarning`. Iterates blocks in `cfg_order` for deterministic
/// diagnostic ordering (matching the other shimmer/taint passes).
///
/// This is the per-function sink scan; the whole-unit `dataflow` / `diag`
/// aggregation calls it over the top level and every procedure unit. The
/// other warning kinds (setter-constraint / uri-split / path-concat /
/// destructive-file) are not emitted here yet.
///
/// `shadowed_builtins` (see [`shadowed_builtin_names`]) lists the bare
/// command names that resolve to a user proc rather than the registry
/// builtin from this function's namespace — those are skipped by the
/// registry-driven sink/source/pattern checks below.
#[must_use]
pub fn find_taint_warnings<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    shadowed_builtins: &HashSet<String, H>,
) -> Vec<TaintWarning> {
    find_taint_warnings_impl(
        cfg,
        ssa,
        taints,
        executable_blocks,
        &SinkWarningContext {
            registry,
            dialect,
            shadowed_builtins,
        },
        None,
    )
}

/// Emit taint sink warnings for one complete compiler function unit.
///
/// Unlike [`find_taint_warnings`], this form can settle a whole-word variable
/// command head (`$command`) from its phase-correct reaching constant
/// definitions. The value-provenance walk is shared with constant-dispatch
/// navigation and remains conservative across traces, frame aliases, dynamic
/// writes, and unprovable joins.
#[must_use]
pub fn find_taint_warnings_for_function<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    fu: &crate::compilation_unit::FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    shadowed_builtins: &HashSet<String, H>,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> Vec<TaintWarning> {
    find_taint_warnings_for_function_with_identities(
        fu,
        taints,
        executable_blocks,
        registry,
        dialect,
        shadowed_builtins,
        module_traces,
        crate::head_identity::HeadIdentityMap::none(),
    )
}

#[allow(clippy::too_many_arguments)]
fn find_taint_warnings_for_function_with_identities<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    fu: &crate::compilation_unit::FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    shadowed_builtins: &HashSet<String, H>,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'_>,
    identities: &crate::head_identity::HeadIdentityMap,
) -> Vec<TaintWarning> {
    find_taint_warnings_impl(
        &fu.cfg,
        &fu.ssa,
        taints,
        executable_blocks,
        &SinkWarningContext {
            registry,
            dialect,
            shadowed_builtins,
        },
        Some(CommandHeadContext {
            fu,
            module_traces,
            identities,
        }),
    )
}

#[derive(Clone, Copy)]
struct CommandHeadContext<'a> {
    fu: &'a crate::compilation_unit::FunctionUnit,
    module_traces: crate::compilation_unit::ModuleTraceFacts<'a>,
    identities: &'a crate::head_identity::HeadIdentityMap,
}

#[derive(Clone, Copy)]
struct SinkWarningContext<'a, H> {
    registry: &'a CommandRegistry,
    dialect: Option<&'a tcl_dialect::DialectProfile>,
    shadowed_builtins: &'a HashSet<String, H>,
}

fn find_taint_warnings_impl<
    S: std::hash::BuildHasher,
    E: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) -> Vec<TaintWarning> {
    let mut warnings: Vec<TaintWarning> = Vec::new();

    for bn in cfg_order(cfg) {
        if !executable_blocks.contains(&bn) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&bn) else {
            continue;
        };

        for ssa_stmt in &ssa_block.statements {
            emit_statement_warnings(
                ssa_stmt,
                taints,
                &mut warnings,
                ssa,
                context,
                command_head_ctx,
            );
        }
        let branch_ctx = BranchTaintCtx {
            ssa_block,
            ssa,
            taints,
            registry: context.registry,
            dialect: context.dialect,
        };
        emit_branch_condition_warnings(cfg, bn, &branch_ctx, &mut warnings);
    }

    warnings
}

/// Emit taint warnings for a single SSA statement.
///
/// Handles three cases:
/// - `AssignExpr` / `ExprEval`: any tainted use is a T100 injection.
/// - `Call` / `Barrier` / `AssignValue` with `[cmd ...]`: classify as a
///   sink via the registry and emit T100/T101 per tainted use.
/// - `Call`: additionally emits T102 option-injection warnings.
fn emit_statement_warnings<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) {
    if emit_expr_statement_warnings(ssa_stmt, taints, warnings, ssa) {
        return;
    }

    let registry = context.registry;
    let dialect = context.dialect;
    let shadowed_builtins = context.shadowed_builtins;
    let stmt = &ssa_stmt.statement;

    // Owned fallback for `AssignValue` — `parse_command_substitution`
    // returns an owned (String, Vec<String>) so we stash it in this
    // scope and borrow into the uniform `(command, call_args)` tuple
    // below. Preserves sub-command args so e.g.
    // `set _ [HTTP::header insert X-Foo $v]` still reaches the
    // IRULE3002 subcommand gate.
    let assign_parsed: Option<(String, Vec<String>)> = match stmt {
        Statement::AssignValue { value, .. } => {
            parse_taint_command_substitution(value.trim(), dialect)
        }
        _ => None,
    };
    // Prefer the canonical target the lowerer already resolved
    // (`stmt.canonical_command_or_source()`, populated for a resolved
    // `interp alias` or static `rename`) over the source spelling, so
    // `interp alias {} myEval {} eval; myEval $tainted` (or `rename eval
    // myEval; myEval $tainted`) is classified against `eval`'s registry
    // sink data instead of silently missing it because `"myEval"` isn't a
    // registered command. Falls back to the source name unchanged when no
    // alias/rename was resolved (the common case). `tokens` carries the
    // per-argument-word spans used to narrow a sink warning's highlight to
    // the one tainted argument (`tight_arg_span`) — only available for a
    // real `Call`/`Barrier` statement; the `AssignValue` fallback
    // re-parses a nested `[cmd …]` out of the `set` RHS text and has no
    // per-word spans for it, so it falls back to the whole-statement span.
    let (raw_command, call_args, tokens): (&str, &[String], Option<&CommandTokens>) = match stmt {
        Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. } => (
            stmt.canonical_command_or_source(),
            args.as_slice(),
            tokens.as_ref(),
        ),
        Statement::AssignValue { .. } => match assign_parsed.as_ref() {
            Some((cmd, sub_args)) => (cmd.as_str(), sub_args.as_slice(), None),
            None => return,
        },
        _ => return,
    };
    let has_canonical_override = matches!(
        stmt,
        Statement::Call { command, .. } | Statement::Barrier { command, .. }
            if raw_command != command
    );
    let static_invocation = (!has_canonical_override)
        .then(|| static_invocation(tokens?, call_args, dialect))
        .flatten();
    let written_command = static_invocation
        .as_ref()
        .map_or(raw_command, |invocation| invocation.head.command.as_str());
    let call_args = static_invocation
        .as_ref()
        .and_then(|invocation| invocation.effective_args.as_deref())
        .unwrap_or(call_args);
    // Once a head expansion inserts or removes argv elements, the original
    // per-word token indices no longer line up with the effective invocation.
    // Preserve correctness and use the statement span rather than pointing a
    // sink diagnostic at the wrong source word.
    let sink_tokens = static_invocation
        .as_ref()
        .map_or(tokens, |invocation| invocation.sink_tokens(tokens));

    // Every command substitution in command position executes before Tcl can
    // dispatch the outer result. Eval's inline-body lowering can produce this
    // generic IR shape for `eval {[eval $x]}`; compound heads such as
    // `prefix[eval $x]` and `"[puts ok][eval $x]"` execute their substitutions
    // too. Classify every one against the registry. The eventual outer command
    // name remains unknown and follows the ordinary conservative path below.
    emit_command_head_substitution_warnings(
        tokens,
        ssa_stmt,
        taints,
        warnings,
        ssa,
        context,
        command_head_ctx,
    );

    let resolved_heads =
        resolve_taint_command_heads(written_command, stmt, tokens, dialect, command_head_ctx);
    if resolved_heads.is_empty() {
        emit_dynamic_command_head_warning(written_command, tokens, ssa_stmt, taints, warnings, ssa);
        return;
    }
    if resolved_heads.len() <= 1 {
        if let Some(head) = resolved_heads.first() {
            let resolved_args = head.effective_args(call_args);
            emit_resolved_statement_warnings(
                &head.command,
                resolved_args.as_deref().unwrap_or(call_args),
                (!head.affects_args()).then_some(sink_tokens).flatten(),
                ssa_stmt,
                &ssa_stmt.uses,
                taints,
                registry,
                dialect,
                shadowed_builtins,
                warnings,
                ssa,
            );
        }
        return;
    }

    // A finite command-head join can contain equivalent source spellings
    // (`eval` / `::eval`). Keep one diagnostic for the semantic invocation,
    // while still evaluating every candidate independently so namespace
    // shadowing cannot hide an explicitly-global sink arm.
    let start = warnings.len();
    let mut candidates = Vec::new();
    for head in &resolved_heads {
        let resolved_args = head.effective_args(call_args);
        emit_resolved_statement_warnings(
            &head.command,
            resolved_args.as_deref().unwrap_or(call_args),
            (!head.affects_args()).then_some(sink_tokens).flatten(),
            ssa_stmt,
            &ssa_stmt.uses,
            taints,
            registry,
            dialect,
            shadowed_builtins,
            &mut candidates,
            ssa,
        );
    }
    for warning in candidates {
        if !warnings[start..].contains(&warning) {
            warnings.push(warning);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StaticCommandHead {
    command: String,
    prepended_args: Vec<String>,
    /// Number of source argument words consumed while leading empty `{*}`
    /// expansions were skipped before finding the runtime command word.
    consumed_source_args: usize,
    expanded: bool,
}

impl StaticCommandHead {
    fn affects_args(&self) -> bool {
        self.expanded || self.consumed_source_args != 0 || !self.prepended_args.is_empty()
    }

    fn effective_args(&self, call_args: &[String]) -> Option<Vec<String>> {
        self.affects_args().then(|| {
            let mut args = self.prepended_args.clone();
            args.extend_from_slice(
                call_args
                    .get(self.consumed_source_args..)
                    .unwrap_or_default(),
            );
            args
        })
    }
}

#[derive(Debug)]
struct StaticInvocation {
    head: StaticCommandHead,
    effective_args: Option<Vec<String>>,
}

impl StaticInvocation {
    fn sink_tokens<'a>(&self, tokens: Option<&'a CommandTokens>) -> Option<&'a CommandTokens> {
        self.effective_args.is_none().then_some(tokens).flatten()
    }
}

fn static_invocation(
    tokens: &CommandTokens,
    call_args: &[String],
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Option<StaticInvocation> {
    let head = static_command_head(tokens, tcl_dialect::EscapeSyntax::of_profile(dialect))?;
    let effective_args = head.effective_args(call_args);
    Some(StaticInvocation {
        head,
        effective_args,
    })
}

/// Resolve the first runtime argv element, accounting for static leading
/// `{*}` list expansions. Expansion can both prepend arguments
/// (`{*}{{eval} safe}`) and disappear entirely (`{*}{}`), in which case a
/// later source word becomes the command.
fn static_command_head(
    tokens: &CommandTokens,
    escapes: tcl_dialect::EscapeSyntax,
) -> Option<StaticCommandHead> {
    static_command_head_from(tokens, escapes, 0)
}

fn static_command_head_from(
    tokens: &CommandTokens,
    escapes: tcl_dialect::EscapeSyntax,
    start: usize,
) -> Option<StaticCommandHead> {
    for (source_index, word) in tokens.word_exprs.iter().enumerate().skip(start) {
        match word {
            WordExpr::Expand { word, .. } => {
                let value = static_word_expr_value(word, escapes)?;
                let elements = tcl_syntax::list::split_list(&value).ok()?;
                let Some((command, rest)) = elements.split_first() else {
                    continue;
                };
                return Some(StaticCommandHead {
                    command: command.to_string(),
                    prepended_args: rest.iter().map(ToString::to_string).collect(),
                    consumed_source_args: source_index,
                    expanded: true,
                });
            }
            _ => {
                return Some(StaticCommandHead {
                    command: static_word_expr_value(word, escapes)?,
                    prepended_args: Vec::new(),
                    consumed_source_args: source_index,
                    expanded: false,
                });
            }
        }
    }
    None
}

/// Exact static value of a command-head word retained by semantic IR.
/// Variable/command substitutions abstain; literal, braced, quoted, and
/// backslash-built heads reduce under Tcl's shared decoder.
fn static_word_expr_value(word: &WordExpr, escapes: tcl_dialect::EscapeSyntax) -> Option<String> {
    match word {
        WordExpr::Literal { text, .. } => Some(text.clone()),
        WordExpr::BracedLiteral { text, .. } => {
            Some(tcl_syntax::backslash::collapse_brace_continuations_str(text).into_owned())
        }
        WordExpr::Template { parts, .. } => {
            let mut value = String::new();
            for part in parts {
                let WordPart::Text { text, .. } = part else {
                    return None;
                };
                value.push_str(&tcl_lexer::backslash_subst_in(text, escapes));
            }
            Some(value)
        }
        WordExpr::Opaque { text, .. } if !text.contains(['$', '[']) => {
            Some(tcl_lexer::backslash_subst_in(text, escapes).into_owned())
        }
        _ => None,
    }
}

/// A tainted value used directly as a command name is itself code injection,
/// even when no finite constant target can be proven. This also covers the
/// canonical IR produced when a static `eval` body reduces to a dynamic head.
fn emit_dynamic_command_head_warning<S: std::hash::BuildHasher>(
    written_command: &str,
    tokens: Option<&CommandTokens>,
    statement: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) {
    if !is_pure_var_ref(written_command) {
        return;
    }
    let name = normalise_var_name(written_command);
    let Some(symbol) = ssa.var_symbol(name) else {
        return;
    };
    let Some(&version) = statement.uses.get(&symbol) else {
        return;
    };
    if !taints
        .get(&(symbol, version))
        .copied()
        .is_some_and(TaintLattice::is_tainted)
    {
        return;
    }
    let span = tokens
        .and_then(|tokens| tokens.argv.first())
        .copied()
        .unwrap_or_else(|| statement.statement.span());
    warnings.push(TaintWarning {
        span,
        variable: name.to_owned(),
        sink_command: "dynamic command dispatch".to_owned(),
        code: DiagCode::T100,
        message: format!(
            "Tainted variable ${name} is used as a dynamic command name; possible code injection"
        ),
        replacement: None,
        fixes: Vec::new(),
    });
}

/// Classify commands executed while substituting a dynamic command head.
fn emit_command_head_substitution_warnings<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    tokens: Option<&CommandTokens>,
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) {
    let mut substitutions = Vec::new();
    if let Some(word) = tokens.and_then(|tokens| tokens.word_exprs.first()) {
        command_substitutions_in_word(word, &mut substitutions);
    }
    for (substitution, source_span) in substitutions {
        let Some(inner) = substitution
            .trim()
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
        else {
            continue;
        };
        emit_isolated_script_warnings(
            inner,
            source_span,
            ssa_stmt,
            taints,
            warnings,
            ssa,
            context,
            command_head_ctx,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_isolated_script_warnings<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    script: &str,
    source_span: Span,
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) {
    let names: FxHashSet<String> = arg_var_names(
        script,
        tcl_dialect::BracedVarStyle::of_profile(context.dialect),
    )
    .into_iter()
    .collect();
    let mut outer_uses = uses_for_names_at_statement(&names, ssa_stmt, ssa);
    if let Some(head_context) = command_head_ctx {
        expand_constant_outer_uses(
            &mut outer_uses,
            ssa_stmt,
            ssa,
            head_context,
            context.dialect,
        );
    }
    let build_nested = |source: &str| {
        context.dialect.map_or_else(
            || crate::compilation_unit::CompilationUnit::build_for(source, context.registry, true),
            |profile| {
                crate::compilation_unit::CompilationUnit::build_for_profile(
                    source,
                    context.registry,
                    true,
                    tcl_dialect::DialectProfile::resolve_known(profile.name)
                        .unwrap_or_else(|| tcl_dialect::DialectProfile::by_name(profile.name)),
                )
            },
        )
    };
    let preliminary_cu = build_nested(script);
    let command_setup = command_head_ctx.map_or_else(String::new, |head_context| {
        ambient_command_setup(
            &preliminary_cu,
            head_context.identities,
            source_span.start(),
            context.shadowed_builtins,
        )
    });
    let constant_setup = command_head_ctx.map_or_else(String::new, |head_context| {
        ambient_constant_setup(&outer_uses, head_context, ssa)
    });
    let (replay, seeds) = nested_script_replay(
        script,
        &command_setup,
        &constant_setup,
        &outer_uses,
        taints,
        ssa,
    );
    let nested_cu = build_nested(&replay);
    for mut warning in find_taint_warnings_for_cu_base_with_external_variable_seeds(
        &nested_cu,
        context.registry,
        context.dialect,
        Some(&seeds),
    ) {
        warning.span = source_span;
        warning.replacement = None;
        warning.fixes.clear();
        if !warnings.contains(&warning) {
            warnings.push(warning);
        }
    }
}

/// Close the nested replay's outer bindings over variables referenced by
/// recovered constant values (`script` = `{$x}` needs both `script` and `x`).
/// The walk is bounded and uses the same exact reaching SSA versions as the
/// first-level names; traced or unprovable values simply stop that branch.
fn expand_constant_outer_uses(
    uses: &mut HashMap<Symbol, Version>,
    statement: &SsaStatement,
    ssa: &SsaFunction,
    context: CommandHeadContext<'_>,
    dialect: Option<&tcl_dialect::DialectProfile>,
) {
    const MAX_NAMES: usize = 128;
    for _ in 0..16 {
        let mut discovered = FxHashSet::default();
        for (&symbol, &version) in uses.iter() {
            let name = ssa.var_name(symbol);
            if context.module_traces.has_dynamic_variable_trace
                || context.module_traces.traced_variables.contains(name)
            {
                continue;
            }
            let Some(contributors) =
                crate::value_provenance::const_contributors_for_version(context.fu, name, version)
            else {
                continue;
            };
            for contributor in contributors {
                discovered.extend(arg_var_names(
                    &contributor.value,
                    tcl_dialect::BracedVarStyle::of_profile(dialect),
                ));
            }
        }
        let additions = uses_for_names_at_statement(&discovered, statement, ssa);
        let before = uses.len();
        for (symbol, version) in additions {
            if uses.len() >= MAX_NAMES {
                return;
            }
            uses.entry(symbol).or_insert(version);
        }
        if uses.len() == before {
            return;
        }
    }
}

/// Put an isolated command substitution in a synthetic procedure and pass its
/// referenced outer values as ordinary parameters. This is the same proven
/// shape as callback replay: interprocedural entry seeding then reaches
/// structured `eval` bodies, while assignments inside the substitution retain
/// their normal CFG/SSA evaluation order.
fn nested_script_replay<S: std::hash::BuildHasher>(
    inner: &str,
    command_setup: &str,
    constant_setup: &str,
    uses: &HashMap<Symbol, Version>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ssa: &SsaFunction,
) -> (String, HashMap<String, TaintLattice>) {
    let collision_text = format!("{command_setup}{constant_setup}{inner}");
    let proc_name = fresh_nested_name(&collision_text, "__tcl_lsp_nested");
    let mut bindings: Vec<(String, Symbol, Version)> = uses
        .iter()
        .map(|(&symbol, &version)| (ssa.var_name(symbol).to_owned(), symbol, version))
        .collect();
    bindings.sort_by(|left, right| left.0.cmp(&right.0));

    let mut params = Vec::with_capacity(bindings.len());
    let mut invocation = Vec::with_capacity(bindings.len() + 1);
    invocation.push(proc_name.clone());
    let mut assignments = String::new();
    let mut seeds = HashMap::new();
    for (index, (name, symbol, version)) in bindings.into_iter().enumerate() {
        let param = fresh_nested_name(&collision_text, &format!("__tcl_lsp_arg_{index}"));
        let seed = fresh_nested_name(&collision_text, &format!("__tcl_lsp_seed_{index}"));
        params.push(param.clone());
        invocation.push(format!("${{::{seed}}}"));
        let _ = writeln!(
            assignments,
            "set {} ${param}",
            tcl_syntax::list::list_element(&name)
        );
        if let Some(taint) = taints.get(&(symbol, version)).copied() {
            seeds.insert(param, taint);
            seeds.insert(format!("::{seed}"), taint);
        }
    }
    // Plumbing runs before the ambient table is materialised and uses global
    // spellings, so an outer user command named `set`, `proc`, or `catch`
    // cannot hijack the replay itself. Ambient facts then affect only the user
    // script they are meant to model.
    let body = format!("{assignments}{constant_setup}{command_setup}{inner}");
    let source = format!(
        "::proc {proc_name} {{{}}} {}\n{}",
        params.join(" "),
        tcl_syntax::list::list_element(&body),
        invocation.join(" "),
    );
    (source, seeds)
}

fn fresh_nested_name(source: &str, base: &str) -> String {
    if !source.contains(base) {
        return base.to_owned();
    }
    (1_u32..=u32::MAX)
        .map(|suffix| format!("{base}_{suffix}"))
        .find(|candidate| !source.contains(candidate))
        .unwrap_or_else(|| format!("{base}_{}", source.len()))
}

/// Materialise phase-correct outer constants referenced by a nested script.
/// This preserves a constant `$command` head and deferred script values such
/// as `set script {$x}; [eval $script]`. Traced variables and non-singleton
/// reaching constants abstain rather than guessing one runtime value.
fn ambient_constant_setup(
    uses: &HashMap<Symbol, Version>,
    context: CommandHeadContext<'_>,
    ssa: &SsaFunction,
) -> String {
    let mut values = Vec::new();
    for (&symbol, &version) in uses {
        let name = ssa.var_name(symbol);
        if context.module_traces.has_dynamic_variable_trace
            || context.module_traces.traced_variables.contains(name)
        {
            continue;
        }
        let Some(contributors) =
            crate::value_provenance::const_contributors_for_version(context.fu, name, version)
        else {
            continue;
        };
        let Some(first) = contributors.first() else {
            continue;
        };
        if contributors
            .iter()
            .any(|contributor| contributor.value != first.value)
        {
            continue;
        }
        values.push((name.to_owned(), first.value.clone()));
    }
    values.sort_by(|left, right| left.0.cmp(&right.0));
    let mut setup = String::new();
    for (name, value) in values {
        let _ = writeln!(
            setup,
            "set {} {}",
            tcl_syntax::list::list_element(&name),
            tcl_syntax::list::list_element(&value),
        );
    }
    setup
}

/// Materialise the enclosing command table for an isolated nested-script
/// analysis. The offset-keyed identity owner already composed imports,
/// aliases, renames, and user-proc rebindings; this adapter merely spells its
/// answer as analysis-only Tcl setup so the ordinary lowerer applies the same
/// identity to sources, sinks, argument roles, and nested bodies.
fn ambient_command_setup<H: std::hash::BuildHasher>(
    cu: &crate::compilation_unit::CompilationUnit,
    identities: &crate::head_identity::HeadIdentityMap,
    at: u32,
    shadowed_builtins: &HashSet<String, H>,
) -> String {
    if identities.is_empty() && shadowed_builtins.is_empty() {
        return String::new();
    }
    let mut heads = FxHashSet::default();
    for function in cu.analysable_body_function_units() {
        for block in function.ssa.blocks.values() {
            for statement in &block.statements {
                match &statement.statement {
                    Statement::Call { command, .. } | Statement::Barrier { command, .. }
                        if !command.contains(['$', '[']) =>
                    {
                        heads.insert(command.clone());
                    }
                    _ => {}
                }
            }
        }
    }
    let mut heads: Vec<String> = heads.into_iter().collect();
    heads.sort();
    let mut aliases = String::new();
    let mut rebindings = Vec::new();
    for head in heads {
        let quoted_head = tcl_syntax::list::list_element(&head);
        if shadowed_builtins.contains(&head) {
            rebindings.push((head, format!("::proc {quoted_head} args {{}}\n")));
            continue;
        }
        match identities.binding_at(&head, at) {
            crate::head_identity::HeadBinding::Unchanged => {}
            crate::head_identity::HeadBinding::Rebound => {
                rebindings.push((head, format!("::proc {quoted_head} args {{}}\n")));
            }
            crate::head_identity::HeadBinding::Command(target) => {
                let quoted_target = tcl_syntax::list::list_element(target);
                let _ = writeln!(
                    aliases,
                    "::interp alias {{}} {quoted_head} {{}} {quoted_target}"
                );
            }
        }
    }
    // Establish every alias while `::interp` is still the scaffold builtin;
    // establish a requested `interp`/`proc` rebinding only afterwards.
    rebindings.sort_by_key(|(head, _)| matches!(head.as_str(), "interp" | "proc"));
    for (_, rebinding) in rebindings {
        aliases.push_str(&rebinding);
    }
    aliases
}

/// Emit the direct expression-injection case and report whether this statement
/// was expression-shaped. Keeping it separate leaves call classification
/// focused on registry-backed command invocations.
fn emit_expr_statement_warnings<S: std::hash::BuildHasher>(
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) -> bool {
    let stmt = &ssa_stmt.statement;
    let (Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. }) = stmt else {
        return false;
    };
    let resolve = |name: &str| {
        let sym = ssa.var_symbol(name)?;
        let ver = *ssa_stmt.uses.get(&sym)?;
        Some((sym, ver))
    };
    emit_expr_coercion_warnings(expr, None, stmt.span(), resolve, taints, warnings);
    true
}

/// Parse one command substitution for registry-backed sink classification,
/// reducing a statically valued command word with the document's escape
/// grammar. The shared shape parser retains the original argument spellings;
/// the segmenter supplies the missing Tcl value of heads such as `e\166al`,
/// `{eval}`, and `"eval"`. Dynamic or compound heads remain unchanged and are
/// handled conservatively by command-head provenance.
fn parse_taint_command_substitution(
    text: &str,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Option<(String, Vec<String>)> {
    let (written_head, args) = parse_command_substitution(text)?;
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    let config = dialect.map_or_else(tcl_lexer::LexerConfig::default, |profile| {
        tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    });
    let commands = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial || !command.word_views_aligned() {
        return None;
    }
    let semantic_head = command
        .texts
        .first()
        .zip(command.word_fragments.first())
        .and_then(|(word, fragments)| callback_static_word_value(word, fragments, config.escapes))
        .unwrap_or(written_head);
    Some((semantic_head, args))
}

fn uses_for_names_at_statement(
    names: &FxHashSet<String>,
    statement: &SsaStatement,
    ssa: &SsaFunction,
) -> HashMap<Symbol, Version> {
    let mut uses: HashMap<Symbol, Version> = statement
        .uses
        .iter()
        .filter(|(symbol, _)| names.contains(ssa.var_name(**symbol)))
        .map(|(&symbol, &version)| (symbol, version))
        .collect();
    if uses.len() == names.len() {
        return uses;
    }

    // A variable mentioned only in brace-quoted script text is not necessarily
    // a direct SSA use of the generic outer call. Recover its version at this
    // precise statement by replaying definitions from the containing block's
    // entry. Pointer identity is valid here: callers iterate the statements in
    // this same `SsaFunction`, rather than passing a clone.
    for block in ssa.blocks.values() {
        let mut versions = block.entry_versions.clone();
        for candidate in &block.statements {
            if std::ptr::eq(candidate, statement) {
                for name in names {
                    if let Some(symbol) = ssa.var_symbol(name) {
                        uses.entry(symbol)
                            .or_insert_with(|| versions.get(&symbol).copied().unwrap_or_default());
                    }
                }
                return uses;
            }
            versions.extend(
                candidate
                    .defs
                    .iter()
                    .map(|(&symbol, &version)| (symbol, version)),
            );
        }
    }
    uses
}

/// Collect every command substitution executed while constructing a command
/// head. Literal prefixes/suffixes affect only the eventual outer command;
/// they do not suppress the nested commands. A [`WordExpr::BracedLiteral`]
/// has no executable parts and therefore contributes nothing.
fn command_substitutions_in_word<'a>(word: &'a WordExpr, out: &mut Vec<(&'a str, Span)>) {
    match word {
        WordExpr::CommandSubstitution { spelling, source } => {
            out.push((spelling, source.span));
        }
        WordExpr::Template { parts, .. } => {
            for part in parts {
                if let WordPart::CommandSubstitution { spelling, source } = part {
                    out.push((spelling, source.span));
                }
            }
        }
        WordExpr::Expand { word, .. } => command_substitutions_in_word(word, out),
        _ => {}
    }
}

/// Resolve a whole-word `$var` command head from the definitions reaching this
/// exact call site. Literal heads take the allocation-free path. A traced or
/// otherwise unprovable variable head yields no candidate rather than guessing.
fn resolve_taint_command_heads(
    written_command: &str,
    stmt: &Statement,
    tokens: Option<&CommandTokens>,
    dialect: Option<&tcl_dialect::DialectProfile>,
    ctx: Option<CommandHeadContext<'_>>,
) -> Vec<StaticCommandHead> {
    if !is_pure_var_ref(written_command) {
        return vec![StaticCommandHead {
            command: written_command.to_owned(),
            prepended_args: Vec::new(),
            consumed_source_args: 0,
            expanded: false,
        }];
    }
    let Some(ctx) = ctx else {
        return vec![StaticCommandHead {
            command: written_command.to_owned(),
            prepended_args: Vec::new(),
            consumed_source_args: 0,
            expanded: false,
        }];
    };
    let var_name = normalise_var_name(written_command);
    if ctx.module_traces.has_dynamic_variable_trace
        || ctx.module_traces.traced_variables.contains(var_name)
    {
        return Vec::new();
    }
    let absolute_offset = ctx.fu.abs_span(stmt.span()).start();
    let contributors =
        crate::value_provenance::known_const_contributors(ctx.fu, absolute_offset, var_name);
    if contributors.is_empty() {
        return Vec::new();
    }
    let expanded = tokens
        .and_then(|tokens| tokens.word_exprs.first())
        .is_some_and(|word| matches!(word, WordExpr::Expand { .. }));
    let escapes = tcl_dialect::EscapeSyntax::of_profile(dialect);
    let mut heads = Vec::with_capacity(contributors.len());
    for contributor in contributors {
        let head = if expanded {
            let Ok(elements) = tcl_syntax::list::split_list(&contributor.value) else {
                // This reaching arm raises while expanding and never invokes a
                // command. It must not erase other arms that do reach a sink.
                continue;
            };
            if let Some((command, rest)) = elements.split_first() {
                StaticCommandHead {
                    command: command.to_string(),
                    prepended_args: rest.iter().map(ToString::to_string).collect(),
                    consumed_source_args: 0,
                    expanded: true,
                }
            } else {
                let Some(tokens) = tokens else {
                    continue;
                };
                let Some(mut following) = static_command_head_from(tokens, escapes, 1) else {
                    // This arm's command remains dynamic. Retain any other
                    // reaching arm already proven to invoke a concrete sink.
                    continue;
                };
                following.expanded = true;
                following
            }
        } else if contributor.value.is_empty() {
            continue;
        } else {
            StaticCommandHead {
                command: contributor.value,
                prepended_args: Vec::new(),
                consumed_source_args: 0,
                expanded: false,
            }
        };
        if !heads.contains(&head) {
            heads.push(head);
        }
    }
    heads
}

#[allow(clippy::too_many_arguments)]
fn emit_resolved_statement_warnings<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    command: &str,
    call_args: &[String],
    tokens: Option<&CommandTokens>,
    ssa_stmt: &SsaStatement,
    uses: &HashMap<Symbol, Version>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    registry: &CommandRegistry,
    dialect: Option<&tcl_dialect::DialectProfile>,
    shadowed_builtins: &HashSet<String, H>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) {
    let stmt = &ssa_stmt.statement;
    let span = stmt.span();

    // A namespace-scoped user proc shadows this bare/canonical name (e.g.
    // `proc ::myns::puts {...}` shadowing the real `puts` for calls made
    // from `::myns`) — every check below keys off the registry, which would
    // otherwise misclassify a call to the user's own (possibly entirely
    // safe) proc as the builtin sink/source. Whatever the shadowing proc
    // itself does is covered separately by interprocedural taint analysis.
    if shadowed_builtins.contains(command) {
        return;
    }

    // Emission order per statement:
    // T103 (regexp pattern) → T106 (double-encode) → the sink loop
    // (T100/output/log, then T102, then T104/T105).

    // The document's `${…}` close rule, carried into every name scan below so
    // they read the same closer the lexer did (issue #1604).
    let braced_var = tcl_dialect::BracedVarStyle::of_profile(dialect);
    let env = TaintScan {
        uses,
        taints,
        ssa,
        braced_var,
        quoted_uses: Some(&ssa_stmt.quoted_uses),
    };

    // T103: tainted data in a regexp/regsub pattern position.
    emit_regexp_pattern_warnings(command, call_args, &env, span, registry, warnings);

    // T106: re-encoding an already-encoded tainted value. A minimal ctx (no
    // interprocedural summaries) is enough — the double-encode colour is added
    // by the encoder command's own registry classification, which `word_taint`
    // reads directly.
    let dbl_ctx = TaintCtx {
        registry,
        ssa,
        interproc: None,
        known_procs: None,
        caller_qname: Some(ssa.name.as_str()),
        dialect,
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
    };
    emit_double_encode_warnings(
        command,
        call_args,
        uses,
        taints,
        span,
        warnings,
        dbl_ctx,
        &ssa_stmt.quoted_uses,
    );

    let sink_call = SinkCall {
        command,
        args: call_args,
        registry,
        tokens,
        braced_var,
    };
    // Primary sink classification (T100 code-exec / T101 + iRules output / log).
    if let Some((code, sink_label)) = classify_sink(registry, command, call_args, dialect) {
        emit_sink_warnings(&env, span, code, &sink_label, &sink_call, warnings);
    }

    // T102: option injection — any statement shape that resolves to a
    // command invocation (bare `Call`, a `Barrier`, or a `set x [cmd ...]`
    // `AssignValue`), after the primary sink (T100/output/log). Uses the
    // same `sink_call` (command, call_args, registry) as the check above
    // so a `set matched [regexp $pattern $s]` capture is scanned exactly
    // like the bare `regexp $pattern $s` call it wraps.
    emit_option_injection(&sink_call, &env, span, dialect, warnings, stmt);

    // Additional registry-driven SSRF / cross-interp sinks (T104 / T105),
    // which can co-occur with the primary classification.
    for (code, sink_label) in classify_network_interp_sinks(registry, command, call_args) {
        emit_sink_warnings(&env, span, code, &sink_label, &sink_call, warnings);
    }
}

/// Per-statement read context for the taint-warning emitters: the SSA
/// versions reaching the statement (`uses`), the taint lattice, and the SSA
/// function they resolve against. Bundled so the emitters stay within the
/// argument limit.
struct TaintScan<'a, S> {
    uses: &'a HashMap<Symbol, u32>,
    taints: &'a HashMap<ValueKey, TaintLattice, S>,
    ssa: &'a SsaFunction,
    /// The document's `${…}` close rule, for the shared owner
    /// [`tcl_lexer::braced_var_name_end`] — carried here so every name scan
    /// below reads the same closer the lexer did (issue #1604).
    braced_var: tcl_dialect::BracedVarStyle,
    /// Uses mentioned only in brace-quoted words at this statement.
    quoted_uses: Option<&'a HashSet<Symbol>>,
}

/// Emit `T103` (regex injection / `ReDoS`) for a tainted variable sitting in
/// the regex-pattern argument of a regex command (`regexp` / `regsub` —
/// gated on `pattern_type == Regex`). Suppressed when the
/// value carries the `REGEX_LITERAL` colour.
fn emit_regexp_pattern_warnings<S: std::hash::BuildHasher>(
    command: &str,
    args: &[String],
    env: &TaintScan<'_, S>,
    span: Span,
    registry: &CommandRegistry,
    warnings: &mut Vec<TaintWarning>,
) {
    let (uses, taints, ssa) = (env.uses, env.taints, env.ssa);
    let is_regex = registry
        .get(command)
        .is_some_and(|s| s.pattern_type == Some(tcl_registry::patterns::PatternType::Regex));
    if !is_regex {
        return;
    }
    let Some(pattern_idx) = regexp_pattern_index(args) else {
        return;
    };
    let Some(arg) = args.get(pattern_idx) else {
        return;
    };
    let mut names: Vec<String> = arg_var_names(arg, env.braced_var).into_iter().collect();
    names.sort_unstable();
    for var in names {
        let Some(sym) = ssa.var_symbol(&var) else {
            continue;
        };
        if env.quoted_uses.is_some_and(|quoted| quoted.contains(&sym)) {
            continue;
        }
        let Some(&ver) = uses.get(&sym) else { continue };
        if is_seeded_global_v0(&var, ver) {
            continue;
        }
        let t = taints
            .get(&(sym, ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if !t.is_tainted() {
            continue;
        }
        // A literal-regex colour proves the pattern is trusted.
        if t.colours.intersects(TaintColour::REGEX_LITERAL) {
            continue;
        }
        warnings.push(TaintWarning {
            span,
            variable: var.clone(),
            sink_command: command.to_owned(),
            code: DiagCode::T103,
            message: format!(
                "Tainted variable ${var} in regexp pattern position ({command}); \
                 risk of regex injection or ReDoS"
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// One `ExprNode::Var` occurrence sitting in a numeric-coercion-relevant
/// operator position — the KCS T100 "expr operand: numeric coercion"
/// hazard (see [`collect_coercion_operands`]).
struct CoercionOperand {
    name: String,
    /// Offset of this occurrence relative to the expr's own source text,
    /// when known. `None` for a var recovered from an unparsed
    /// [`ExprNode::Raw`] fallback, which carries no per-occurrence
    /// position.
    rel_span: Option<(u32, u32)>,
}

/// `true` when a binary operator attempts numeric (or, for `And`/`Or`,
/// boolean) coercion of its operands — Tcl's arithmetic, bitwise, shift,
/// numeric-comparison, and logical operators all do. The pure string/list
/// operators (`eq`/`ne`/`lt`/`le`/`gt`/`ge`, `in`/`ni`, and the iRules
/// word-comparison forms `contains`/`starts_with`/`ends_with`/`equals`/
/// `matches_glob`/`matches_regex`) never coerce — `expr {$tainted eq
/// "danger"}` carries no numeric-coercion hazard at all, so flagging it
/// with that message would be a false positive.
const fn binop_coerces(op: crate::expr_ast::BinOp) -> bool {
    use crate::expr_ast::BinOp;
    !matches!(
        op,
        BinOp::StrEq
            | BinOp::StrNe
            | BinOp::StrLt
            | BinOp::StrLe
            | BinOp::StrGt
            | BinOp::StrGe
            | BinOp::In
            | BinOp::Ni
            | BinOp::Contains
            | BinOp::StartsWith
            | BinOp::EndsWith
            | BinOp::StrEquals
            | BinOp::MatchesGlob
            | BinOp::MatchesRegex
    )
}

/// Collect every `Var` occurrence of `expr` that sits in a position where
/// Tcl attempts numeric or boolean coercion of the operand.
///
/// `in_context` is the coercion status of `expr` itself as seen by its
/// parent (the top-level call passes `true`: a braced expr's overall
/// result is used as a number/boolean by its caller). Recursion rules:
///
/// - `Binary`: both children inherit [`binop_coerces`] of the operator —
///   a string-safe operator (`eq`, `in`, …) clears the context for its
///   *own* operands, but a nested coercing sub-expression on either side
///   still flags independently (`($a + 1) eq "3"` still flags `$a`).
/// - `Unary`: all five unary operators (arithmetic negation, bitwise
///   complement, logical `!`/`not`) coerce their operand.
/// - `Ternary`: the condition is always boolean-coerced; each branch
///   *inherits* the ternary's own context, since whichever branch's value
///   flows out stands in for the ternary as a whole in the parent
///   operator (`($c ? $tainted : 0) + 1` must still flag `$tainted`).
/// - `Call` (a math function): every argument is numeric, regardless of
///   context.
/// - `Command` (`[cmd …]`, a nested command substitution): opaque — its
///   contents are a separate statement's own arguments, not an expr
///   operand at all, so this walk does not descend into it. If `cmd` is
///   itself a T100/T101/… sink, that nested statement's own sink
///   classification already covers it.
/// - `Raw` (unparsed fallback text): position-blind but conservative —
///   every variable found by [`ExprNode::vars`] is flagged (`rel_span:
///   None`) regardless of `in_context`, since the parser gave up and the
///   operator structure inside is unknown; erring towards a false
///   positive here is safer than silently dropping coverage.
fn collect_coercion_operands(
    expr: &ExprNode,
    in_context: bool,
    out: &mut Vec<CoercionOperand>,
    depth: u32,
) {
    // Native-stack safety net (issue #996): this walks the `ExprNode` tree,
    // one native frame per level. Past the cap, stop descending — a collector
    // that returns what it has gathered so far is the safe fallback (the only
    // effect is that operands buried deeper than the cap are not flagged;
    // never a crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match expr {
        ExprNode::Var {
            name, start, end, ..
        } => {
            if in_context && !name.is_empty() {
                out.push(CoercionOperand {
                    name: name.clone(),
                    rel_span: Some((*start, *end)),
                });
            }
        }
        ExprNode::Literal { .. } | ExprNode::String { .. } | ExprNode::Command { .. } => {}
        ExprNode::Binary { op, left, right } => {
            let child_ctx = binop_coerces(*op);
            collect_coercion_operands(left, child_ctx, out, depth + 1);
            collect_coercion_operands(right, child_ctx, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            collect_coercion_operands(operand, true, out, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_coercion_operands(condition, true, out, depth + 1);
            collect_coercion_operands(true_branch, in_context, out, depth + 1);
            collect_coercion_operands(false_branch, in_context, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_coercion_operands(arg, true, out, depth + 1);
            }
        }
        ExprNode::Raw { text } => {
            for name in (ExprNode::Raw { text: text.clone() }).vars() {
                out.push(CoercionOperand {
                    name,
                    rel_span: None,
                });
            }
        }
    }
}

/// Emit T100 warnings for numeric-coercion-risk operands of a braced
/// `expr` — an `AssignExpr`/`ExprEval` statement, or an `if`/`while`/`for`
/// branch condition (see [`emit_branch_condition_warnings`]).
///
/// `resolve` maps a variable name occurring in `expr` to the `(Symbol,
/// SSA version)` reaching this point; a `None` result skips that
/// occurrence (no binding here). `base_offset`, when `Some`, is the
/// absolute source offset of `expr`'s own text — `ExprNode::Var` offsets
/// are relative to it, so a hit's absolute span is computable; `None`
/// (or a `Raw`-derived hit with no `rel_span`) falls back to
/// `fallback_span`, the caller's coarser span for the whole expr.
fn emit_expr_coercion_warnings<S: std::hash::BuildHasher>(
    expr: &ExprNode,
    base_offset: Option<u32>,
    fallback_span: Span,
    resolve: impl Fn(&str) -> Option<(Symbol, u32)>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
) {
    let mut hits = Vec::new();
    collect_coercion_operands(expr, true, &mut hits, 0);
    let mut emitted: FxHashSet<Symbol> = FxHashSet::default();
    for hit in hits {
        let Some((sym, ver)) = resolve(&hit.name) else {
            continue;
        };
        if is_seeded_global_v0(&hit.name, ver) || !emitted.insert(sym) {
            continue;
        }
        let t = taints
            .get(&(sym, ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if !t.is_tainted() {
            continue;
        }
        let span = match (hit.rel_span, base_offset) {
            (Some((s, e)), Some(base)) => Span::new(base + s, base + e),
            _ => fallback_span,
        };
        warnings.push(TaintWarning {
            span,
            variable: hit.name.clone(),
            sink_command: "expr".to_owned(),
            code: DiagCode::T100,
            message: format!(
                "Tainted variable ${} flows into expr operand; \
                 numeric coercion may misinterpret value \
                 (use Tcl numeric-validation guards)",
                hit.name
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// Per-block context for the branch-condition taint scan, bundled so
/// [`emit_branch_condition_warnings`] / [`emit_branch_condition_nested_command_warnings`]
/// stay under the 7-argument clippy limit — mirrors [`TaintScan`] /
/// [`SinkCall`] elsewhere in this module.
struct BranchTaintCtx<'a, S> {
    /// The condition's own block — `exit_versions` gives the SSA version of
    /// each condition variable reaching the terminator.
    ssa_block: &'a crate::ssa::SsaBlock,
    ssa: &'a SsaFunction,
    taints: &'a HashMap<ValueKey, TaintLattice, S>,
    registry: &'a CommandRegistry,
    dialect: Option<&'a tcl_dialect::DialectProfile>,
}

/// Emit taint warnings for a block's `if`/`while`/`for` branch condition:
/// T100 for a numeric-coercion-risk operand directly in the condition
/// expression, plus the full sink family for any nested `[cmd …]`
/// command substitution the condition contains (see
/// [`emit_branch_condition_nested_command_warnings`]).
///
/// Branch conditions live on `Terminator::Branch`, not in
/// `ssa_block.statements` — `find_taint_warnings`'s per-statement scan
/// never reaches them, so without this dedicated pass a condition like
/// `if {$tainted + 1 > 5} { … }` raised no T100 at all, even though it is
/// evaluated exactly like any other braced `expr`. Resolves each
/// condition variable's SSA version via the block's `exit_versions` (the
/// version reaching the terminator), mirroring how
/// [`crate::sccp::branch_decision`]'s constant-folding reads the same
/// condition. Uses the condition's own span (tight — just the `{…}` text,
/// not the enclosing `if`/`while`/`for` statement) for every hit: unlike
/// `AssignExpr`/`ExprEval`, no absolute per-variable offset is threaded
/// through the CFG-flattened `Terminator::Branch`, so this falls back to
/// the whole-condition span rather than a per-operand one.
fn emit_branch_condition_warnings<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    bn: BlockId,
    ctx: &BranchTaintCtx<'_, S>,
    warnings: &mut Vec<TaintWarning>,
) {
    let Some(block) = cfg.blocks.get(&bn) else {
        return;
    };
    let Some(Terminator::Branch {
        condition, span, ..
    }) = &block.terminator
    else {
        return;
    };
    let Some(fallback_span) = span else {
        return;
    };
    let resolve = |name: &str| {
        let sym = ctx.ssa.var_symbol(name)?;
        let ver = ctx.ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
        Some((sym, ver))
    };
    emit_expr_coercion_warnings(
        condition,
        None,
        *fallback_span,
        resolve,
        ctx.taints,
        warnings,
    );
    emit_branch_condition_nested_command_warnings(condition, ctx, *fallback_span, warnings);
}

/// Emit sink warnings for `[cmd …]` command substitutions nested inside a
/// branch condition (e.g. `if {[exec $x] eq "ok"} { … }`).
///
/// A nested command substitution executes exactly like any other
/// statement's, but the CFG builder's synthetic `<cond>` placeholder
/// statement (`cfg_lower::lower_if`/`lower_for`/`lower_while`) records
/// only `catch`/`regexp`-style output vars for W210 — not the real
/// command name or arguments — so `find_taint_warnings`'s per-statement
/// scan has nothing to dispatch `classify_sink` against for it. Uses
/// [`ExprNode::command_texts`] to recover each nested `[cmd …]` verbatim
/// (already the shape `classify_sink`/`emit_sink_warnings` expect, the
/// same one the `AssignValue` "owned fallback" path re-parses), resolves
/// its argument variables' SSA versions the same way as the coercion
/// scan above, and reuses the very same sink classification and warning
/// emission every other call site in this module goes through — no new
/// per-command knowledge, just a new place a command's arguments can
/// come from.
fn emit_branch_condition_nested_command_warnings<S: std::hash::BuildHasher>(
    condition: &ExprNode,
    ctx: &BranchTaintCtx<'_, S>,
    fallback_span: Span,
    warnings: &mut Vec<TaintWarning>,
) {
    for text in condition.command_texts() {
        let Some((command, call_args)) = parse_command_substitution(&text) else {
            continue;
        };
        let Some((code, sink_label)) =
            classify_sink(ctx.registry, &command, &call_args, ctx.dialect)
        else {
            continue;
        };
        let braced_var = tcl_dialect::BracedVarStyle::of_profile(ctx.dialect);
        let mut uses: HashMap<Symbol, u32> = HashMap::new();
        for name in arg_var_names(&text, braced_var) {
            if let Some(sym) = ctx.ssa.var_symbol(&name) {
                let ver = ctx.ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
                uses.insert(sym, ver);
            }
        }
        let env = TaintScan {
            uses: &uses,
            taints: ctx.taints,
            ssa: ctx.ssa,
            braced_var,
            quoted_uses: None,
        };
        let sink_call = SinkCall {
            command: &command,
            args: &call_args,
            registry: ctx.registry,
            tokens: None,
            braced_var,
        };
        emit_sink_warnings(&env, fallback_span, code, &sink_label, &sink_call, warnings);
    }
}

/// Emit one warning per tainted use flowing into a classified sink.
///
/// Deduplicates on variable name so the same variable appearing multiple
/// times in `uses` only produces one warning. For iRules sinks
/// (`IRULE3001` / `IRULE3002` / `IRULE3003` / `IRULE3004`), applies the
/// per-code mitigation masks via [`irules_sink_suppressed`] plus the
/// name-position `HEADER_TOKEN_SAFE` carve-out for IRULE3002.
/// Context for a classified sink call, bundled so
/// [`emit_sink_warnings`] stays under the 7-argument clippy limit
/// while still carrying the command name + arg slice needed for the
/// IRULE3002 name-position mitigation.
struct SinkCall<'a> {
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
    /// The document's `${…}` close rule, for the shared owner
    /// [`tcl_lexer::braced_var_name_end`] — the position-aware sink filters
    /// below all key on variable *names* scanned out of `args` (issue #1604).
    braced_var: tcl_dialect::BracedVarStyle,
}

/// The tight span of the argument word that carries `name`, using the
/// statement's per-word token spans (`call.tokens.argv`, offset by one to
/// skip the command-name slot at index 0 — `call.args` excludes the command
/// name, `argv` does not). Falls back to `fallback` (the whole-statement
/// span) when no token spans are available, or `name` isn't found in any
/// argument word (e.g. it reached the sink through an already-resolved
/// alias with no textual match). Narrows the T100/T101-family highlight
/// from "the whole sink statement" down to "the one argument word that
/// carries the tainted value" — e.g. `eval "prefix" $x "suffix"` underlines
/// only `$x`.
fn sink_arg_span(call: &SinkCall<'_>, name: &str, fallback: Span) -> Span {
    let Some(tokens) = call.tokens else {
        return fallback;
    };
    for (i, arg) in call.args.iter().enumerate() {
        if arg_var_names(arg, call.braced_var).contains(name)
            && let Some(&arg_span) = tokens.argv.get(i + 1)
        {
            return arg_span;
        }
    }
    fallback
}

/// Positional argument strings of `args` under `spec`, skipping option
/// flags (`-foo`) and the value of any option whose [`OptionSpec`] declares
/// `takes_value`. `--` ends option processing; everything after is
/// positional.
fn positional_arg_strings(spec: &tcl_registry::CommandSpec, args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a.len() > 1 && a.starts_with('-') {
            if a == "--" {
                out.extend(args[i + 1..].iter().cloned());
                break;
            }
            // Skip the option and the value word(s) it consumes (arity-aware).
            let consumed = spec
                .options
                .iter()
                .find(|o| o.matches(a))
                .map_or(0, |o| o.value_word_count(args, i));
            i += 1 + consumed;
            continue;
        }
        out.push(a.clone());
        i += 1;
    }
    out
}

/// Position-aware sink filter: `true` when a tainted variable `name`
/// occupies a *non-dangerous* argument slot for `code` and so must not trip
/// the sink.
fn sink_var_position_safe(call: &SinkCall<'_>, code: DiagCode, name: &str) -> bool {
    let (registry, command, args, braced_var) =
        (call.registry, call.command, call.args, call.braced_var);
    // A value that only reaches an `ArgRole::Channel` argument slot (e.g.
    // `puts`'s optional leading `channelId`) is a handle, not sink content
    // — regardless of which sink code classified the call. The position
    // comes from the command's registry-declared arg roles
    // (`arg_role_resolver` for `puts`, since the channel slot shifts with
    // the optional leading `-nonewline`), so a future output sink with its
    // own channel argument is covered for free with no name check here.
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let channel_positions = registry.arg_indices_for_role(command, &arg_refs, ArgRole::Channel);
    if !channel_positions.is_empty()
        && args.iter().enumerate().all(|(i, a)| {
            channel_positions.contains(&i) || !arg_var_names(a, braced_var).contains(name)
        })
    {
        return true;
    }
    let Some(spec) = registry.get(command) else {
        return false;
    };
    match code {
        // T104 SSRF — only the network-address positional slots named by
        // `taint_network_sink_args`. `None`/`Some(&[])` (positions
        // unspecified) imposes no filter.
        DiagCode::T104 => {
            var_only_in_safe_positions(spec, spec.taint_network_sink_args, args, name, braced_var)
        }
        // T100 code injection — only the code-execution slot named by
        // `taint_code_sink_args` (`apply`'s lambda `func`, `open`'s
        // pipeline-selecting `fileName` — both position 0). A tainted value
        // in any other positional slot is ordinary data (the lambda's bound
        // parameters, `open`'s access/permissions) and must not trip the
        // sink. `None` (`eval`/`uplevel`/`subst`/`exec` — the whole tail is
        // one script) or `Some(&[])` imposes no filter, so every tainted
        // argument stays dangerous as before.
        DiagCode::T100 => {
            var_only_in_safe_positions(spec, spec.taint_code_sink_args, args, name, braced_var)
        }
        _ => false,
    }
}

/// `true` when `name` appears in `args` only *outside* the dangerous
/// positional slots named by `positions` (option flags skipped) — i.e. the
/// value never reaches a dangerous slot, so the sink must not fire.
/// `None` or `Some(&[])` means "positions unspecified": no filter, so this
/// returns `false` (the value is treated as potentially dangerous). Shared
/// by the T100 (`taint_code_sink_args`) and T104 (`taint_network_sink_args`)
/// position filters in [`sink_var_position_safe`].
fn var_only_in_safe_positions(
    spec: &tcl_registry::CommandSpec,
    positions: Option<&'static [u8]>,
    args: &[String],
    name: &str,
    braced_var: tcl_dialect::BracedVarStyle,
) -> bool {
    let Some(positions) = positions else {
        return false;
    };
    if positions.is_empty() {
        return false;
    }
    let positionals = positional_arg_strings(spec, args);
    let in_dangerous_slot = positions.iter().any(|&p| {
        positionals
            .get(p as usize)
            .is_some_and(|s| arg_var_names(s, braced_var).contains(name))
    });
    !in_dangerous_slot
}

/// `true` when one of `args` is a `[list <head> …]` command substitution
/// whose constructed-list command word (`<head>`) is a literal known
/// registry command and `name` is referenced in that argument. The tainted
/// variable is then a *quoted argument* of the list, never the command
/// word, so `eval`/`uplevel`/`interp eval` of the list runs no injected
/// command. `[list $x …]` (variable head) and `[list]`-free args return
/// `false`.
fn list_wrapped_arg_command_is_literal(call: &SinkCall<'_>, name: &str) -> bool {
    let (registry, args, braced_var) = (call.registry, call.args, call.braced_var);
    for arg in args {
        let trimmed = arg.trim();
        let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
            continue;
        };
        let mut words = inner.split_whitespace();
        if words.next() != Some("list") {
            continue;
        }
        // First element of the constructed list = the command word.
        let Some(head) = words.next() else {
            continue;
        };
        // The head must be a literal known command, not a substitution.
        if head.starts_with('$') || head.contains(['[', '$', '{']) {
            continue;
        }
        if registry.get(head).is_none() {
            continue;
        }
        // `name` must actually flow through this list arg (and, since the
        // head is a literal, only at an argument position ≥ 1).
        if arg_var_names(arg, braced_var).contains(name) {
            return true;
        }
    }
    false
}

/// Split `arg` into its residual text (everything outside top-level `[...]`
/// command substitutions) and the list of those top-level `[...]` slices.
/// Brackets are ASCII, so the byte-range slicing stays on char boundaries.
fn split_top_level_cmd_subs(arg: &str) -> (String, Vec<&str>) {
    let mut residual = String::new();
    let mut subs = Vec::new();
    let b = arg.as_bytes();
    let mut i = 0;
    let mut seg_start = 0;
    while i < b.len() {
        if b[i] == b'[' {
            residual.push_str(&arg[seg_start..i]);
            let start = i;
            let mut depth = 0i32;
            while i < b.len() {
                match b[i] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            subs.push(&arg[start..i]);
            seg_start = i;
        } else {
            i += 1;
        }
    }
    residual.push_str(&arg[seg_start..]);
    (residual, subs)
}

/// True when every appearance of `name` in the sink arguments is consumed by an
/// embedded sanitiser command substitution, so the value reaching the sink is
/// clean — e.g. `puts [string length $x]` outputs the integer length, never
/// `$x`'s content (tclsh-verified). Conservative: a bare `$name` outside any
/// `[...]`, or one inside a *non*-sanitiser substitution, returns `false` (the
/// taint reaches the sink). Mirrors the carve-out the `expr`/`word_taint` path
/// already applies, which `emit_sink_warnings` (iterating raw SSA uses) lacked.
fn var_consumed_by_sanitiser(call: &SinkCall<'_>, name: &str) -> bool {
    let (registry, args, braced_var) = (call.registry, call.args, call.braced_var);
    let mut seen = false;
    for arg in args {
        if !arg_var_names(arg, braced_var).contains(name) {
            continue;
        }
        seen = true;
        let (residual, subs) = split_top_level_cmd_subs(arg);
        // A bare reference outside any command substitution reaches the sink.
        if arg_var_names(&residual, braced_var).contains(name) {
            return false;
        }
        // Each top-level substitution that references `name` must be a sanitiser
        // (a sanitiser's result is a clean bounded value regardless of nesting).
        for sub in subs {
            if !arg_var_names(sub, braced_var).contains(name) {
                continue;
            }
            let Some((cmd, sub_args)) = parse_command_substitution(sub) else {
                return false;
            };
            let refs: Vec<&str> = sub_args.iter().map(String::as_str).collect();
            if !is_sanitiser(registry, &cmd, &refs) {
                return false;
            }
        }
    }
    seen
}

fn emit_sink_warnings<S: std::hash::BuildHasher>(
    env: &TaintScan<'_, S>,
    span: Span,
    code: DiagCode,
    sink_label: &str,
    call: &SinkCall<'_>,
    warnings: &mut Vec<TaintWarning>,
) {
    let (uses, taints, ssa) = (env.uses, env.taints, env.ssa);
    let mut emitted: FxHashSet<Symbol> = FxHashSet::default();
    for (&sym, &ver) in uses {
        let name = ssa.var_name(sym);
        if is_seeded_global_v0(name, ver) || emitted.contains(&sym) {
            continue;
        }
        let t = taints
            .get(&(sym, ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if !t.is_tainted() {
            continue;
        }
        if env.quoted_uses.is_some_and(|quoted| quoted.contains(&sym))
            && !quoted_sink_use_is_evaluated(call, name)
        {
            continue;
        }
        // The value reaching the sink is clean when every appearance of `name`
        // in the sink arguments is consumed by an embedded sanitiser
        // substitution — `puts [string length $x]` outputs the integer length,
        // not `$x` (tclsh-verified). The expr-operand path applies this via
        // word_taint; mirror it here for the direct sink-argument path.
        if var_consumed_by_sanitiser(call, name) {
            continue;
        }
        // Per-code mitigation suppression (IRULE3001–3004).
        if irules_sink_suppressed(code, t) {
            continue;
        }
        // Registry-declared sink-safe colour (e.g. `exec` ← SHELL_ATOM): a
        // tainted value carrying the sink's safe colour cannot break out of its
        // dangerous slot — an IP/port atom can't word-split or inject shell
        // metacharacters into an `exec` argument. LIST_CANONICAL (eval/uplevel)
        // is excluded: its safety is position-dependent (the list *head* must be
        // a literal known command), handled by `list_wrapped_arg_command_is_literal`
        // below — a blanket check would wrongly clear `eval [list $raw]`, where
        // the tainted value is the command word.
        if let Some(safe) = tcl_registry::taint::taint_sink_safe_colour(call.registry, call.command)
        {
            let safe = reg_colour(safe);
            if safe != TaintColour::LIST_CANONICAL && t.colours.contains(safe) {
                continue;
            }
        }
        if code == DiagCode::Irule3002
            && irule3002_name_position_safe(call.registry, call.command, call.args, name, t)
        {
            continue;
        }
        // Position-aware sink filter: a tainted variable only trips the
        // sink when it occupies a *dangerous* argument slot (the `puts`
        // content arg, a `taint_network_sink_args` network-address slot).
        if sink_var_position_safe(call, code, name) {
            continue;
        }
        // `eval`/`uplevel`/`interp eval [list <known-cmd> $v …]`: the
        // command word of the constructed list is a literal known command,
        // so the tainted `$v` is a quoted argument, not the command word —
        // no code-injection vector LIST_CANONICAL
        // head-literal filter).
        if matches!(code, DiagCode::T100 | DiagCode::T105)
            && list_wrapped_arg_command_is_literal(call, name)
        {
            continue;
        }
        // T104 / T105 mitigations: a
        // validated network address (IP / port / FQDN colour) clears
        // T104; a canonical list (`LIST_CANONICAL`) clears T105.
        if code == DiagCode::T104
            && t.colours
                .intersects(TaintColour::IP_ADDRESS | TaintColour::PORT | TaintColour::FQDN)
        {
            continue;
        }
        if code == DiagCode::T105 && t.colours.intersects(TaintColour::LIST_CANONICAL) {
            continue;
        }
        let message = match code {
            DiagCode::T100 => format!(
                "Tainted variable ${name} flows into {sink_label}; \
                 possible code injection"
            ),
            DiagCode::T101 => format!(
                "Tainted variable ${name} flows into {sink_label}; \
                 output may contain injected content"
            ),
            DiagCode::Irule3001 => format!(
                "Tainted variable ${name} in HTTP response body ({sink_label}); \
                 risk of XSS or content injection"
            ),
            DiagCode::Irule3002 => format!(
                "Tainted variable ${name} in HTTP header/cookie value ({sink_label}); \
                 risk of header injection"
            ),
            DiagCode::Irule3003 => format!(
                "Tainted variable ${name} in log output ({sink_label}); \
                 risk of log injection or log forging"
            ),
            DiagCode::Irule3004 => format!(
                "Tainted variable ${name} in redirect URL ({sink_label}); \
                 risk of open redirect"
            ),
            DiagCode::T104 => format!(
                "Tainted variable ${name} in network address argument of {sink_label}; \
                 risk of SSRF (server-side request forgery)"
            ),
            DiagCode::T105 => format!(
                "Tainted variable ${name} in {sink_label} script argument; \
                 risk of cross-interpreter code injection"
            ),
            _ => format!("Tainted variable ${name} flows into {sink_label}"),
        };
        warnings.push(TaintWarning {
            span: sink_arg_span(call, name, span),
            variable: name.to_owned(),
            sink_command: sink_label.to_owned(),
            code,
            message,
            replacement: None,
            fixes: Vec::new(),
        });
        emitted.insert(sym);
    }
}

/// Whether a brace-quoted occurrence is in a registry-declared Tcl body slot.
/// A quoted word is literal data for `exec`/`puts`, but deferred source for
/// `eval`, `uplevel`, and `interp eval`; the role/trait projection owns that
/// distinction and the concatenating eval family extends through the tail.
fn quoted_sink_use_is_evaluated(call: &SinkCall<'_>, name: &str) -> bool {
    let arg_refs: Vec<&str> = call.args.iter().map(String::as_str).collect();
    let traits = call
        .registry
        .invocation_traits(call.command, &arg_refs, DialectSet::empty());
    let is_eval = traits.contains(Traits::EVALUATES_CODE)
        || classify_network_interp_sinks(call.registry, call.command, call.args)
            .iter()
            .any(|(code, _)| *code == DiagCode::T105);
    if !is_eval {
        return false;
    }
    let mut indices = call
        .registry
        .arg_indices_for_role(call.command, &arg_refs, ArgRole::Body);
    if traits.contains(Traits::SCRIPT_CONCATENATES_ARGS)
        && let Some(&first) = indices.first()
    {
        indices = (first..call.args.len()).collect();
    }
    indices.into_iter().any(|index| {
        call.args
            .get(index)
            .is_some_and(|arg| arg_var_names(arg, call.braced_var).contains(name))
    })
}

/// Emit T102 warnings for option injection into a command declaring a `--`
/// option terminator.
///
/// A T102 violation occurs when a tainted pure-variable-reference argument
/// is passed to a command at a position that can be misinterpreted as a
/// command option (flag), without a preceding `--` terminator to end flag
/// parsing.
///
/// Example: `regexp $pattern $string` where `$pattern` is tainted —
/// if `$pattern` starts with `-`, it will be treated as a `regexp` flag
/// rather than the pattern, producing option injection (T102).
/// Whether `arg` could expand to a string beginning with `-` and thus be
/// (mis)interpreted as a switch — a leading literal `-`, or a leading
/// substitution (`$` / `[` / `{*}` expansion) whose runtime value is
/// unknown. Any other leading literal char is a definite positional and
/// ends option scanning.
fn arg_can_be_option(arg: &str) -> bool {
    match arg.as_bytes().first() {
        None => false,
        Some(&c) => c == b'-' || c == b'$' || c == b'[' || arg.starts_with("{*}"),
    }
}

/// Return the argument indexes still within Tcl's option-scanning region.
/// Tcl scans for `-switch` args from `scan_start` until the first definite
/// positional literal (one that cannot begin with `-`) or `--`. A literal
/// `-option` that takes a value also consumes the following arg.
/// `reserved_trailing_words` excludes a command's structurally-fixed
/// trailing words (e.g. `switch`'s `string` + pattern-list) from the scan
/// entirely, regardless of their shape — see
/// [`tcl_registry::spec::CommandSpec::reserved_trailing_words`].
fn option_scan_region(
    args: &[String],
    scan_start: usize,
    options: &[tcl_registry::hover::OptionSpec],
    reserved_trailing_words: usize,
) -> HashSet<usize> {
    let mut region: HashSet<usize> = HashSet::new();
    let mut i = scan_start;
    let n = args.len().saturating_sub(reserved_trailing_words);
    while i < n {
        let arg = &args[i];
        if arg == "--" {
            region.insert(i);
            break;
        }
        if !arg_can_be_option(arg) {
            // Definite positional literal → option scanning ends here.
            break;
        }
        region.insert(i);
        if arg.starts_with('-')
            && let Some(opt) = options.iter().find(|o| o.matches(arg))
        {
            let consumed = opt.value_word_count(args, i);
            if consumed > 0 {
                i += 1 + consumed;
                continue;
            }
        }
        i += 1;
    }
    region
}

/// Resolve a tight per-argument span for an argument-anchored taint
/// diagnostic (T102 option injection, W313 destructive-file path), so the
/// highlighted range (and any fix) covers only the offending argument
/// rather than the whole statement.
///
/// - `Call` / `Barrier`: the argument's own token span, read directly off
///   `tokens.argv` (offset by 1 to skip the command-name word — `argv[0]`
///   is the command, `argv[i + 1]` is `args[i]`).
/// - `AssignValue` (`set x [cmd ...]`): the embedded command has no
///   token of its own on *this* statement — `tokens.argv[2]` is the
///   whole bracketed value word (`argv[0]` = `set`, `argv[1]` = the
///   variable name, `argv[2]` = the `[...]` value), so the argument's
///   position is re-located inside that word's own text via
///   `parse_command_substitution_with_spans` and offset by the value
///   word's span start.
///
/// Returns `None` when the token data needed isn't available (e.g.
/// synthetic/test-built IR with `tokens: None`) — callers fall back to
/// the whole-statement span in that case. A tight span is a
/// highlighting nicety, never a correctness requirement, so this never
/// panics or guesses.
fn arg_index_span(stmt: &Statement, arg_index: usize) -> Option<Span> {
    match stmt {
        Statement::Call {
            tokens: Some(t), ..
        }
        | Statement::Barrier {
            tokens: Some(t), ..
        } => t.argv.get(arg_index + 1).copied(),
        Statement::AssignValue {
            value,
            tokens: Some(t),
            ..
        } => {
            let value_span = t.argv.get(2)?;
            let (_cmd, args) = parse_command_substitution_with_spans(value)?;
            let (_text, arg_span) = args.get(arg_index)?;
            Some(Span::new(
                value_span.start() + arg_span.start(),
                value_span.start() + arg_span.end(),
            ))
        }
        _ => None,
    }
}

/// Emit `T102` (option injection) for tainted variables that sit in an
/// option-scanning position of a command declaring a `--` terminator.
///
/// The option-terminator profile
/// (`resolve_option_terminator`) supplies the subcommand-aware command
/// label and the scan start, `option_scan_region` filters positions, and
/// the `T102_SAFE` colour set mitigates. Each warning is anchored at the
/// offending argument's own span (falling back to the whole statement's
/// span when the tight span can't be resolved) and carries a `--`
/// insertion fix alongside it. Takes `sink_call` (rather than separate
/// `command`/`args`/`registry` parameters) since that's already built at
/// the one call site for the primary sink classification — reusing it
/// keeps this under clippy's argument-count lint without an `#[allow]`.
fn emit_option_injection<S: std::hash::BuildHasher>(
    sink_call: &SinkCall<'_>,
    env: &TaintScan<'_, S>,
    span: Span,
    dialect: Option<&tcl_dialect::DialectProfile>,
    warnings: &mut Vec<TaintWarning>,
    stmt: &Statement,
) {
    // `tokens` is unused here: T102's own `arg_index_span(stmt, arg_index)`
    // re-derives the tight per-argument span directly from `stmt` (it
    // already knows the exact `arg_index` from `option_scan_region`'s
    // iteration, so an index-based lookup is more precise than
    // `sink_arg_span`'s name-based scan, which could mismatch when the
    // same variable name occurs in more than one scanned position).
    let &SinkCall {
        command,
        args,
        registry,
        tokens: _,
        braced_var: _,
    } = sink_call;
    let (uses, taints, ssa) = (env.uses, env.taints, env.ssa);
    let args_str: Vec<&str> = args.iter().map(String::as_str).collect();
    let Some(profile) =
        registry.resolve_option_terminator(command, &args_str, dialect_to_set(dialect))
    else {
        // No `--` terminator declared → no option-injection sink.
        return;
    };

    // Ensemble subcommands report a compound label ("file delete"),
    // mirroring `cmd_label`.
    let cmd_label = match profile.subcommand {
        Some(sub) => format!("{command} {sub}"),
        None => command.to_owned(),
    };

    let region = option_scan_region(
        args,
        profile.scan_start,
        profile.options,
        profile.reserved_trailing_words,
    );
    if region.is_empty() {
        return;
    }

    // One warning per tainted variable in an in-region position. Iterate
    // arg indexes in order (then names within an arg sorted) for a
    // deterministic, source-ordered emission.
    let mut ordered: Vec<usize> = region.into_iter().collect();
    ordered.sort_unstable();
    let mut emitted: FxHashSet<Symbol> = FxHashSet::default();
    for i in ordered {
        let Some(arg) = args.get(i) else { continue };
        let mut names: Vec<String> = arg_var_names(arg, env.braced_var).into_iter().collect();
        names.sort_unstable();
        for var in names {
            let Some(sym) = ssa.var_symbol(&var) else {
                continue;
            };
            if env.quoted_uses.is_some_and(|quoted| quoted.contains(&sym)) {
                continue;
            }
            if emitted.contains(&sym) {
                continue;
            }
            let Some(&ver) = uses.get(&sym) else { continue };
            if is_seeded_global_v0(&var, ver) {
                continue;
            }
            let t = taints
                .get(&(sym, ver))
                .copied()
                .unwrap_or(TaintLattice::clean());
            if !t.is_tainted() {
                continue;
            }
            // Suppress when a mitigating colour proves the value cannot
            // start with `-` (PATH_PREFIXED / NON_DASH_PREFIXED /
            // IP_ADDRESS / PORT / FQDN) — the T102_SAFE set.
            if t.colours.intersects(TaintColour::T102_SAFE) {
                continue;
            }
            // Anchor the diagnostic (and its fix) at the offending
            // argument itself when the source tokens are available;
            // fall back to the whole statement's span otherwise (no
            // fix in that case — a fabricated span could misplace the
            // insertion, e.g. before the command name rather than the
            // argument).
            let (warn_span, fixes) = match arg_index_span(stmt, i) {
                Some(tight) => (
                    tight,
                    vec![CodeFix {
                        span: Span::new(tight.start(), tight.start()),
                        new_text: "-- ".to_owned(),
                        description: "Insert '--' option terminator".to_owned(),
                        // T102: `--` stops a tainted value being read as an option — the
                        // fix, and a change for a call that meant it as one.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                ),
                None => (span, Vec::new()),
            };
            warnings.push(TaintWarning {
                span: warn_span,
                variable: var.clone(),
                sink_command: cmd_label.clone(),
                code: DiagCode::T102,
                message: format!(
                    "Tainted variable ${var} in option position of '{cmd_label}' \
                     without '--' terminator; risk of option injection"
                ),
                replacement: None,
                fixes,
            });
            emitted.insert(sym);
        }
    }
}

// IRULE3101 — setter-constraint violations

/// Find setter-constraint violations (IRULE3101). Currently constrains
/// `HTTP::uri` / `HTTP::path` setters to paths beginning with `/`.
///
/// Dialect-gated: returns an empty vector unless `dialect` is
/// `"f5-irules"` / `"irules"`. The gate is applied internally (defense
/// in depth) so a caller outside `compiler_checks::run_all_checks`
/// can't accidentally emit IRULE3101 errors against user-defined
/// commands that happen to be named `HTTP::uri` / `HTTP::path`.
///
/// Three cases per constraint:
///
/// 1. **Literal** (not `$`-prefixed, no `[`) — check the prefix directly.
/// 2. **Pure var-ref** — look up the SSA-resolved taint colour; suppress
///    when `PATH_PREFIXED | PATH_NORMALISED | PATH_BOUNDED` is set.
/// 3. **Dynamic expression** (interpolation, command sub) — always warn.
#[must_use]
pub fn find_setter_constraint_warnings<S: std::hash::BuildHasher, E: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    dialect: Option<&tcl_dialect::DialectProfile>,
) -> Vec<TaintWarning> {
    let mut out: Vec<TaintWarning> = Vec::new();
    if !is_irules_dialect(dialect) {
        return out;
    }
    let safe_path_colours =
        TaintColour::PATH_PREFIXED | TaintColour::PATH_NORMALISED | TaintColour::PATH_BOUNDED;

    for bn in cfg_order(cfg) {
        if !executable_blocks.contains(&bn) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&bn) else {
            continue;
        };

        for ssa_stmt in &ssa_block.statements {
            let Statement::Call { command, args, .. } = &ssa_stmt.statement else {
                continue;
            };
            let span = ssa_stmt.statement.span();
            // The setter constraints are command metadata, read straight
            // from the registry spec (`HTTP::uri` / `HTTP::path` declare
            // their `/`-prefix IRULE3101 rule) rather than a hardcoded
            // table here.
            for constraint in tcl_registry::taint::setter_constraints(registry, command) {
                let Some(arg_val) = args.get(constraint.arg_index as usize) else {
                    continue;
                };
                let stripped = arg_val.trim();
                let warn = |variable: String| TaintWarning {
                    span,
                    variable,
                    sink_command: command.clone(),
                    code: constraint.code,
                    message: constraint.message.to_owned(),
                    replacement: None,
                    fixes: Vec::new(),
                };

                // Literal: neither `$` nor `[`.
                if !stripped.starts_with('$') && !stripped.contains('[') {
                    if !stripped.starts_with(constraint.required_prefix) {
                        out.push(warn(String::new()));
                    }
                    continue;
                }

                // Pure variable reference: check SSA-resolved taint colour.
                if is_pure_var_ref(stripped) {
                    let var_name = normalise_var_name(stripped);
                    let sym = ssa.var_symbol(var_name);
                    let ver = sym
                        .and_then(|s| ssa_stmt.uses.get(&s))
                        .copied()
                        .unwrap_or(0);
                    let t = sym
                        .and_then(|s| taints.get(&(s, ver)))
                        .copied()
                        .unwrap_or(TaintLattice::clean());
                    if t.is_tainted() && t.colours.intersects(safe_path_colours) {
                        continue;
                    }
                    out.push(warn(var_name.to_owned()));
                    continue;
                }

                // Dynamic (interpolation, command sub, mixed) — always warn.
                out.push(warn(String::new()));
            }
        }
    }

    out
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sccp::SccpResult;

    fn simple_sccp(blocks: &[BlockId]) -> SccpResult {
        SccpResult {
            values: HashMap::new(),
            executable_blocks: blocks.iter().copied().collect(),
            executable_edges: HashSet::new(),
            constant_branches: Vec::new(),
        }
    }

    /// Bare intra-procedural propagation with no optional inputs.
    fn plain_taints(
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        sccp: &SccpResult,
        registry: &CommandRegistry,
    ) -> HashMap<ValueKey, TaintLattice> {
        let instance_classes = local_instance_classes(cfg, registry);
        propagate_taints(
            &TaintGraph::new(cfg, ssa, sccp),
            registry,
            None,
            None,
            None,
            None,
            None,
            &instance_classes,
        )
    }

    #[test]
    fn clean_and_tainted_constructors() {
        let c = TaintLattice::clean();
        assert!(!c.is_tainted());
        let t = TaintLattice::tainted();
        assert!(t.is_tainted());
    }

    #[test]
    fn join_propagates_taint_intersects_mitigations() {
        // Two TAINTED operands: taint unions, mitigations are must-have so only
        // the colour present on both survives the intersection.
        let a = TaintLattice {
            colours: TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::PATH_PREFIXED,
        };
        let b = TaintLattice {
            colours: TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::NON_DASH_PREFIXED,
        };
        let j = a.join(b);
        assert!(j.colours.contains(TaintColour::TAINTED));
        assert!(j.colours.contains(TaintColour::CRLF_FREE));
        assert!(!j.colours.contains(TaintColour::PATH_PREFIXED));
        assert!(!j.colours.contains(TaintColour::NON_DASH_PREFIXED));
    }

    #[test]
    fn join_with_untainted_is_identity() {
        // A clean/untainted operand is the join identity: it contributes no
        // taint, so it must not dilute the tainted operand's mitigation colours.
        // Joining with the annihilating empty set previously wrongly stripped
        // PATH_PREFIXED.
        let tainted = TaintLattice {
            colours: TaintColour::TAINTED | TaintColour::PATH_PREFIXED,
        };
        assert_eq!(tainted.join(TaintLattice::clean()).colours, tainted.colours);
        assert_eq!(TaintLattice::clean().join(tainted).colours, tainted.colours);
        // An untainted operand that happens to carry colours is still the
        // identity — its taint contribution is nil, so it cannot remove a
        // mitigation from the tainted side.
        let untainted_coloured = TaintLattice {
            colours: TaintColour::CRLF_FREE,
        };
        assert_eq!(tainted.join(untainted_coloured).colours, tainted.colours);
    }

    #[test]
    fn with_and_sanitised() {
        let v = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
        assert!(v.is_tainted());
        assert!(v.colours.contains(TaintColour::CRLF_FREE));
        let s = v.sanitised();
        assert!(!s.is_tainted());
        assert!(s.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn t102_safe_mask_excludes_tainted() {
        assert!(!TaintColour::T102_SAFE.contains(TaintColour::TAINTED));
        assert!(TaintColour::T102_SAFE.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn crlf_safe_mask_composition() {
        // `CRLF_SAFE = CRLF_FREE | IP_ADDRESS | PORT | FQDN`.
        // HEADER_TOKEN_SAFE only suppresses IRULE3002 in the name position
        // (handled by `irule3002_name_position_safe`); HTML_ESCAPED / URL_ENCODED
        // do not prove CRLF-injection safety in the value position.
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::CRLF_FREE));
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::IP_ADDRESS));
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::PORT));
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::FQDN));
        assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::HEADER_TOKEN_SAFE));
        assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::HTML_ESCAPED));
        assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::URL_ENCODED));
        assert!(!TaintColour::CRLF_SAFE.contains(TaintColour::TAINTED));
    }

    /// A deeply-nested `[a [a [a … x]]]` command-substitution word and a
    /// deeply-nested `ExprNode` tree, plus a minimal [`TaintCtx`], for the
    /// two ctx-taking recursion caps below.
    fn deep_bracket_word(depth: usize) -> String {
        format!("{}x{}", "[a ".repeat(depth), "]".repeat(depth))
    }

    /// Regression coverage for issue #996: `word_taint` recurses once per
    /// nested `[cmd …]` command substitution inside a single word's raw text
    /// (Tier 1B), and `expr_command_taint` recurses once per `ExprNode`
    /// operator-tree level (Tier 1A) — both genuinely unbounded before this
    /// fix, independent of any statement-tree cap. Empirically each
    /// overflowed the native stack (SIGABRT) in the low thousands of levels
    /// on a 2 MiB thread (`cargo test`'s default). 3000 is comfortably past
    /// that crash range and past both caps (256); the assertion is that each
    /// returns at all.
    #[test]
    fn deeply_nested_taint_walks_survive() {
        use crate::cfg::Function;
        use crate::ssa::SsaFunction;

        let registry = CommandRegistry::build_default();
        let cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        let ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let uses: HashMap<Symbol, u32> = HashMap::new();
        let taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
        let ctx = TaintCtx {
            registry: &registry,
            ssa: &ssa,
            interproc: None,
            known_procs: None,
            caller_qname: None,
            dialect: None,
            taint_summaries: None,
            instance_classes: None,
            source_position: None,
        };

        // Tier 1B: nested `[a [a [a … x]]]` command-substitution text.
        let _ = word_taint(&deep_bracket_word(3000), &uses, &taints, ctx);

        // Tier 1A: a 3000-deep `ExprNode` tree (nested unary `!`).
        let mut node = ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: crate::expr_ast::UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let _ = expr_command_taint(&node, &uses, &taints, ctx, 0);
    }

    /// Regression coverage for issue #996: `extract_guard_var` and
    /// `collect_coercion_operands` each recurse once per `ExprNode` level
    /// with no depth cap before this fix. 3000-deep trees are past the crash
    /// range and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that each
    /// returns at all.
    #[test]
    fn deeply_nested_expr_guard_and_coercion_walks_survive() {
        // A 3000-deep nested-unary tree drives both walkers' recursion.
        let mut node = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: crate::expr_ast::UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let _ = extract_guard_var(&node, false, 0);
        let mut out = Vec::new();
        collect_coercion_operands(&node, true, &mut out, 0);
    }

    #[test]
    fn propagate_taints_gets_is_source() {
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let registry = CommandRegistry::build_default();

        // Minimal CFG: entry block with `set x [gets stdin]`.
        let stmt = Statement::AssignValue {
            span: Span::new(0, 20),
            name: "x".into(),
            name_braced: false,
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(stmt.clone());
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let x = ssa.intern_var("x");
        let ssa_stmt = SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: [(x, 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        assert!(
            taints.get(&(x, 1)).is_some_and(|t| t.is_tainted()),
            "gets stdin result should be tainted"
        );
    }

    #[test]
    fn find_taint_warnings_eval_with_tainted_var() {
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let registry = CommandRegistry::build_default();

        // Statements: set x [gets stdin]; eval $x
        let assign = Statement::AssignValue {
            span: Span::new(0, 12),
            name: "x".into(),
            name_braced: false,
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let eval_call = Statement::Call {
            span: Span::new(13, 20),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["$x".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };

        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(assign.clone());
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(eval_call.clone());
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let x = ssa.intern_var("x");
        let ssa_assign = SsaStatement {
            statement: assign,
            uses: HashMap::new(),
            defs: [(x, 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let ssa_eval = SsaStatement {
            statement: eval_call,
            uses: [(x, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };

        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_assign, ssa_eval],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
            &HashSet::new(),
        );

        assert!(
            warnings
                .iter()
                .any(|w| w.code == DiagCode::T100 && w.variable == "x"),
            "expected T100 for tainted $x passed to eval, got {warnings:?}"
        );
    }

    /// Wire `set x [gets stdin]` (a taint source) followed by `sink`
    /// (which uses `$x`) and return the taint warnings.  Shared by the
    /// T104 / T105 sink tests.
    fn warnings_for_tainted_sink(sink: Statement, sink_uses: &[(&str, u32)]) -> Vec<TaintWarning> {
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let registry = CommandRegistry::build_default();
        let assign = Statement::AssignValue {
            span: Span::new(0, 12),
            name: "x".into(),
            name_braced: false,
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        {
            let b = cfg.blocks.get_mut(&entry).unwrap();
            b.statements.push(assign.clone());
            b.statements.push(sink.clone());
            b.terminator = Some(Terminator::Return {
                value: None,
                span: None,
                expr: None,
                braced: false,
            });
        }
        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let ssa_assign = SsaStatement {
            statement: assign,
            uses: HashMap::new(),
            defs: [(ssa.intern_var("x"), 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let ssa_sink = SsaStatement {
            statement: sink,
            uses: sink_uses
                .iter()
                .map(|&(n, v)| (ssa.intern_var(n), v))
                .collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_assign, ssa_sink],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
            &HashSet::new(),
        )
    }

    fn call_stmt(command: &str, args: &[&str]) -> Statement {
        Statement::Call {
            span: tcl_lexer::Span::new(13, 30),
            command: command.into(),
            canonical_command: None,
            args: args.iter().map(|a| (*a).to_string()).collect(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    #[test]
    fn t104_ssrf_for_tainted_socket_address() {
        // `socket $x 80` with tainted `$x` → SSRF (T104).
        let sink = call_stmt("socket", &["$x", "80"]);
        let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
        assert!(
            w.iter()
                .any(|w| w.code == DiagCode::T104 && w.variable == "x"),
            "expected T104 for tainted socket address; got {w:?}"
        );
    }

    #[test]
    fn t105_cross_interp_for_tainted_interp_eval() {
        // `interp eval $child $x` with tainted `$x` → cross-interp (T105).
        let sink = call_stmt("interp", &["eval", "$child", "$x"]);
        let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
        let t105 = w
            .iter()
            .find(|w| w.code == DiagCode::T105 && w.variable == "x");
        assert!(t105.is_some(), "expected T105 for interp eval; got {w:?}");
        assert_eq!(t105.unwrap().sink_command, "interp eval");
    }

    #[test]
    fn t105_cross_interp_for_tainted_console_eval() {
        // `console eval $x` with tainted `$x` → cross-interp (T105): the
        // script runs in the separate console interpreter, the same
        // category as `interp eval` (issue #925).
        let sink = call_stmt("console", &["eval", "$x"]);
        let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
        let t105 = w
            .iter()
            .find(|w| w.code == DiagCode::T105 && w.variable == "x");
        assert!(t105.is_some(), "expected T105 for console eval; got {w:?}");
        assert_eq!(t105.unwrap().sink_command, "console eval");
    }

    #[test]
    fn t105_cross_interp_for_tainted_consoleinterp_eval_and_record() {
        // `consoleinterp eval $x` / `consoleinterp record $x` with tainted
        // `$x` → cross-interp (T105): both cross back into the interpreter
        // the console is attached to (issue #925 follow-up).
        for sub in ["eval", "record"] {
            let sink = call_stmt("consoleinterp", &[sub, "$x"]);
            let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
            let t105 = w
                .iter()
                .find(|w| w.code == DiagCode::T105 && w.variable == "x");
            assert!(
                t105.is_some(),
                "expected T105 for consoleinterp {sub}; got {w:?}"
            );
            assert_eq!(t105.unwrap().sink_command, format!("consoleinterp {sub}"));
        }
    }

    #[test]
    fn t106_double_encode_through_uri_encode() {
        // `set x [URI::encode $tainted]` stamps URL_ENCODED on x; passing
        // x back through `URI::encode` double-encodes → T106.
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_dialect::DialectSet::IRULES);

        // set x [gets stdin]      (taint source)
        // set y [URI::encode $x]  (x tainted → y URL_ENCODED)
        // URI::encode $y          (y already URL_ENCODED → T106)
        let s0 = Statement::AssignValue {
            span: Span::new(0, 12),
            name: "x".into(),
            name_braced: false,
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let s1 = Statement::AssignValue {
            span: Span::new(13, 35),
            name: "y".into(),
            name_braced: false,
            value: "[URI::encode $x]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let s2 = call_stmt("URI::encode", &["$y"]);

        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        {
            let b = cfg.blocks.get_mut(&entry).unwrap();
            b.statements.push(s0.clone());
            b.statements.push(s1.clone());
            b.statements.push(s2.clone());
            b.terminator = Some(Terminator::Return {
                value: None,
                span: None,
                expr: None,
                braced: false,
            });
        }
        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let x = ssa.intern_var("x");
        let y = ssa.intern_var("y");
        let ssa_s0 = SsaStatement {
            statement: s0,
            uses: HashMap::new(),
            defs: [(x, 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let ssa_s1 = SsaStatement {
            statement: s1,
            uses: [(x, 1u32)].into_iter().collect(),
            defs: [(y, 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let ssa_s2 = SsaStatement {
            statement: s2,
            uses: [(y, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_s0, ssa_s1, ssa_s2],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        // The transform colour must have propagated to y.
        assert!(
            taints
                .get(&(y, 1))
                .is_some_and(|t| t.colours.intersects(TaintColour::URL_ENCODED)),
            "y should carry URL_ENCODED after URI::encode; got {:?}",
            taints.get(&(y, 1))
        );
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::irules()),
            &HashSet::new(),
        );
        let t106 = warnings
            .iter()
            .find(|w| w.code == DiagCode::T106 && w.variable == "y");
        assert!(
            t106.is_some(),
            "expected T106 double-encode; got {warnings:?}"
        );
        assert!(
            t106.unwrap().message.contains("already URL-encoded"),
            "{:?}",
            t106.unwrap().message
        );
    }

    /// Run W313 over a single-block function whose SSA statements are built by
    /// `build` against the function's variable interner.
    fn w313_warnings(
        build: impl FnOnce(&mut SsaFunction) -> Vec<crate::ssa::SsaStatement>,
    ) -> Vec<TaintWarning> {
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction};

        let registry = CommandRegistry::build_default();
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let ssa_stmts = build(&mut ssa);
        {
            let b = cfg.blocks.get_mut(&entry).unwrap();
            for s in &ssa_stmts {
                b.statements.push(s.statement.clone());
            }
            b.terminator = Some(Terminator::Return {
                value: None,
                span: None,
                expr: None,
                braced: false,
            });
        }
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: ssa_stmts,
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        find_destructive_file_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
        )
    }

    fn file_call(args: &[&str]) -> Statement {
        Statement::Call {
            span: tcl_lexer::Span::new(0, 20),
            command: "file".into(),
            canonical_command: Some("::file".into()),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    #[test]
    fn w313_variable_path_to_file_delete() {
        use crate::ssa::SsaStatement;
        let w = w313_warnings(|ssa| {
            vec![SsaStatement {
                statement: file_call(&["delete", "$p"]),
                uses: [(ssa.intern_var("p"), 1u32)].into_iter().collect(),
                defs: HashMap::new(),
                may_defs: std::collections::HashSet::new(),
                quoted_uses: std::collections::HashSet::new(),
            }]
        });
        let d = w.iter().find(|w| w.code == DiagCode::W313).expect("W313");
        assert_eq!(d.variable, "p");
        assert!(
            d.message
                .contains("file delete with a variable path ($p) risks path-traversal"),
            "{}",
            d.message
        );
    }

    #[test]
    fn w313_normalised_path_gets_softer_message() {
        use crate::ssa::SsaStatement;
        use tcl_lexer::Span;
        // set p [file normalize $base]; file delete $p
        let assign = Statement::AssignValue {
            span: Span::new(0, 25),
            name: "p".into(),
            name_braced: false,
            value: "[file normalize $base]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let w = w313_warnings(|ssa| {
            let base = ssa.intern_var("base");
            let p = ssa.intern_var("p");
            let s0 = SsaStatement {
                statement: assign,
                uses: [(base, 0u32)].into_iter().collect(),
                defs: [(p, 1u32)].into_iter().collect(),
                may_defs: std::collections::HashSet::new(),
                quoted_uses: std::collections::HashSet::new(),
            };
            let s1 = SsaStatement {
                statement: file_call(&["delete", "$p"]),
                uses: [(p, 1u32)].into_iter().collect(),
                defs: HashMap::new(),
                may_defs: std::collections::HashSet::new(),
                quoted_uses: std::collections::HashSet::new(),
            };
            vec![s0, s1]
        });
        let d = w.iter().find(|w| w.code == DiagCode::W313).expect("W313");
        assert!(
            d.message.contains("file delete with normalised path ($p)"),
            "{}",
            d.message
        );
        // The `[string match …]` example must interpolate the variable
        // name (`${p}`), not render the literal placeholder `${name}`.
        assert!(
            d.message.contains("[string match \"$base/*\" ${p}]"),
            "{}",
            d.message
        );
        assert!(!d.message.contains("${name}"), "{}", d.message);
    }

    #[test]
    fn w313_silent_for_literal_path() {
        use crate::ssa::SsaStatement;
        // `file delete /tmp/foo` has no variable path argument.
        assert!(
            w313_warnings(|_ssa| {
                vec![SsaStatement {
                    statement: file_call(&["delete", "/tmp/foo"]),
                    uses: HashMap::new(),
                    defs: HashMap::new(),
                    may_defs: std::collections::HashSet::new(),
                    quoted_uses: std::collections::HashSet::new(),
                }]
            })
            .is_empty()
        );
    }

    #[test]
    fn w313_helpers_parse_vars_and_guards() {
        assert!(arg_var_names("$p", tcl_dialect::BracedVarStyle::default()).contains("p"));
        assert!(
            arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()).contains("dir")
        );
        assert!(
            arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()).contains("file")
        );
        assert_eq!(extract_var_name("$p").as_deref(), Some("p"));
        assert_eq!(extract_var_name("${p}").as_deref(), Some("p"));
        assert_eq!(
            guard_var_from_string_command("[string match \"$base/*\" $p]").as_deref(),
            Some("p")
        );
        assert_eq!(
            guard_var_from_string_command("[string equal -length 4 $a $p]").as_deref(),
            Some("a")
        );
    }

    #[test]
    fn classify_network_interp_sinks_maps_socket_and_interp() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            classify_network_interp_sinks(&reg, "socket", &["host".into(), "80".into()]),
            vec![(DiagCode::T104, "socket".to_owned())]
        );
        assert_eq!(
            classify_network_interp_sinks(&reg, "interp", &["eval".into(), "$c".into()]),
            vec![(DiagCode::T105, "interp eval".to_owned())]
        );
        // A non-eval interp subcommand and a plain command map to nothing.
        assert!(classify_network_interp_sinks(&reg, "interp", &["share".into()]).is_empty());
        assert!(classify_network_interp_sinks(&reg, "puts", &["hi".into()]).is_empty());
    }

    #[test]
    fn const_assignment_is_not_tainted() {
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let registry = CommandRegistry::build_default();

        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "hello".into(),
            value_span: None,
        };
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(stmt.clone());
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let x = ssa.intern_var("x");
        let ssa_stmt = SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: [(x, 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        assert!(
            taints.get(&(x, 1)).is_none_or(|t| !t.is_tainted()),
            "constant assignment should not be tainted"
        );
    }

    /// T102: tainted variable passed to `regexp` without `--` terminator.
    #[test]
    fn t102_emitted_for_tainted_regexp_pattern() {
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let registry = CommandRegistry::build_default();

        // set pattern [gets stdin] ; regexp $pattern $haystack
        let assign = Statement::AssignValue {
            span: Span::new(0, 25),
            name: "pattern".into(),
            name_braced: false,
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let regexp_call = Statement::Call {
            span: Span::new(26, 50),
            command: "regexp".into(),
            canonical_command: None,
            args: vec!["$pattern".into(), "haystack_value".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };

        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(assign.clone());
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(regexp_call.clone());
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let pattern = ssa.intern_var("pattern");
        let ssa_assign = SsaStatement {
            statement: assign,
            uses: HashMap::new(),
            defs: [(pattern, 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let ssa_regexp = SsaStatement {
            statement: regexp_call,
            uses: [(pattern, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };

        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_assign, ssa_regexp],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
            &HashSet::new(),
        );

        assert!(
            warnings
                .iter()
                .any(|w| w.code == DiagCode::T102 && w.variable == "pattern"),
            "expected T102 for tainted $pattern passed to regexp, got {warnings:?}"
        );
    }

    /// T102 is suppressed when a `--` terminator precedes the tainted argument.
    #[test]
    fn t102_suppressed_with_terminator() {
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let registry = CommandRegistry::build_default();

        let assign = Statement::AssignValue {
            span: Span::new(0, 25),
            name: "pattern".into(),
            name_braced: false,
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        // regexp -- $pattern $haystack  (safe: -- terminates option parsing)
        let regexp_call = Statement::Call {
            span: Span::new(26, 55),
            command: "regexp".into(),
            canonical_command: None,
            args: vec!["--".into(), "$pattern".into(), "haystack_value".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };

        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(assign.clone());
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(regexp_call.clone());
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let pattern = ssa.intern_var("pattern");
        let ssa_assign = SsaStatement {
            statement: assign,
            uses: HashMap::new(),
            defs: [(pattern, 1u32)].into_iter().collect(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };
        let ssa_regexp = SsaStatement {
            statement: regexp_call,
            uses: [(pattern, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        };

        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_assign, ssa_regexp],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&[entry]);
        let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
            &HashSet::new(),
        );

        let t102: Vec<_> = warnings
            .iter()
            .filter(|w| w.code == DiagCode::T102)
            .collect();
        assert!(
            t102.is_empty(),
            "expected no T102 when '--' terminator present, got {t102:?}"
        );
    }

    /// Taint propagates through an interpolated-string word that embeds a
    /// tainted variable: `set out "prefix_${x}_suffix"` where `$x` is tainted.
    #[test]
    fn interpolated_string_word_propagates_taint() {
        use crate::compilation_unit::CompilationUnit;
        use tcl_registry::CommandRegistry;

        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "set x [gets stdin]\nset out \"prefix_${x}_suffix\"",
            &registry,
            false,
        );
        let fu = cu.function("::top").unwrap();
        let out_tainted = fu
            .taints
            .iter()
            .any(|((name, _ver), t)| fu.ssa.var_name(*name) == "out" && t.is_tainted());
        assert!(
            out_tainted,
            "expected 'out' to be tainted via interpolated string embedding tainted $x"
        );
    }

    /// iRules-dialect: `HTTP::uri` is a taint source when dialect is
    /// enabled, and clean when it is not.
    #[test]
    fn irules_http_uri_is_a_dialect_agnostic_source() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();

        // `TAINT_HINTS` is an import-time global, so `HTTP::uri`
        // is a taint source in *every* dialect — including a `tcl8.6`
        // document whose registry never loaded the iRules commands. (The
        // analyser taints `u` here; only the separate W002
        // "disabled command" check is dialect-gated.) The getter form
        // carries the path-prefixed, option-injection-safe colours.
        for dialect in [None, Some(tcl_dialect::DialectProfile::irules())] {
            let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry, false)
                .with_interprocedural(&registry, dialect);
            let fu = cu.function("::top").unwrap();
            let u = fu
                .taints
                .iter()
                .find(|((n, _), _)| fu.ssa.var_name(*n) == "u")
                .map(|(_, t)| *t);
            assert!(
                u.is_some_and(TaintLattice::is_tainted),
                "HTTP::uri should be a taint source (dialect={dialect:?})",
            );
            assert!(
                u.is_some_and(|t| t.colours.contains(TaintColour::PATH_PREFIXED)),
                "HTTP::uri getter should carry PATH_PREFIXED (dialect={dialect:?})",
            );
        }
    }

    /// Inter-procedural: `proc id {x} { return $x }` + tainted actual
    /// should taint the call's result because the passthrough parameter
    /// forwards the tainted value through the proc.
    #[test]
    fn interprocedural_passthrough_transfers_taint() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc ::id {x} { return $x }\n\
             set tainted [gets stdin]\n\
             set out [::id $tainted]",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let fu = cu.function("::top").unwrap();
        assert!(
            fu.taints
                .iter()
                .any(|((n, _), t)| fu.ssa.var_name(*n) == "out" && t.is_tainted()),
            "expected 'out' to be tainted via passthrough proc: {:?}",
            fu.taints,
        );
    }

    /// Global-write seeding must be scoped to procs actually reachable
    /// from the current function. An unrelated helper that writes to
    /// `::state` should not taint `::other_global` in a function that
    /// never calls it.
    #[test]
    fn global_write_seeding_is_scoped_to_reachable_callees() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();
        // `::writer` writes ::state. `::top` never calls ::writer, so
        // reads of other globals here must stay clean.
        let cu = CompilationUnit::build_for(
            "proc ::writer {} { set ::state 1 }\n\
             set local $::safe",
            &registry,
            false,
        )
        .with_interprocedural(&registry, None);
        let fu = cu.function("::top").unwrap();
        let local_tainted = fu
            .taints
            .iter()
            .any(|((n, _), t)| fu.ssa.var_name(*n) == "local" && t.is_tainted());
        assert!(
            !local_tainted,
            "`local` must stay clean — `::writer` is unreachable from top-level: {:?}",
            fu.taints,
        );
    }

    /// Rendered-property colouring: a value starting with `/` should
    /// pick up `PATH_PREFIXED` (implying `NON_DASH_PREFIXED`).
    #[test]
    fn rendered_props_colour_slash_value_path_prefixed() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("set path /etc/hosts", &registry, false);
        let fu = cu.function("::top").unwrap();
        let entry = fu
            .taints
            .iter()
            .find(|((n, _), _)| fu.ssa.var_name(*n) == "path")
            .expect("path taint entry");
        assert!(
            entry.1.colours.contains(TaintColour::PATH_PREFIXED),
            "expected PATH_PREFIXED colour on /-prefixed literal",
        );
    }

    // IRULE3001–3004 sink classifier + end-to-end detection

    fn irules_warnings_for(source: &str) -> Vec<TaintWarning> {
        use crate::compilation_unit::CompilationUnit;
        // Mirror production (tcl-lsp-db): build_default + load the iRules
        // dialect, so the iRules command *specs* (with their subcommands and
        // taint_output_sink_subcommands) are present for the registry-driven,
        // prefix-aware sink classification.
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(DialectSet::IRULES);
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, Some(tcl_dialect::DialectProfile::irules()));
        let mut out: Vec<TaintWarning> = Vec::new();
        for fu in cu.analysable_functions() {
            out.extend(find_taint_warnings(
                &fu.cfg,
                &fu.ssa,
                &fu.taints,
                &fu.sccp.executable_blocks,
                &registry,
                Some(tcl_dialect::DialectProfile::irules()),
                &HashSet::new(),
            ));
        }
        out
    }

    #[test]
    fn redirect_safe_mask_includes_path_prefixed() {
        assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_PREFIXED));
        assert!(TaintColour::REDIRECT_SAFE.contains(TaintColour::PATH_NORMALISED));
        // Must not spill into unrelated colours.
        assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::TAINTED));
        assert!(!TaintColour::REDIRECT_SAFE.contains(TaintColour::CRLF_FREE));
    }

    /// Issue #1410 — the compiler and registry colour domains are a bijection.
    /// Keep both directions pinned so a future colour cannot be silently
    /// dropped by a permissive bitflags conversion.
    #[test]
    fn registry_taint_colour_bridge_preserves_every_bit_in_both_directions() {
        let registry_all = tcl_registry::TaintColour::all();
        let compiler_all = TaintColour::all();
        assert_eq!(reg_colour(registry_all), compiler_all);
        assert_eq!(compiler_colour(compiler_all), registry_all);
        for colour in [
            tcl_registry::TaintColour::TAINTED,
            tcl_registry::TaintColour::PATH_PREFIXED,
            tcl_registry::TaintColour::NON_DASH_PREFIXED,
            tcl_registry::TaintColour::CRLF_FREE,
            tcl_registry::TaintColour::SHELL_ATOM,
            tcl_registry::TaintColour::LIST_CANONICAL,
            tcl_registry::TaintColour::REGEX_LITERAL,
            tcl_registry::TaintColour::PATH_NORMALISED,
            tcl_registry::TaintColour::PATH_BOUNDED,
            tcl_registry::TaintColour::HEADER_TOKEN_SAFE,
            tcl_registry::TaintColour::HTML_ESCAPED,
            tcl_registry::TaintColour::URL_ENCODED,
            tcl_registry::TaintColour::IP_ADDRESS,
            tcl_registry::TaintColour::PORT,
            tcl_registry::TaintColour::FQDN,
            tcl_registry::TaintColour::PATH_JOINED,
            tcl_registry::TaintColour::CHANNEL,
        ] {
            let compiler = reg_colour(colour);
            assert_eq!(compiler_colour(compiler), colour, "round-trip {colour:?}");
        }
    }

    #[test]
    fn file_join_transform_reaches_the_live_cfg_taint_lattice() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();
        let colour_for = |source: &str, name: &str| {
            let cu = CompilationUnit::build_for(source, &registry, false);
            let fu = cu.function("::top").expect("top-level CFG built");
            fu.taints
                .iter()
                .find(|((symbol, _), _)| fu.ssa.var_name(*symbol) == name)
                .map(|(_, taint)| *taint)
                .expect("assignment has a live SSA taint value")
        };

        let joined = colour_for(
            "set raw [gets stdin]\nset joined [file join /base $raw]",
            "joined",
        );
        assert!(joined.is_tainted(), "the source taint must reach file join");
        assert!(
            joined.colours.contains(TaintColour::PATH_JOINED),
            "file join's registry transform must reach the CFG taint lattice: {joined:?}"
        );

        // Mutation control: replacing the transform-bearing `file join` with
        // a same-shape non-transform command removes the only PATH_JOINED
        // producer. If `word_taint_at` stopped applying registry transforms,
        // this and the live vector above would become indistinguishable.
        let without_transform = colour_for(
            "set raw [gets stdin]\nset joined [file tail $raw]",
            "joined",
        );
        assert!(without_transform.is_tainted());
        assert!(
            !without_transform.colours.contains(TaintColour::PATH_JOINED),
            "the mutation control must not inherit file join's transform: {without_transform:?}"
        );
    }

    #[test]
    fn classify_irules_sink_http_respond() {
        let reg = tcl_registry::registry_for_dialect("f5-irules");
        let hit = classify_sink(
            reg,
            "HTTP::respond",
            &[],
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3001));
    }

    #[test]
    fn classify_irules_sink_http_header_insert() {
        let reg = tcl_registry::registry_for_dialect("f5-irules");
        let hit = classify_sink(
            reg,
            "HTTP::header",
            &["insert".to_owned(), "X-Foo".to_owned(), "bar".to_owned()],
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
        assert_eq!(hit.as_ref().unwrap().1, "HTTP::header insert");
    }

    #[test]
    fn classify_irules_sink_http_cookie_replace() {
        let reg = tcl_registry::registry_for_dialect("f5-irules");
        let hit = classify_sink(
            reg,
            "HTTP::cookie",
            &["replace".to_owned(), "sid".to_owned(), "val".to_owned()],
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
        assert_eq!(hit.as_ref().unwrap().1, "HTTP::cookie replace");
    }

    #[test]
    fn classify_irules_sink_prefix_abbreviation() {
        // `HTTP::cookie ins` is a legal abbreviation of `insert`
        // and must still classify as the IRULE3002 output sink.
        let reg = tcl_registry::registry_for_dialect("f5-irules");
        let hit = classify_sink(
            reg,
            "HTTP::cookie",
            &["ins".to_owned(), "sid".to_owned(), "val".to_owned()],
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
        // The label reports the canonical subcommand name.
        assert_eq!(hit.as_ref().unwrap().1, "HTTP::cookie insert");
    }

    #[test]
    fn classify_irules_sink_http_header_remove_is_none() {
        let reg = tcl_registry::registry_for_dialect("f5-irules");
        let hit = classify_sink(
            reg,
            "HTTP::header",
            &["remove".to_owned(), "X-Foo".to_owned()],
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert!(hit.is_none(), "remove subcommand must not emit IRULE3002");
    }

    #[test]
    fn classify_irules_sink_log_and_redirect() {
        let reg = tcl_registry::registry_for_dialect("f5-irules");
        assert_eq!(
            classify_sink(
                reg,
                "log",
                &["local0.info".to_owned(), "x".to_owned()],
                Some(tcl_dialect::DialectProfile::irules())
            )
            .as_ref()
            .map(|(c, _)| *c),
            Some(DiagCode::Irule3003),
        );
        assert_eq!(
            classify_sink(
                reg,
                "HTTP::redirect",
                &["https://evil".to_owned()],
                Some(tcl_dialect::DialectProfile::irules())
            )
            .as_ref()
            .map(|(c, _)| *c),
            Some(DiagCode::Irule3004),
        );
    }

    #[test]
    fn classify_sink_skips_irules_without_dialect() {
        let registry = CommandRegistry::build_default();
        let hit = classify_sink(&registry, "HTTP::respond", &["body".to_owned()], None);
        assert!(hit.is_none(), "no dialect → no IRULE3001, got {hit:?}");
    }

    #[test]
    fn classify_sink_skips_irules_without_dialect_even_when_specs_loaded() {
        // Stronger than `classify_sink_skips_irules_without_dialect`: the
        // registry here *does* have the iRules specs loaded (as a shared
        // multi-dialect registry might), so `registry.get("HTTP::respond")`
        // succeeds. The IRULE3001 classification must still be withheld
        // because the active *document* dialect (`None`) isn't iRules —
        // mirrors `find_setter_constraint_warnings`'s explicit
        // `is_irules_dialect` gate for IRULE3101.
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        assert!(registry.get("HTTP::respond").is_some());
        let hit = classify_sink(&registry, "HTTP::respond", &["body".to_owned()], None);
        assert!(
            hit.is_none(),
            "no dialect → no IRULE3001 even with iRules specs loaded, got {hit:?}"
        );
    }

    #[test]
    fn classify_sink_exec_stays_t100_under_irules_dialect() {
        // `exec` is universal spec data (`dialects: None` since the
        // spec); under f5-irules it is *banned* by the
        // profile's §9 disable list, not by any dialect mask. T100
        // classification must not route through profile availability —
        // it's a plain trait fact, independent of the active document
        // dialect — or `exec` would lose its T100 sink status in exactly
        // the dialect where flagging it matters most.
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let hit = classify_sink(
            &registry,
            "exec",
            &["$cmd".to_owned()],
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::T100));
    }

    #[test]
    fn irule3001_fires_on_tainted_respond_body() {
        let w = irules_warnings_for("set u [HTTP::uri]\nHTTP::respond 200 content $u");
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3001),
            "expected IRULE3001, got {w:?}"
        );
    }

    #[test]
    fn irule3001_no_warning_for_literal_body() {
        let w = irules_warnings_for("HTTP::respond 200 content \"hello\"");
        assert!(
            w.iter().all(|x| x.code != DiagCode::Irule3001),
            "expected no IRULE3001 on literal body, got {w:?}"
        );
    }

    #[test]
    fn irule3002_fires_on_tainted_header_value() {
        let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header insert X-Echo $v");
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3002),
            "expected IRULE3002, got {w:?}"
        );
    }

    #[test]
    fn irule3002_fires_in_command_sub_form() {
        // Regression: `set _ [HTTP::header insert X-Foo $v]` — sink is
        // inside a command substitution, so the AssignValue branch must
        // preserve the subcommand args so classify_irules_sink sees
        // "insert" at arg-index 0.
        let w = irules_warnings_for(
            "set v [HTTP::header X-Src]\nset _ [HTTP::header insert X-Echo $v]",
        );
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3002),
            "expected IRULE3002 inside command-sub sink, got {w:?}"
        );
    }

    #[test]
    fn irule3002_skipped_on_remove_subcommand() {
        let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header remove $v");
        assert!(
            w.iter().all(|x| x.code != DiagCode::Irule3002),
            "remove subcommand must not fire IRULE3002, got {w:?}"
        );
    }

    #[test]
    fn irule3003_fires_on_tainted_log() {
        let w = irules_warnings_for("set u [HTTP::uri]\nlog local0.info $u");
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3003),
            "expected IRULE3003, got {w:?}"
        );
    }

    #[test]
    fn irule3004_fires_on_tainted_redirect() {
        let w = irules_warnings_for("set target [HTTP::header Location]\nHTTP::redirect $target");
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3004),
            "expected IRULE3004, got {w:?}"
        );
    }

    #[test]
    fn irule3004_redirect_safe_suppresses_via_lattice() {
        // Direct lattice check: tainted + REDIRECT_SAFE must suppress.
        // (Latent as an end-to-end test until iRules sources are tagged
        // with PATH_PREFIXED from their `taint_hints` — `HTTP::path`
        // carries PATH_PREFIXED on the getter form.)
        let lat = TaintLattice::tainted().with(TaintColour::PATH_PREFIXED);
        assert!(irules_sink_suppressed(DiagCode::Irule3004, lat));
        let lat = TaintLattice::tainted().with(TaintColour::PATH_NORMALISED);
        assert!(irules_sink_suppressed(DiagCode::Irule3004, lat));
        // Plain tainted should not suppress.
        assert!(!irules_sink_suppressed(
            DiagCode::Irule3004,
            TaintLattice::tainted()
        ));
    }

    #[test]
    fn t102_uses_ensemble_subcommand_label_and_t103_for_regexp() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        // `file delete $u` → T102 labelled "file delete" (ensemble
        // subcommand, via resolve_option_terminator). `regexp $u …` →
        // T103 (regex injection) then T102 (regexp also takes `--`).
        let cu = CompilationUnit::build_for(
            "set u [gets stdin]\nfile delete $u\nregexp $u \"abc\"",
            &registry,
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        let t102 = warnings.iter().find(|w| w.code == DiagCode::T102);
        assert!(
            t102.is_some_and(|w| w.sink_command == "file delete"),
            "expected a T102 with ensemble label 'file delete', got {warnings:?}",
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.code == DiagCode::T103 && w.sink_command == "regexp"),
            "expected T103 for tainted regexp pattern, got {warnings:?}",
        );
        // T103 must precede the regexp T102 (the pattern check is
        // emitted before the sink loop).
        let regexp_codes: Vec<&str> = warnings
            .iter()
            .filter(|w| w.sink_command == "regexp")
            .map(|w| w.code.as_str())
            .collect();
        assert_eq!(
            regexp_codes,
            vec!["T103", "T102"],
            "regexp warnings must be ordered T103 then T102",
        );
    }

    #[test]
    fn t102_suppressed_after_double_dash_terminator() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        // `file delete -- $u`: the `--` terminator protects the path
        // position, so no T102 (W313 still fires elsewhere).
        let cu =
            CompilationUnit::build_for("set u [gets stdin]\nfile delete -- $u", &registry, false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T102),
            "no T102 when `--` precedes the tainted path, got {warnings:?}",
        );
    }

    /// FN regression (see `emit_statement_warnings`): T102 must fire for
    /// a `set matched [regexp $pattern $s]` capture exactly as it does for
    /// the bare `regexp $pattern $s` call it wraps — the earlier code only
    /// ran the option-injection check on `Statement::Call`, silently
    /// skipping the (dominant, in real Tcl) `set x [cmd ...]` idiom.
    #[test]
    fn t102_fires_for_assign_value_wrapped_call() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "set pattern [gets stdin]\nset result [regexp $pattern $haystack]",
            &registry,
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.code == DiagCode::T102 && w.variable == "pattern"),
            "expected T102 for tainted $pattern inside `set result [regexp ...]`, got {warnings:?}"
        );
    }

    /// T102's diagnostic span must cover only the offending argument
    /// (`$pattern`), not the whole statement — tight enough that the
    /// editor squiggle sits on the actual problem, matching what W304
    /// already does for the same position.
    #[test]
    fn t102_span_is_tight_around_the_tainted_argument_bare_call() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let src = "set pattern [gets stdin]\nregexp $pattern $haystack";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        let t102 = warnings
            .iter()
            .find(|w| w.code == DiagCode::T102)
            .expect("expected a T102 warning");
        let slice = &src[t102.span.start() as usize..t102.span.end() as usize];
        assert_eq!(
            slice, "$pattern",
            "T102 span should cover exactly the tainted argument, got {slice:?} from {t102:?}"
        );
    }

    /// Same tight-span requirement for the `set x [cmd ...]` shape: the
    /// embedded command's argument has no token of its own on the
    /// `AssignValue` statement, so the span is re-derived from the
    /// bracketed value text — this pins that derivation to the exact
    /// same byte range as the bare-call case above.
    #[test]
    fn t102_span_is_tight_around_the_tainted_argument_assign_value() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let src = "set pattern [gets stdin]\nset result [regexp $pattern $haystack]";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        let t102 = warnings
            .iter()
            .find(|w| w.code == DiagCode::T102)
            .expect("expected a T102 warning");
        let slice = &src[t102.span.start() as usize..t102.span.end() as usize];
        assert_eq!(
            slice, "$pattern",
            "T102 span should cover exactly the tainted argument, got {slice:?} from {t102:?}"
        );
    }

    /// T102 must carry a `--`-insertion fix anchored immediately before
    /// the tainted argument, so applying it turns the flagged call into
    /// the documented safe form (`regexp -- $pattern ...`).
    #[test]
    fn t102_carries_insert_terminator_fix_at_the_argument_start() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let src = "set pattern [gets stdin]\nregexp $pattern $haystack";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        let t102 = warnings
            .iter()
            .find(|w| w.code == DiagCode::T102)
            .expect("expected a T102 warning");
        assert_eq!(t102.fixes.len(), 1, "expected exactly one fix: {t102:?}");
        let fix = &t102.fixes[0];
        assert_eq!(
            fix.span.start(),
            fix.span.end(),
            "fix must be a pure insertion"
        );
        assert_eq!(fix.span.start(), t102.span.start());
        assert_eq!(fix.new_text, "-- ");
        let mut fixed = src.to_owned();
        fixed.insert_str(fix.span.start() as usize, &fix.new_text);
        assert_eq!(
            fixed,
            "set pattern [gets stdin]\nregexp -- $pattern $haystack"
        );
    }

    /// `interp alias {} myregexp {} regexp` makes `myregexp` behave
    /// exactly like `regexp` at runtime (tclsh 9.0.4-verified), including
    /// its option parsing — so a tainted argument reaching `myregexp`
    /// must trip T102 just as it would through the bare `regexp` name.
    /// `emit_statement_warnings`'s `canonical_command_or_source()` dispatch
    /// (shared by the whole sink family, see the `t100_fires_through_*`
    /// tests) resolves the alias for registry classification, while the
    /// message and span still anchor on the alias call the source
    /// actually wrote.
    #[test]
    fn t102_fires_through_an_interp_alias() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let src = "interp alias {} myregexp {} regexp\nset pattern [gets stdin]\nmyregexp $pattern $haystack";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        let t102 = warnings
            .iter()
            .find(|w| w.code == DiagCode::T102)
            .unwrap_or_else(|| {
                panic!("expected a T102 warning through the alias, got {warnings:?}")
            });
        assert_eq!(t102.variable, "pattern");
        assert!(
            t102.message.contains("'regexp'"),
            "message should name the resolved target command, got {:?}",
            t102.message
        );
        // The span/fix still anchor on the alias call's own argument,
        // not some fabricated position for the aliased-to command.
        assert_eq!(t102.fixes.len(), 1);
        let fix = &t102.fixes[0];
        let mut fixed = src.to_owned();
        fixed.insert_str(fix.span.start() as usize, &fix.new_text);
        assert_eq!(
            fixed,
            "interp alias {} myregexp {} regexp\nset pattern [gets stdin]\nmyregexp -- $pattern $haystack"
        );
    }

    /// `rename regexp myregexp` resolves through the same `CommandAliasMap`
    /// mechanism as `interp alias` (`detect_rename`, see the T100 sibling
    /// tests `t100_fires_through_rename_indirection` /
    /// `t100_silent_for_unaliased_lookalike_command`), so T102 must also
    /// fire through a renamed command name, not just an `interp alias`.
    #[test]
    fn t102_fires_through_a_rename() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let src = "rename regexp myregexp\nset pattern [gets stdin]\nmyregexp $pattern $haystack";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            Some(tcl_dialect::DialectProfile::by_name("tcl8.6")),
            &HashSet::new(),
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.code == DiagCode::T102 && w.variable == "pattern"),
            "expected T102 for tainted $pattern through the `myregexp` rename, got {warnings:?}"
        );
    }

    #[test]
    fn irule_sinks_do_not_fire_without_dialect() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        // Without `with_interprocedural(Some(tcl_dialect::DialectProfile::irules()))`, no dialect
        // is active, and HTTP commands aren't sinks.
        let cu = CompilationUnit::build_for(
            "set u [HTTP::uri]\nHTTP::respond 200 content $u",
            &registry,
            false,
        );
        let fu = cu.function("::top").unwrap();
        let warnings = find_taint_warnings(
            &fu.cfg,
            &fu.ssa,
            &fu.taints,
            &fu.sccp.executable_blocks,
            &registry,
            None,
            &HashSet::new(),
        );
        assert!(
            warnings
                .iter()
                .all(|w| !w.code.as_str().starts_with("IRULE")),
            "no IRULE warnings without dialect, got {warnings:?}"
        );
    }

    #[test]
    fn irules_sink_suppressed_html_escaped_only_mitigates_3001() {
        let tainted_html = TaintLattice::tainted().with(TaintColour::HTML_ESCAPED);
        // IRULE3001 (HTTP response body) — HTML_ESCAPED directly mitigates.
        assert!(irules_sink_suppressed(DiagCode::Irule3001, tainted_html));
        // IRULE3002/3003 (header / log) — HTML_ESCAPED does NOT prove
        // CRLF-injection safety (the escape rewrites `<`/`>`/`&` but
        // leaves raw CR/LF untouched). The CRLF-safe mask excludes
        // `HTML_ESCAPED`.
        assert!(!irules_sink_suppressed(DiagCode::Irule3002, tainted_html));
        assert!(!irules_sink_suppressed(DiagCode::Irule3003, tainted_html));
        // IRULE3004 (redirect) — also not mitigated by HTML_ESCAPED.
        assert!(!irules_sink_suppressed(DiagCode::Irule3004, tainted_html));

        // `CRLF_FREE` does suppress IRULE3002/3003 (the one mitigation
        // accepted in the value position).
        let tainted_crlf_free = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
        assert!(irules_sink_suppressed(
            DiagCode::Irule3002,
            tainted_crlf_free
        ));
        assert!(irules_sink_suppressed(
            DiagCode::Irule3003,
            tainted_crlf_free
        ));
    }

    #[test]
    fn irule3002_header_token_safe_name_position_suppresses() {
        let registry = tcl_registry::registry_for_dialect("f5-irules");
        let args = vec!["insert".to_owned(), "$name".to_owned(), "$value".to_owned()];
        let lat = TaintLattice::tainted().with(TaintColour::HEADER_TOKEN_SAFE);
        // Var `name` occupies arg-index 1 (name position) → suppressed.
        assert!(irule3002_name_position_safe(
            registry,
            "HTTP::header",
            &args,
            "name",
            lat
        ));
        // Var `value` occupies arg-index 2 (value position) → not suppressed.
        assert!(!irule3002_name_position_safe(
            registry,
            "HTTP::header",
            &args,
            "value",
            lat
        ));
        // Without HEADER_TOKEN_SAFE colour: never suppressed, even at name position.
        let plain = TaintLattice::tainted();
        assert!(!irule3002_name_position_safe(
            registry,
            "HTTP::header",
            &args,
            "name",
            plain
        ));
        // Wrong subcommand: not suppressed.
        let rm_args = vec!["remove".to_owned(), "$name".to_owned()];
        assert!(!irule3002_name_position_safe(
            registry,
            "HTTP::header",
            &rm_args,
            "name",
            lat
        ));
    }

    // IRULE3101 — setter-constraint violations

    /// Default helper: run the setter check under the `f5-irules` dialect
    /// (which is the only dialect that can surface IRULE3101 post-internal-gate).
    fn setter_warnings_for(source: &str) -> Vec<TaintWarning> {
        setter_warnings_for_dialect(source, Some(tcl_dialect::DialectProfile::irules()))
    }

    fn setter_warnings_for_dialect(
        source: &str,
        dialect: Option<&tcl_dialect::DialectProfile>,
    ) -> Vec<TaintWarning> {
        use crate::compilation_unit::CompilationUnit;
        let mut registry = CommandRegistry::build_default();
        // The IRULE3101 setter constraints live on the iRules specs, so the
        // registry must have them loaded for the dialect-gated check to see
        // anything.
        registry.load_irules();
        let mut cu = CompilationUnit::build_for(source, &registry, false);
        if dialect.is_some() {
            cu = cu.with_interprocedural(&registry, dialect);
        }
        let mut out: Vec<TaintWarning> = Vec::new();
        for fu in cu.analysable_functions() {
            out.extend(find_setter_constraint_warnings(
                &registry,
                &fu.cfg,
                &fu.ssa,
                &fu.taints,
                &fu.sccp.executable_blocks,
                dialect,
            ));
        }
        out
    }

    #[test]
    fn irule3101_literal_missing_slash_warns() {
        let w = setter_warnings_for("HTTP::uri foo");
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].code, DiagCode::Irule3101);
        assert!(w[0].message.contains("HTTP::uri value must start"));
    }

    #[test]
    fn irule3101_literal_with_slash_clean() {
        let w = setter_warnings_for("HTTP::uri /foo");
        assert!(w.is_empty(), "literal /foo must be clean, got {w:?}");
    }

    #[test]
    fn irule3101_http_path_literal_variants() {
        let bad = setter_warnings_for("HTTP::path bar");
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].code, DiagCode::Irule3101);
        let good = setter_warnings_for("HTTP::path /bar");
        assert!(good.is_empty());
    }

    #[test]
    fn irule3101_generic_taint_warns() {
        // `HTTP::header X-Foo` is a tainted source; reusing it as a path
        // has generic taint (no PATH_PREFIXED / _NORMALISED / _BOUNDED).
        let w = setter_warnings_for("set v [HTTP::header X-Foo]\nHTTP::uri $v");
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3101),
            "generic taint must fire IRULE3101, got {w:?}"
        );
    }

    #[test]
    fn irule3101_pure_var_ref_always_warns_without_safe_colour() {
        // A plain `$p` setter value (no taint + no provable path colour)
        // cannot be proved `/`-prefixed by the static analyser, so
        // IRULE3101 fires. Latent suppression paths
        // via tainted-with-PATH_PREFIXED / _NORMALISED / _BOUNDED colours
        // will light up once iRules source `taint_hints` reach the
        // lattice.
        let w = setter_warnings_for("set p /safe\nHTTP::uri $p");
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3101),
            "pure var-ref setter value must warn without tainted-safe-colour, got {w:?}"
        );
    }

    #[test]
    fn irule3101_dynamic_command_sub_warns() {
        // RHS is a command sub `[foo]` → hits the dynamic branch, which
        // always warns. The literal-check also bails on `[`.
        let w = setter_warnings_for("HTTP::uri [something]");
        assert!(
            w.iter().any(|x| x.code == DiagCode::Irule3101),
            "command-sub setter value must warn, got {w:?}"
        );
    }

    #[test]
    fn irule3101_internally_gated_on_dialect() {
        // Defence-in-depth: even if a caller invokes
        // `find_setter_constraint_warnings` directly (bypassing
        // `run_all_checks`), IRULE3101 must not fire under a non-iRules
        // dialect against user-defined commands named `HTTP::uri` /
        // `HTTP::path`.
        let under_irules = setter_warnings_for_dialect(
            "HTTP::uri foo",
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert_eq!(under_irules.len(), 1);
        assert_eq!(under_irules[0].code, DiagCode::Irule3101);

        let under_none = setter_warnings_for_dialect("HTTP::uri foo", None);
        assert!(
            under_none.is_empty(),
            "no IRULE3101 under None dialect, got {under_none:?}"
        );

        let under_tcl = setter_warnings_for_dialect(
            "HTTP::uri foo",
            Some(tcl_dialect::DialectProfile::by_name("tcl")),
        );
        assert!(
            under_tcl.is_empty(),
            "no IRULE3101 under tcl dialect, got {under_tcl:?}"
        );
    }

    // registry-driven source / sink / setter-constraint coverage

    /// The Tcl-core source classification flows from the registry's
    /// [`Traits::TAINT_SOURCE`] flag: registry-side query and
    /// end-to-end taint-warning emission must agree on `gets` /
    /// `read` / `exec` / `socket`.
    #[test]
    fn arch3_tcl_core_source_is_registry_driven() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        // Registry-side: TAINT_SOURCE trait is stamped on these.
        for cmd in ["gets", "read", "exec", "socket"] {
            let spec = registry.get(cmd).unwrap_or_else(|| panic!("{cmd} missing"));
            assert!(
                spec.traits.contains(tcl_registry::Traits::TAINT_SOURCE),
                "{cmd} must carry TAINT_SOURCE in the registry",
            );
        }
        // Same fact via the registry-side taint helper.
        assert!(tcl_registry::taint::is_taint_source(
            &registry,
            "gets",
            &["stdin"],
            tcl_dialect::DialectSet::empty(),
        ));

        // End-to-end: `gets` → `eval` raises T100. The fact reaches
        // the diagnostic via the registry, not via a compiler-side
        // command-name table.
        let cu = CompilationUnit::build_for("set x [gets stdin]\neval $x", &registry, false)
            .with_interprocedural(&registry, None);
        let mut warnings: Vec<TaintWarning> = Vec::new();
        for fu in cu.analysable_functions() {
            warnings.extend(find_taint_warnings(
                &fu.cfg,
                &fu.ssa,
                &fu.taints,
                &fu.sccp.executable_blocks,
                &registry,
                None,
                &HashSet::new(),
            ));
        }
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 from gets→eval with registry-driven source, got {warnings:?}",
        );
    }

    /// The iRules sink classification is registry-driven (via
    /// `TAINT_SINK` and `EVALUATES_CODE` traits): tainted data into
    /// `expr` (which carries `TAINT_SINK`) raises T100. The trait
    /// is the single source of truth — flipping it removes the
    /// classification.
    #[test]
    fn arch3_irules_sink_is_registry_driven() {
        let registry = CommandRegistry::build_default();
        let expr_spec = registry.get("expr").expect("expr in registry");
        assert!(
            expr_spec.traits.contains(tcl_registry::Traits::TAINT_SINK),
            "expr must carry TAINT_SINK in the registry",
        );

        // End-to-end: tainted iRules data (`HTTP::uri`) into expr
        // surfaces T100 under the iRules dialect.
        let w = irules_warnings_for("set u [HTTP::uri]\nexpr $u");
        assert!(
            w.iter().any(|d| d.code == DiagCode::T100),
            "expected T100 on tainted expr sink under iRules, got {w:?}",
        );
    }

    /// IRULE3101 setter-constraint violations are gated by the
    /// iRules dialect filter — the diagnostic must not fire under
    /// the plain Tcl dialect because the setter constraint comes
    /// from the iRules-only registry entry.
    #[test]
    fn arch3_setter_constraint_is_dialect_driven() {
        // `HTTP::uri foo` (no leading slash) under iRules produces
        // an IRULE3101 setter-constraint violation.
        let irules = setter_warnings_for_dialect(
            "HTTP::uri foo",
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert!(
            irules.iter().any(|w| w.code == DiagCode::Irule3101),
            "expected IRULE3101 under iRules dialect, got {irules:?}",
        );

        // The same source under plain Tcl: a user-defined proc named
        // `HTTP::uri` is legal and the setter check must stay silent.
        let plain = setter_warnings_for_dialect("HTTP::uri foo", None);
        assert!(
            plain.iter().all(|w| w.code != DiagCode::Irule3101),
            "no IRULE3101 outside iRules, got {plain:?}",
        );
    }

    #[test]
    fn taint_warnings_emitted_in_deterministic_proc_order() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        // `bbb` is defined *before* `aaa` in source. Per-proc warning order
        // must follow qualified name (aaa, then bbb) — not source order and
        // not the (random) HashMap order — so the output is reproducible.
        let source = "proc bbb {p} { set v [exec $p]\n eval $v }\n\
                      proc aaa {p} { set v [exec $p]\n eval $v }\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert_eq!(warnings.len(), 2, "one eval sink per proc: {warnings:?}");
        // `aaa` is defined later in source (larger span) yet sorts first.
        assert!(
            warnings[0].span.start() > warnings[1].span.start(),
            "expected name order (aaa before bbb) regardless of source order: {warnings:?}",
        );
    }

    #[test]
    fn callback_replays_batch_into_one_extra_interprocedural_build() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();
        let source = "entry .one -validatecommand {eval %P}\n\
                      entry .two -validatecommand {eval %S}\n\
                      bind .two <Key> {eval %A}";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        reset_callback_replay_build_count();
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert_eq!(
            callback_replay_build_count(),
            1,
            "every statically recoverable callback must share one replay solve"
        );
        assert_eq!(
            warnings.len(),
            3,
            "each registered sink still warns: {warnings:?}"
        );
    }

    #[test]
    fn callback_replay_batches_past_the_first_batch_without_truncating_later_sinks() {
        use crate::compilation_unit::CompilationUnit;
        use std::fmt::Write as _;

        let mut source = String::new();
        for index in 0..=MAX_CALLBACK_TAINT_REPLAYS {
            writeln!(
                &mut source,
                "entry .benign{index} -validatecommand {{set value %P}}"
            )
            .expect("write to String");
        }
        source.push_str("entry .malicious -validatecommand {eval %P}");
        let malicious_start = u32::try_from(
            source
                .rfind("{eval %P}")
                .expect("malicious callback is present"),
        )
        .expect("test source fits in u32");

        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(&source, &registry, false)
            .with_interprocedural(&registry, None);
        reset_callback_replay_build_count();
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);

        assert_eq!(
            callback_replay_build_count(),
            2,
            "callbacks beyond one replay batch must receive a second bounded solve"
        );
        assert!(
            warnings.iter().any(|warning| {
                warning.code == DiagCode::T100 && warning.span.start() == malicious_start
            }),
            "the callback after 32 benign registrations must still warn: {warnings:?}"
        );
    }

    #[test]
    fn callback_free_units_skip_replay_name_search_and_solve() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();
        for source in [
            "set value fixed\nputs $value",
            // This is a callback registration, but its builder is dynamic and
            // therefore deliberately has no statically recoverable replay.
            "entry .entry -validatecommand [format {eval %P}]",
        ] {
            let cu = CompilationUnit::build_for(source, &registry, false)
                .with_interprocedural(&registry, None);
            reset_callback_replay_build_count();
            reset_callback_replay_name_search_count();
            let _warnings = find_taint_warnings_for_cu(&cu, &registry, None);
            assert_eq!(
                callback_replay_build_count(),
                0,
                "no recoverable callback may compile an empty replay: {source}"
            );
            assert_eq!(
                callback_replay_name_search_count(),
                0,
                "no recoverable callback may scan source for replay names: {source}"
            );
        }
    }

    #[test]
    fn callback_replay_name_search_is_single_pass_under_collision_stress() {
        use std::fmt::Write as _;

        let mut source = String::new();
        for suffix in 0..(MAX_CALLBACK_TAINT_REPLAYS + 8) {
            let input = if suffix == 0 {
                CALLBACK_REPLAY_INPUT_VAR_BASE.to_owned()
            } else {
                format!("{CALLBACK_REPLAY_INPUT_VAR_BASE}_{suffix}")
            };
            let proc = if suffix == 0 {
                CALLBACK_REPLAY_PROC_BASE.to_owned()
            } else {
                format!("{CALLBACK_REPLAY_PROC_BASE}_{suffix}")
            };
            writeln!(
                &mut source,
                "set {input} safe\nproc ::{proc}_0 {{}} {{ return }}"
            )
            .expect("write to String");
        }
        source.push_str("entry .entry -validatecommand {eval %P}");

        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(&source, &registry, false)
            .with_interprocedural(&registry, None);
        reset_callback_replay_build_count();
        reset_callback_replay_name_search_count();
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.code == DiagCode::T100),
            "collision avoidance must preserve the callback warning: {warnings:?}"
        );
        assert_eq!(callback_replay_build_count(), 1);
        assert_eq!(
            callback_replay_name_search_count(),
            1,
            "even a collision-rich source gets one linear synthetic-name scan"
        );
    }

    /// TP: `interp alias {} myEval {} eval; myEval $x` still raises T100 —
    /// `classify_sink` dispatches on `Statement::canonical_command_or_source()`,
    /// so the alias resolves to `eval` even though the source spells the call
    /// `myEval`, which is not itself a registered command.
    #[test]
    fn t100_fires_through_interp_alias_indirection() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "interp alias {} myEval {} eval\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 through the `myEval` alias for `eval`, got {warnings:?}",
        );
    }

    /// TP: `rename eval myEval; myEval $x` also raises T100 — the same
    /// `CommandAliasMap` mechanism `interp alias` uses now also recognises
    /// a static `rename oldName newName` (`detect_rename`), so a renamed
    /// built-in resolves through `canonical_command_or_source()` exactly
    /// like an `interp alias`.
    #[test]
    fn t100_fires_through_rename_indirection() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "rename eval myEval\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 through the `myEval` rename of `eval`, got {warnings:?}",
        );
    }

    /// TN: without the alias, a plain user command named `myEval` is not a
    /// sink at all — proves the alias fix above isn't matching on the bare
    /// name.
    #[test]
    fn t100_silent_for_unaliased_lookalike_command() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "proc myEval {x} { return $x }\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "a user proc named myEval must not be treated as the eval sink, got {warnings:?}",
        );
    }

    /// TP: `puts $tainted` still raises T101 (not T100) end-to-end, now via
    /// the registry-driven `taint_output_sink` dispatch in `classify_sink`
    /// rather than a hardcoded `command == "puts"` match.
    #[test]
    fn t101_fires_for_puts_via_registry_driven_dispatch() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set x [gets stdin]\nputs $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T101),
            "expected T101 for tainted puts, got {warnings:?}",
        );
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "puts must not also raise T100, got {warnings:?}",
        );
    }

    /// TP: `open $tainted` raises T100. A `fileName` argument whose first
    /// character is `|` runs the rest of the word as a command pipeline —
    /// the same command-injection hazard `exec` carries — so tainted data
    /// reaching `open`'s first argument is a code-execution sink exactly
    /// like `exec`, not merely a file-I/O one.
    #[test]
    fn t100_fires_for_open_tainted_filename() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        assert!(
            registry
                .get("open")
                .expect("open in registry")
                .traits
                .contains(tcl_registry::Traits::TAINT_SINK),
            "open must carry TAINT_SINK in the registry",
        );
        let source = "set x [gets stdin]\nopen $x r\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for tainted open fileName, got {warnings:?}",
        );
    }

    /// FP fix / TN: `open /tmp/out $mode` — a *literal* safe fileName with a
    /// tainted access-mode argument must not raise T100. Only arg 0 selects a
    /// `|`-pipeline; a tainted access mode or octal permissions can never
    /// turn a literal path into a command, so `open`'s `taint_code_sink_args
    /// = Some(&[0])` gates the sink to the fileName slot alone.
    #[test]
    fn t100_silent_for_open_tainted_access_mode() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set mode [gets stdin]\nopen /tmp/out $mode\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "a tainted open access mode with a literal fileName must not raise T100, got {warnings:?}",
        );
    }

    /// TP: `apply $lambda ...` with a tainted lambda (arg 0) raises T100 — the
    /// lambda body is executed verbatim, so tainted code in the `func` slot
    /// is a genuine code-injection sink.
    #[test]
    fn t100_fires_for_apply_tainted_lambda() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set lambda [gets stdin]\napply $lambda 1 2\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for a tainted apply lambda (arg 0), got {warnings:?}",
        );
    }

    /// FP fix / TN: `apply {{x} {return $x}} $tainted` — a tainted value
    /// passed as an ordinary lambda *argument* (bound to a formal parameter,
    /// never re-evaluated as script) must not raise T100. `apply`'s
    /// `taint_code_sink_args = Some(&[0])` gates the sink to the `func` slot.
    #[test]
    fn t100_silent_for_apply_tainted_argument() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set x [gets stdin]\napply {{a} {return $a}} $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "a tainted apply argument (not the lambda) must not raise T100, got {warnings:?}",
        );
    }

    /// FP fix / TN: `subst -nocommands $tainted` — the exact mitigation
    /// `subst`'s own hover snippet recommends — must not raise T100, since
    /// `-nocommands` disables the only hazard (command substitution) the
    /// code names.
    #[test]
    fn t100_silent_for_subst_nocommands() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set x [gets stdin]\nsubst -nocommands $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "subst -nocommands must not raise T100, got {warnings:?}",
        );
    }

    /// TP regression: plain `subst $tainted` (no flags) keeps raising T100 —
    /// the new sink gate must not suppress the common, genuinely dangerous
    /// default-options call.
    #[test]
    fn t100_fires_for_subst_without_nocommands() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set x [gets stdin]\nsubst $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for bare `subst $x`, got {warnings:?}",
        );
    }

    /// TN: the Tcl 9.1 positive option form without `-commands` is equally
    /// safe — `-variables` alone never runs command substitution.
    #[test]
    fn t100_silent_for_subst_positive_form_without_commands() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set x [gets stdin]\nsubst -variables $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "subst -variables (Tcl 9.1 positive form) must not raise T100, got {warnings:?}",
        );
    }

    /// TP: the Tcl 9.1 positive form *with* `-commands` present stays a sink.
    #[test]
    fn t100_fires_for_subst_positive_form_with_commands() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "set x [gets stdin]\nsubst -variables -commands $x\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for `subst -variables -commands $x`, got {warnings:?}",
        );
    }

    /// FN fix — TP: a tainted variable used directly in an `if` branch
    /// condition is evaluated exactly like any other braced `expr`, but
    /// conditions live on `Terminator::Branch`, not in `ssa_block.statements`,
    /// so the per-statement scan alone never reached them. Confirmed
    /// previously absent by direct testing before `emit_branch_condition_warnings`
    /// was added.
    #[test]
    fn t100_fires_for_tainted_if_condition() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source =
            "proc f {} {\n  set x [gets stdin]\n  if {$x + 1 > 5} {\n    puts big\n  }\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        let hit = warnings
            .iter()
            .find(|w| w.code == DiagCode::T100)
            .unwrap_or_else(|| panic!("expected T100 for tainted if-condition, got {warnings:?}"));
        // Tight span: just the `{...}` condition text, not the whole
        // `if {...} { puts big }` statement (which also spans the body).
        let text = &source[hit.span.start() as usize..hit.span.end() as usize];
        assert!(
            text.contains("$x + 1 > 5") && !text.contains("puts"),
            "expected a condition-only span, got {text:?}",
        );
    }

    /// TP: the same hazard through `while` and `for` conditions.
    #[test]
    fn t100_fires_for_tainted_while_and_for_conditions() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        for source in [
            "proc f {} {\n  set x [gets stdin]\n  while {$x < 10} {\n    incr x\n  }\n}\n",
            "proc f {} {\n  set x [gets stdin]\n  for {set i 0} {$i < $x} {incr i} {\n    puts $i\n  }\n}\n",
        ] {
            let cu = CompilationUnit::build_for(source, &registry, false)
                .with_interprocedural(&registry, None);
            let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
            assert!(
                warnings.iter().any(|w| w.code == DiagCode::T100),
                "expected T100 for tainted loop condition in {source:?}, got {warnings:?}",
            );
        }
    }

    /// FP fix / TN: `if {$tainted eq "admin"}` raises nothing — `eq` is a
    /// pure string comparison with no numeric coercion, so neither the
    /// "numeric coercion" message nor any other T100 variant applies. This
    /// also proves the branch-condition pass reuses the same
    /// `collect_coercion_operands` operator-context filter as the
    /// `AssignExpr`/`ExprEval` path, not a re-implementation that flags
    /// every variable indiscriminately.
    #[test]
    fn t100_silent_for_string_only_if_condition() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source =
            "proc f {} {\n  set x [gets stdin]\n  if {$x eq \"admin\"} {\n    puts ok\n  }\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "eq is a pure string compare, expected no T100, got {warnings:?}",
        );
    }

    /// FP fix — TN: a tainted variable that only appears inside a nested
    /// `[cmd $x]` command substitution within a braced expr must not be
    /// flagged as an "expr operand: numeric coercion" hit — it flows into
    /// that nested call's own argument, a completely different risk
    /// category (code execution, if `cmd` is itself dangerous) which that
    /// call's own sink classification already covers independently.
    #[test]
    fn t100_expr_operand_message_not_raised_for_var_only_in_nested_command_sub() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        // `string length` is a sanitiser (fixed numeric return) and not a
        // sink, so the nested call itself raises nothing either — isolating
        // the "expr operand" false positive this test targets.
        let source = "proc f {} {\n  set x [gets stdin]\n  if {[string length $x] > 5} {\n    puts big\n  }\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "$x only reaches a nested command-sub argument, not a direct \
             expr operand; expected no T100, got {warnings:?}",
        );
    }

    /// TP: a tainted variable nested through a nested command sub still
    /// raises T100 when that nested command is *itself* a sink.
    #[test]
    fn t100_fires_when_nested_command_sub_is_itself_a_sink() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "proc f {} {\n  set x [gets stdin]\n  if {[exec $x] eq \"ok\"} {\n    puts ok\n  }\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 from the nested `exec $x`, got {warnings:?}",
        );
    }

    /// TP: `$tainted` inside a ternary branch still flags when the ternary
    /// itself sits in a numeric-coercion position — the branch's value
    /// stands in for the ternary as a whole in the parent operator.
    #[test]
    fn t100_fires_for_tainted_var_through_ternary_branch_in_coercing_context() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "proc f {c} {\n  set x [gets stdin]\n  if {($c ? $x : 0) + 1 > 5} {\n    puts big\n  }\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for $x flowing through a coerced ternary branch, got {warnings:?}",
        );
    }

    /// TP: the ternary *condition* is always boolean-coerced, regardless of
    /// the ternary's own surrounding context.
    #[test]
    fn t100_fires_for_tainted_var_in_ternary_condition() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source =
            "proc f {} {\n  set x [gets stdin]\n  if {$x ? \"a\" : \"b\"} {\n    puts ok\n  }\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T100),
            "expected T100 for tainted ternary condition, got {warnings:?}",
        );
    }

    /// Precision: `eval "prefix" $x "suffix"` underlines only the `$x`
    /// argument word (2 characters), not the whole `eval "prefix" $x
    /// "suffix"` statement — `tight_arg_span` narrows the highlight to the
    /// one argument word that actually carries the tainted value, using the
    /// real per-word source spans a compiled `Statement::Call` carries.
    #[test]
    fn t100_span_is_the_tainted_argument_word_not_the_whole_statement() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let source = "proc f {} {\n  set x [gets stdin]\n  eval \"prefix\" $x \"suffix\"\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        let hit = warnings
            .iter()
            .find(|w| w.code == DiagCode::T100)
            .unwrap_or_else(|| panic!("expected T100, got {warnings:?}"));
        let text = &source[hit.span.start() as usize..hit.span.end() as usize];
        assert_eq!(
            text, "$x",
            "expected the tight `$x` argument span, got {text:?} from {hit:?}",
        );
    }

    /// Issue #1604 — the `${…}` closer comes from the shared owner, so the
    /// name taint is keyed on is the one the lexer spanned.
    ///
    /// Taint is name-keyed: a scan that recovers `a{b` where the document's
    /// lexer spanned `a{b}c` looks up a cell nothing ever writes, so the
    /// tainted value's colour is lost and the sink diagnostics that depend on
    /// it (T1xx, W313) go quiet on a flow they should flag. Oracle:
    /// `set {a{b}c} 7; subst {${a{b}c}}` is `7` on tclsh 9.0.4 and
    /// `can't read "a{b"` on 8.6.16.
    #[test]
    fn arg_var_names_follows_the_release_close_rule() {
        use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};
        assert_eq!(
            arg_var_names("/tmp/${a{b}c}", Tcl9Nesting),
            HashSet::from(["a{b}c".to_owned()])
        );
        assert_eq!(
            arg_var_names("/tmp/${a{b}c}", FirstClose),
            HashSet::from(["a{b".to_owned()])
        );
        // The scan resumes past the reference under either rule, so a second
        // ordinary reference in the same word is still collected.
        assert_eq!(
            arg_var_names("${a{b}c}/$tail", Tcl9Nesting),
            HashSet::from(["a{b}c".to_owned(), "tail".to_owned()])
        );
    }

    /// `arg_var_names` vs. `crate::var_refs::vars_in_word` on the tricky
    /// shapes the centralisation question
    /// turns on. The two are **not** interchangeable — each is right where
    /// the other is wrong — so `arg_var_names` stays a bespoke scanner
    /// rather than a call to the shared lexer-based helper. Locks in the
    /// current (now bug-fixed) behaviour of both so a future refactor
    /// attempt trips these before merging.
    mod arg_var_names_vs_vars_in_word {
        use super::*;

        /// TP: a bare `$name` reference — both scanners agree.
        #[test]
        fn agree_on_bare_var() {
            let registry = CommandRegistry::build_default();
            assert_eq!(
                arg_var_names("$p", tcl_dialect::BracedVarStyle::default()),
                HashSet::from(["p".to_owned()])
            );
            assert_eq!(
                crate::var_refs::vars_in_word("$p", &registry)
                    .into_iter()
                    .collect::<HashSet<_>>(),
                HashSet::from(["p".to_owned()])
            );
        }

        /// TP: `${braced}` next to a bare `$var` in the same word — both
        /// scanners agree, including across a non-variable separator (`/`).
        #[test]
        fn agree_on_braced_and_bare_in_one_word() {
            let registry = CommandRegistry::build_default();
            let want = HashSet::from(["dir".to_owned(), "file".to_owned()]);
            assert_eq!(
                arg_var_names("${dir}/$file", tcl_dialect::BracedVarStyle::default()),
                want
            );
            assert_eq!(
                crate::var_refs::vars_in_word("${dir}/$file", &registry)
                    .into_iter()
                    .collect::<HashSet<_>>(),
                want
            );
        }

        /// TP (command substitution): a `$var` nested inside `[cmd …]` —
        /// both scanners recurse into the substitution and agree.
        #[test]
        fn agree_on_var_inside_command_substitution() {
            let registry = CommandRegistry::build_default();
            assert_eq!(
                arg_var_names("[foo $x]", tcl_dialect::BracedVarStyle::default()),
                HashSet::from(["x".to_owned()])
            );
            assert_eq!(
                crate::var_refs::vars_in_word("[foo $x]", &registry)
                    .into_iter()
                    .collect::<HashSet<_>>(),
                HashSet::from(["x".to_owned()])
            );
        }

        /// DISAGREEMENT (documented, not a bug to fix here): a variable
        /// reference *nested inside another variable's array index*
        /// (`$arr($cmd)`, the `cmdAH-1.4`/`1.5`-style idiom the segmenter's
        /// `word_piece` doc also calls out) is a single opaque `Var` token
        /// to the real lexer — `vars_in_word` only reads the outer array
        /// name (`arr`) and never recurses into the index the way it
        /// recurses into `[…]` substitutions. `arg_var_names`'s naive
        /// byte-scan is blind to token structure, so it happens to find
        /// *both* names. Replacing `arg_var_names` with `vars_in_word`
        /// would silently drop `cmd` from a W313 warning's variable set —
        /// a false-negative regression — so this asymmetry is exactly why
        /// the hand-rolled scanner is being kept, not centralised.
        #[test]
        fn disagree_on_var_nested_in_array_index() {
            let registry = CommandRegistry::build_default();
            assert_eq!(
                arg_var_names("$arr($cmd)", tcl_dialect::BracedVarStyle::default()),
                HashSet::from(["arr".to_owned(), "cmd".to_owned()]),
                "arg_var_names must keep catching the nested index variable"
            );
            assert_eq!(
                crate::var_refs::vars_in_word("$arr($cmd)", &registry)
                    .into_iter()
                    .collect::<HashSet<_>>(),
                HashSet::from(["arr".to_owned()]),
                "vars_in_word does not recurse into the array index — documented gap"
            );
        }

        /// FP fixed: a backslash-escaped `\$name` is literal text (Tcl
        /// backslash substitution turns `\$` into a literal `$`, so no
        /// variable is referenced) — `arg_var_names` used to misparse this
        /// as a reference to `notavar` because its byte-scan didn't know
        /// about backslash escaping. Now agrees with `vars_in_word`, which
        /// already got this right via the real lexer.
        #[test]
        fn escaped_dollar_is_not_a_reference() {
            let registry = CommandRegistry::build_default();
            assert!(
                arg_var_names("\\$notavar", tcl_dialect::BracedVarStyle::default()).is_empty(),
                "an escaped \\$ must not be read as a variable reference"
            );
            assert!(crate::var_refs::vars_in_word("\\$notavar", &registry).is_empty());
        }

        /// FP fixed, doubled escape: `\\$foo` is an escaped backslash
        /// (literal `\`) followed by a live, unescaped `$foo` — the
        /// variable reference still must be found.
        #[test]
        fn double_backslash_then_live_var_is_still_found() {
            assert_eq!(
                arg_var_names("\\\\$foo", tcl_dialect::BracedVarStyle::default()),
                HashSet::from(["foo".to_owned()])
            );
        }

        /// TN: plain literal text with no `$` sigil at all.
        #[test]
        fn no_dollar_sigil_is_empty() {
            assert!(
                arg_var_names("hello world", tcl_dialect::BracedVarStyle::default()).is_empty()
            );
        }
    }

    /// `interp alias` / `rename` command-name resolution feeding sink
    /// classification. Sink checks key off
    /// `Statement::canonical_command_or_source()`, so a call through a
    /// renamed or aliased name resolves to whatever it now denotes.
    mod rename_and_alias_taint {
        use super::*;
        use crate::compilation_unit::CompilationUnit;

        fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
            let registry = CommandRegistry::build_default();
            let cu = CompilationUnit::build_for(source, &registry, false)
                .with_interprocedural(&registry, None);
            find_taint_warnings_for_cu(&cu, &registry, None)
        }

        /// TP (regression guard): tainted data reaching the real `puts` sink
        /// through an `interp alias` name. This was silently missed before
        /// sink classification started reading `canonical_command` — the
        /// alias table was populated but never consulted here.
        #[test]
        fn tainted_data_through_interp_alias_is_flagged() {
            let w =
                taint_warnings_for("set u [gets stdin]\ninterp alias {} myputs {} puts\nmyputs $u");
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "expected T101 through the `myputs` alias, got {w:?}"
            );
        }

        /// TP: tainted data reaching the real `puts` sink through a renamed
        /// command name (`rename puts myputs`) — the scenario this
        /// investigation was specifically scoped to cover.
        #[test]
        fn tainted_data_through_renamed_sink_is_flagged() {
            let w = taint_warnings_for("set u [gets stdin]\nrename puts myputs\nmyputs $u");
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "expected T101 through the renamed `myputs`, got {w:?}"
            );
        }

        /// TN: `rename someSafeProc puts` shadows the builtin — the bare
        /// name `puts` now denotes the user's own proc, not the real
        /// output sink, so a subsequent `puts $tainted` must NOT be flagged.
        /// Guards the false-positive direction of the same fix (a renamed
        /// binding can point *away* from a builtin just as easily as onto
        /// one).
        #[test]
        fn rename_shadowing_a_builtin_suppresses_the_sink() {
            let w = taint_warnings_for(
                "proc mysafeputs {x} { return }\n\
                 set u [gets stdin]\n\
                 rename mysafeputs puts\n\
                 puts $u",
            );
            assert!(
                w.iter().all(|d| d.code != DiagCode::T101),
                "puts is shadowed by a user proc after the rename, got {w:?}"
            );
        }

        /// TN: a *dynamic* rename (`rename $old puts`) must not be
        /// approximated — the static `new` name (`puts`) keeps its normal
        /// registry classification rather than silently losing sink
        /// coverage to an unresolvable placeholder target.
        #[test]
        fn dynamic_rename_does_not_suppress_the_builtin_sink() {
            let w = taint_warnings_for(
                "set old somecmd\nset u [gets stdin]\nrename $old puts\nputs $u",
            );
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "a dynamic rename must not suppress the real puts sink, got {w:?}"
            );
        }

        /// TN control: an *unrelated* rename must not disturb ordinary
        /// `puts` sink detection.
        #[test]
        fn unrelated_rename_does_not_affect_puts_sink() {
            let w = taint_warnings_for("rename string mystr\nset u [gets stdin]\nputs $u");
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "expected T101 for a plain puts call, got {w:?}"
            );
        }

        /// TN: the deletion form `rename puts {}` creates no new binding —
        /// nothing to alias — so it must not be mistaken for a rename that
        /// resolves some other name to `puts`.
        #[test]
        fn rename_deletion_form_creates_no_alias() {
            let w = taint_warnings_for("set u [gets stdin]\nrename puts {}\nputs $u");
            // `puts` itself was deleted, not aliased — this codebase does not
            // model command deletion, so the bare name keeps resolving to its
            // registry default (still flagged) rather than silently going
            // opaque. What matters here is that the deletion form did not
            // crash `detect_rename` or install a bogus alias entry.
            assert!(w.iter().any(|d| d.code == DiagCode::T101), "got {w:?}");
        }
    }

    /// Unit tests for the pure [`shadowed_builtin_names`] helper, isolated
    /// from the end-to-end taint-warning plumbing tested by
    /// `namespace_shadowed_builtin_taint` below.
    mod shadowed_builtin_names_tests {
        use super::*;

        #[test]
        fn namespace_scoped_proc_shadows_only_its_own_namespace() {
            let registry = CommandRegistry::build_default();
            let procs = ["::myns::puts", "::myns::helper"];
            let shadowed = shadowed_builtin_names("::myns", procs.iter().copied(), &registry);
            assert_eq!(shadowed, HashSet::from(["puts".to_owned()]));
            // A different namespace sees no shadow at all.
            assert!(shadowed_builtin_names("::other", procs.iter().copied(), &registry).is_empty());
        }

        #[test]
        fn global_proc_shadows_every_namespace() {
            let registry = CommandRegistry::build_default();
            let procs = ["::puts"];
            for ns in ["::", "::myns", "::a::b"] {
                assert_eq!(
                    shadowed_builtin_names(ns, procs.iter().copied(), &registry),
                    HashSet::from(["puts".to_owned()]),
                    "namespace {ns} must see the global shadow"
                );
            }
        }

        #[test]
        fn nested_child_proc_does_not_shadow_its_parent() {
            let registry = CommandRegistry::build_default();
            let procs = ["::a::b::puts"];
            assert!(shadowed_builtin_names("::a", procs.iter().copied(), &registry).is_empty());
        }

        #[test]
        fn non_registry_proc_name_is_not_shadowing() {
            // A proc named after something the registry doesn't know is not a
            // "shadow" in this sense — there's no builtin sink to suppress.
            let registry = CommandRegistry::build_default();
            let procs = ["::myns::totallyMadeUpName"];
            assert!(shadowed_builtin_names("::myns", procs.iter().copied(), &registry).is_empty());
        }
    }

    /// A user proc shadowing a builtin's bare name: `proc ::myns::puts {args}
    /// {…}` (or a plain top-level `proc puts {…}`) makes a bare `puts` call
    /// from the shadowing scope dispatch to the user's own proc, not the
    /// output-sink builtin.
    mod namespace_shadowed_builtin_taint {
        use super::*;
        use crate::compilation_unit::CompilationUnit;

        fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
            let registry = CommandRegistry::build_default();
            let cu = CompilationUnit::build_for(source, &registry, false)
                .with_interprocedural(&registry, None);
            find_taint_warnings_for_cu(&cu, &registry, None)
        }

        /// FP fixed: a namespace-scoped proc of the same name as a builtin
        /// must suppress the builtin sink classification for bare calls made
        /// from that namespace — this was the false positive the
        /// investigation's example (`proc ::myns::puts`) called out.
        #[test]
        fn namespace_scoped_proc_shadowing_builtin_suppresses_sink() {
            let w = taint_warnings_for(
                "namespace eval ::myns {\n\
                 \x20 proc puts {x} { return }\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
            );
            assert!(
                w.iter().all(|d| d.code != DiagCode::T101),
                "puts is shadowed by ::myns::puts, must not be a sink, got {w:?}"
            );
        }

        /// FP fixed: the same shadowing effect for a plain top-level proc
        /// redefinition (no namespace involved at all).
        #[test]
        fn top_level_proc_shadowing_builtin_suppresses_sink() {
            let w = taint_warnings_for("proc puts {x} { return }\nset u [gets stdin]\nputs $u");
            assert!(
                w.iter().all(|d| d.code != DiagCode::T101),
                "puts is shadowed by a top-level proc, got {w:?}"
            );
        }

        /// FP fixed: a *global*-level shadowing proc suppresses the sink for
        /// bare calls made from *any* namespace, not just the global one —
        /// Tcl's unqualified-name lookup always falls back to global.
        #[test]
        fn global_proc_shadows_calls_from_other_namespaces_too() {
            let w = taint_warnings_for(
                "proc puts {x} { return }\n\
                 namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
            );
            assert!(
                w.iter().all(|d| d.code != DiagCode::T101),
                "the global puts proc must shadow calls from ::myns too, got {w:?}"
            );
        }

        /// TN control: an unshadowed bare `puts` inside a namespace is still
        /// the real builtin sink.
        #[test]
        fn unshadowed_namespace_call_is_still_a_sink() {
            let w = taint_warnings_for(
                "namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
            );
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "expected T101 for an unshadowed puts call, got {w:?}"
            );
        }

        /// TN control: a shadowing proc in an unrelated namespace must not
        /// suppress a sink in a *different* namespace.
        #[test]
        fn shadow_in_unrelated_namespace_does_not_suppress_sink() {
            let w = taint_warnings_for(
                "namespace eval ::other {\n\
                 \x20 proc puts {x} { return }\n\
                 }\n\
                 namespace eval ::myns {\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   puts $u\n\
                 \x20 }\n\
                 }",
            );
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "::other's shadow must not affect ::myns, got {w:?}"
            );
        }

        /// TN control: an explicit fully-qualified `::puts` call bypasses
        /// namespace resolution entirely, so it must still be flagged even
        /// when the bare name is locally shadowed.
        #[test]
        fn fully_qualified_call_bypasses_the_shadow() {
            let w = taint_warnings_for(
                "namespace eval ::myns {\n\
                 \x20 proc puts {x} { return }\n\
                 \x20 proc run {} {\n\
                 \x20   set u [gets stdin]\n\
                 \x20   ::puts $u\n\
                 \x20 }\n\
                 }",
            );
            assert!(
                w.iter()
                    .any(|d| matches!(d.code, DiagCode::T100 | DiagCode::T101)),
                "an explicit ::puts must still be classified as a sink, got {w:?}"
            );
        }
    }

    /// `trace add variable` / `trace variable` conservatively forcing a
    /// traced name to fully tainted (`apply_module_variable_traces`).
    mod variable_trace_taint {
        use super::*;
        use crate::compilation_unit::CompilationUnit;

        fn taint_warnings_for(source: &str) -> Vec<TaintWarning> {
            let registry = CommandRegistry::build_default();
            let cu = CompilationUnit::build_for(source, &registry, false)
                .with_interprocedural(&registry, None);
            find_taint_warnings_for_cu(&cu, &registry, None)
        }

        /// FN fixed (the sharper, pre-existing bug): installing a `write`
        /// trace on an already-tainted variable used to silently clear its
        /// taint (the trace-add call's own literal argument words — `add`,
        /// `variable`, `x`, `write`, `{handler}` — were themselves clean, and
        /// the generic "propagate from arguments" fallback treated the
        /// trace-add's synthetic write to `x` as deriving from them). The
        /// pre-existing taint must survive the trace installation.
        #[test]
        fn write_trace_does_not_clear_prior_taint() {
            let w = taint_warnings_for(
                "set x [gets stdin]\ntrace add variable x write {sanitize}\nputs $x",
            );
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "trace installation must not launder x's existing taint, got {w:?}"
            );
        }

        /// FN direction: a variable set to a literal (clean) value is
        /// conservatively treated as tainted once *any* trace (even a
        /// `read` trace) is attached — the handler could inject
        /// attacker-controlled content on every future access.
        #[test]
        fn traced_literal_value_is_conservatively_tainted() {
            let w = taint_warnings_for(
                "set x \"safe-literal\"\ntrace add variable x read {inject}\nputs $x",
            );
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "a traced variable must be treated as tainted even from a literal set, got {w:?}"
            );
        }

        /// Cross-proc: a trace installed by one proc still taints a read of
        /// the same global variable in a different function (the module-wide
        /// `Module::traced_variables` over-approximation, not a per-function
        /// one).
        #[test]
        fn trace_installed_in_another_proc_still_taints_this_read() {
            let w = taint_warnings_for(
                "proc install_trace {} { trace add variable ::x write {cb} }\n\
                 set x [gets stdin]\n\
                 install_trace\n\
                 puts $x",
            );
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "expected T101 despite the trace living in a different proc, got {w:?}"
            );
        }

        /// A dynamic trace target (`trace add variable $name …`) can't be
        /// resolved to a literal name, so — mirroring
        /// `Module::has_dynamic_variable_trace` — every name in the module is
        /// conservatively treated as traced (tainted).
        #[test]
        fn dynamic_trace_target_taints_every_name() {
            let w = taint_warnings_for(
                "set x \"safe-literal\"\ntrace add variable $somevar write {cb}\nputs $x",
            );
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "a dynamic trace target must conservatively taint every name, got {w:?}"
            );
        }

        /// TN control: an untraced variable behaves exactly as before — a
        /// clean literal stays clean.
        #[test]
        fn untraced_literal_value_stays_clean() {
            let w = taint_warnings_for("set x hello\nputs $x");
            assert!(
                w.is_empty(),
                "no trace in scope, expected no warnings: {w:?}"
            );
        }

        /// TN control: an untraced tainted variable is unaffected by this
        /// fix — still flagged exactly as the pre-existing behaviour.
        #[test]
        fn untraced_tainted_value_still_flagged() {
            let w = taint_warnings_for("set x [gets stdin]\nputs $x");
            assert!(
                w.iter().any(|d| d.code == DiagCode::T101),
                "expected T101 for a plain untraced tainted puts call, got {w:?}"
            );
        }
    }

    /// Unit tests for the pure [`apply_module_variable_traces`] helper,
    /// isolated from the end-to-end plumbing tested by
    /// `variable_trace_taint` above.
    mod apply_module_variable_traces_tests {
        use super::*;
        use crate::compilation_unit::ModuleTraceFacts;

        #[test]
        fn forces_existing_entries_for_a_traced_name() {
            let cu = crate::compilation_unit::CompilationUnit::build_for(
                "proc p {} { trace add variable t write cb\nset t 1\nputs $t }",
                &CommandRegistry::build_default(),
                false,
            );
            assert!(cu.ir_module.traced_variables.contains("t"));
            let traces = ModuleTraceFacts {
                traced_variables: &cu.ir_module.traced_variables,
                has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            };

            let fu = cu.function("::p").unwrap();
            let patched = apply_module_variable_traces((*fu.taints).clone(), &fu.ssa, traces);
            let sym = fu.ssa.var_symbol("t").expect("t interned");
            assert!(
                patched.get(&(sym, 1)).is_some_and(|t| t.is_tainted()),
                "t#1 must be forced tainted: {patched:?}"
            );
        }

        #[test]
        fn leaves_untraced_names_unchanged() {
            let registry = CommandRegistry::build_default();
            let cu = crate::compilation_unit::CompilationUnit::build_for(
                "proc p {} { set t 1\nputs $t }",
                &registry,
                false,
            );
            assert!(!cu.ir_module.traced_variables.contains("t"));
            assert!(!cu.ir_module.has_dynamic_variable_trace);
            let traces = ModuleTraceFacts {
                traced_variables: &cu.ir_module.traced_variables,
                has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            };
            let fu = cu.function("::p").unwrap();
            let patched = apply_module_variable_traces((*fu.taints).clone(), &fu.ssa, traces);
            assert_eq!(
                patched, *fu.taints,
                "no trace in scope — map must be unchanged"
            );
        }
    }
}
