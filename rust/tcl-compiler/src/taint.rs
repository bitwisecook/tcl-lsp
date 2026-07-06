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
//! ## What is implemented (was previously listed as deferred)
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
//! Sinks are still resolved through the [`Traits::TAINT_SINK`] /
//! [`Traits::EVALUATES_CODE`] flags inside `find_taint_warnings`.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use bitflags::bitflags;
use rustc_hash::FxHashSet;

use tcl_lexer::{Lexer, SourceMap, Span, TokenType, backslash_subst};
use tcl_registry::dialects::DialectSet;
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::rendered_properties::{RenderedProperties, RenderedValueProps};
use crate::sccp::{SccpResult, cfg_order};
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};

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
            | Self::FQDN.bits(),
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
    /// always `None` for taint diagnostics — wired through ahead of
    /// the rich-fix work so the `PyO3` surface stays stable.
    pub replacement: Option<String>,
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
    dialect: Option<&str>,
) -> Option<TaintLattice> {
    let dialect_set = dialect_to_set(dialect);
    tcl_registry::taint::taint_source_colour(registry, command, args, dialect_set).map(|c| {
        TaintLattice {
            colours: reg_colour(c),
        }
    })
}

/// Bridge a `tcl_registry::TaintColour` to the compiler's mirror enum.
/// The bit layouts are identical (registry is the canonical source), so
/// a `from_bits_truncate` of the raw bits is exact.
fn reg_colour(c: tcl_registry::TaintColour) -> TaintColour {
    TaintColour::from_bits_truncate(c.bits())
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
pub fn is_irules_dialect(dialect: Option<&str>) -> bool {
    tcl_registry::prelude::DialectSet::is_irules_dialect(dialect)
}

fn dialect_to_set(dialect: Option<&str>) -> DialectSet {
    if is_irules_dialect(dialect) {
        DialectSet::IRULES
    } else {
        match dialect.and_then(DialectSet::parse) {
            Some(d) => d,
            None => DialectSet::empty(),
        }
    }
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
    pub(crate) dialect: Option<&'a str>,
    /// Colour-aware return summaries from the interprocedural taint
    /// solve (`taint_interproc::solve_interprocedural_taints`). When
    /// present, calls to a known proc apply the full
    /// [`crate::taint_interproc::apply_proc_return_summary`] transfer
    /// instead of the conservative single-passthrough rule.
    pub(crate) taint_summaries:
        Option<&'a HashMap<String, crate::taint_interproc::ProcTaintSummary>>,
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
        if let Some(t) = source_colour(ctx.registry, &cmd, &arg_refs, ctx.dialect) {
            return t;
        }
        // Interprocedural: if `cmd` resolves to a known proc with a
        // passthrough parameter, propagate the taint of the matching
        // actual.
        if let Some(t) = interproc_call_taint(&cmd, &args, uses, taints, ctx) {
            return t;
        }
        // Propagate from the arguments inside the command sub.
        let mut t = TaintLattice::clean();
        for arg in &args {
            t = t.join(word_taint(arg, uses, taints, ctx));
        }
        // Stamp the encoder/transform colour the command adds to a
        // tainted result (e.g. `uri::encode` → `URL_ENCODED`), so a
        // later pass through the same encoder is detectable as a
        // double-encode (T106).
        if t.is_tainted()
            && let Some(colour) = transform_colour(ctx.registry, &cmd, &arg_refs)
        {
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
                    t = t.join(word_taint(sub, uses, taints, ctx));
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
                if let Some(end) = rest.find('}') {
                    let n = &rest[1..end];
                    rest = &rest[end + 1..];
                    n
                } else {
                    break;
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

/// Determine the taint produced by a statement's definition(s).
fn evaluate_taint_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    ssa: &SsaFunction,
) -> TaintLattice {
    match stmt {
        // Expression: join taint from all used variables.
        Statement::AssignExpr { .. } => join_uses(uses, taints, ssa),

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
            if is_sanitiser(ctx.registry, command, &arg_refs) {
                return TaintLattice::clean();
            }
            if let Some(t) = source_colour(ctx.registry, command, &arg_refs, ctx.dialect) {
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
/// passes as the Rust-only conservative global-write taint seeding rather
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
/// present in the taint map. The Rust-only
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
#[must_use]
// `too_many_arguments`: the call sites live in `compilation_unit.rs` (another
// subsystem) and pass these analyses positionally; bundling would require
// editing those out-of-scope callers. The grouping is already minimal —
// each argument is an independent analysis input.
#[allow(clippy::too_many_arguments)]
pub(crate) fn propagate_taints(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
    rendered_props: Option<&HashMap<ValueKey, RenderedValueProps>>,
    interproc: Option<&InterproceduralAnalysis>,
    dialect: Option<&str>,
    param_taints: Option<&HashMap<String, TaintLattice>>,
    taint_summaries: Option<&HashMap<String, crate::taint_interproc::ProcTaintSummary>>,
) -> HashMap<ValueKey, TaintLattice> {
    let preds = cfg.predecessors();
    let order = cfg_order(cfg);

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
    };

    let mut taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
    seed_entry_taints(&mut taints, ssa, cfg, interproc, param_taints);

    let mut changed = true;
    while changed {
        changed = false;
        for bn in &order {
            if !sccp.executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };
            changed |= propagate_phi_taints(&mut taints, ssa_block, *bn, &preds, sccp);
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
}

/// Join phi taints from edge-executable predecessors into `taints` for one
/// block. Returns `true` if any lattice value changed.
fn propagate_phi_taints(
    taints: &mut HashMap<ValueKey, TaintLattice>,
    ssa_block: &crate::ssa::SsaBlock,
    bn: BlockId,
    preds: &HashMap<BlockId, HashSet<BlockId>>,
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
            let mut inferred = evaluate_taint_def(stmt, &ssa_stmt.uses, &*taints, ctx, ssa);
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
/// Covers:
/// - **T100** — code-execution sinks (`eval`, `exec`, `uplevel`,
///   `subst`, `expr` via `EVALUATES_CODE` / `TAINT_SINK` traits).
/// - **T101** — output sinks (`puts`).
/// - **IRULE3001 / IRULE3002 / IRULE3003 / IRULE3004** — iRules output
///   sinks, only under the `"f5-irules"` / `"irules"` dialect. See
///   [`classify_irules_sink`].
fn classify_sink(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&str>,
) -> Option<(DiagCode, String)> {
    if let Some(spec) = registry.get(command) {
        // T100: dangerous code-execution sinks.
        if spec.traits.contains(Traits::EVALUATES_CODE) {
            return Some((DiagCode::T100, command.to_owned()));
        }
        // expr, subst, exec also carry TAINT_SINK but not EVALUATES_CODE.
        if spec.traits.contains(Traits::TAINT_SINK) {
            // puts → T101 (output, not code execution).
            if command == "puts" {
                return Some((DiagCode::T101, "puts".to_owned()));
            }
            // Everything else with TAINT_SINK is T100.
            return Some((DiagCode::T100, command.to_owned()));
        }
    }

    // iRules-dialect sinks. Kept after registry-driven T100/T101 so
    // shared commands (currently none) would prefer the generic
    // classification.
    if is_irules_dialect(dialect)
        && let Some(hit) = classify_irules_sink(command, args)
    {
        return Some(hit);
    }

    None
}

/// Classify iRules-specific output sinks.
///
/// Recognised sinks:
///
/// | Command                               | Code        | Label              |
/// |---------------------------------------|-------------|--------------------|
/// | `HTTP::respond`                       | `IRULE3001` | `HTTP::respond`    |
/// | `HTTP::header insert\|replace`        | `IRULE3002` | `HTTP::header …`   |
/// | `HTTP::cookie insert\|replace`        | `IRULE3002` | `HTTP::cookie …`   |
/// | `HTTP::redirect`                      | `IRULE3004` | `HTTP::redirect`   |
/// | `log`                                 | `IRULE3003` | `log`              |
///
/// TODO: once the Rust command registry carries `taint_hints` /
/// `taint_output_sink_subcommands` metadata, replace the hardcoded
/// command list with registry lookups.
fn classify_irules_sink(command: &str, args: &[String]) -> Option<(DiagCode, String)> {
    match command {
        "HTTP::respond" => Some((DiagCode::Irule3001, command.to_owned())),
        "HTTP::header" | "HTTP::cookie" => {
            let sub = args.first().map(String::as_str);
            if matches!(sub, Some("insert" | "replace")) {
                Some((DiagCode::Irule3002, format!("{command} {}", sub.unwrap())))
            } else {
                None
            }
        }
        "HTTP::redirect" => Some((DiagCode::Irule3004, command.to_owned())),
        "log" => Some((DiagCode::Irule3003, command.to_owned())),
        _ => None,
    }
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
        out.push((DiagCode::T104, command.to_owned()));
    }
    if let Some(sub) = args.first()
        && spec.taint_interp_eval_subcommands.contains(&sub.as_str())
    {
        out.push((DiagCode::T105, format!("{command} {sub}")));
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
) -> Vec<TaintWarning> {
    let destructive = destructive_file_subs(registry);
    if destructive.is_empty() {
        return Vec::new();
    }
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
            // Collect candidate path variables in argument (source) order:
            // `ssa_stmt.uses` is a `HashMap`, so iterating it would pick a
            // nondeterministic variable for a multi-path sink like
            // `file rename $a $b` — making the warning's message (and the memo
            // vs whole-module builds) differ run-to-run.
            let mut seen: FxHashSet<String> = FxHashSet::default();
            let mut ordered: Vec<String> = Vec::new();
            for a in args.iter().skip(path_start) {
                for name in arg_var_names_ordered(a) {
                    if seen.insert(name.clone()) {
                        ordered.push(name);
                    }
                }
            }
            for name in &ordered {
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
                    span: *span,
                    variable: name.clone(),
                    sink_command: format!("file {sub}"),
                    code: DiagCode::W313,
                    message,
                    replacement: None,
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
fn arg_var_names(arg: &str) -> HashSet<String> {
    arg_var_names_ordered(arg).into_iter().collect()
}

/// Variable names referenced in `arg`, in left-to-right source order with
/// duplicates preserved.  Callers that need a deterministic *first* variable
/// (e.g. W313's "first offending path variable") iterate this rather than the
/// `HashSet` from [`arg_var_names`], whose order is nondeterministic.
fn arg_var_names_ordered(arg: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = arg.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        if bytes.get(i + 1) == Some(&b'{')
            && let Some(rel) = arg[i + 2..].find('}')
        {
            names.push(normalise_var_name(&arg[i + 2..i + 2 + rel]).to_owned());
            i = i + 2 + rel + 1;
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
        let (negated, var) = extract_guard_var(condition, false);
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
fn extract_guard_var(expr: &ExprNode, negated: bool) -> (bool, Option<String>) {
    match expr {
        ExprNode::Unary { operand, .. } => extract_guard_var(operand, !negated),
        ExprNode::Binary { left, right, .. } => {
            let (n, r) = extract_guard_var(left, negated);
            if r.is_some() {
                return (n, r);
            }
            extract_guard_var(right, negated)
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
fn emit_double_encode_warnings<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    command: &str,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    span: Span,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) {
    let Some(dup_colour) = registry
        .get(command)
        .and_then(|s| s.taint_double_encode_colour)
        .map(reg_colour)
    else {
        return;
    };
    let label = double_encode_label(dup_colour);
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
            });
            emitted.insert(sym);
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
    command: &str,
    args: &[String],
    var_name: &str,
    lat: TaintLattice,
) -> bool {
    if !lat.colours.contains(TaintColour::HEADER_TOKEN_SAFE) {
        return false;
    }
    if !matches!(command, "HTTP::header" | "HTTP::cookie") {
        return false;
    }
    if !matches!(args.first().map(String::as_str), Some("insert" | "replace")) {
        return false;
    }
    let Some(arg) = args.get(1) else { return false };
    let stripped = arg.trim();
    is_pure_var_ref(stripped) && normalise_var_name(stripped) == var_name
}

/// Find every taint warning across a whole compilation unit.
///
/// Public `*_for_cu` entry point (mirroring
/// [`crate::shimmer::find_shimmer_warnings_for_cu`]) composing the
/// `TaintWarning`-producing passes — sink detection, setter-constraint
/// violations, iRules URI-split suggestions, and destructive-file warnings
/// — over each function in the unit, in `cu.functions()` order. The
/// path-concatenation pass is omitted: its Rust lattice colours are not yet
/// assigned (latent, always empty today).
#[must_use]
pub fn find_taint_warnings_for_cu(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<TaintWarning> {
    let mut out = Vec::new();
    let solved = crate::taint_interproc::solve_interprocedural_taints(cu, registry, dialect);
    for fu in cu.analysable_functions() {
        let exec = &fu.sccp.executable_blocks;
        let taints = solved.taints_for(&fu.name, &fu.taints);
        out.extend(find_taint_warnings(
            &fu.cfg, &fu.ssa, taints, exec, registry, dialect,
        ));
        out.extend(find_setter_constraint_warnings(
            registry, &fu.cfg, &fu.ssa, taints, exec, dialect,
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
            &fu.cfg, &fu.ssa, taints, exec, registry,
        ));
    }
    out
}

/// Run sink detection over a single function.
///
/// For each SSA use of a tainted variable in a sink statement, emits
/// one `TaintWarning`. Iterates blocks in `cfg_order` for deterministic
/// diagnostic ordering (matching the other shimmer/taint passes).
#[must_use]
/// Emit taint sink warnings (`T100` family) for one function.
///
/// This is the per-function sink scan; the whole-unit `dataflow` / `diag`
/// aggregation calls it over the top level and every procedure unit. The
/// other warning kinds (setter-constraint / uri-split / path-concat /
/// destructive-file) are not emitted here yet.
pub fn find_taint_warnings<S: std::hash::BuildHasher, E: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    executable_blocks: &HashSet<BlockId, E>,
    registry: &CommandRegistry,
    dialect: Option<&str>,
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
            emit_statement_warnings(ssa_stmt, taints, registry, dialect, &mut warnings, ssa);
        }
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
fn emit_statement_warnings<S: std::hash::BuildHasher>(
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) {
    let stmt = &ssa_stmt.statement;
    let span = stmt.span();

    // AssignExpr / ExprEval: any tainted variable in the expression
    // is a T100 violation (direct expr injection).
    if matches!(
        stmt,
        Statement::AssignExpr { .. } | Statement::ExprEval { .. }
    ) {
        emit_expr_warnings(&ssa_stmt.uses, taints, span, warnings, ssa);
        return;
    }

    // Owned fallback for `AssignValue` — `parse_command_substitution`
    // returns an owned (String, Vec<String>) so we stash it in this
    // scope and borrow into the uniform `(command, call_args)` tuple
    // below. Preserves sub-command args so e.g.
    // `set _ [HTTP::header insert X-Foo $v]` still reaches the
    // IRULE3002 subcommand gate.
    let assign_parsed: Option<(String, Vec<String>)> = match stmt {
        Statement::AssignValue { value, .. } => parse_command_substitution(value.trim()),
        _ => None,
    };
    let (command, call_args): (&str, &[String]) = match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            (command.as_str(), args.as_slice())
        }
        Statement::AssignValue { .. } => match assign_parsed.as_ref() {
            Some((cmd, sub_args)) => (cmd.as_str(), sub_args.as_slice()),
            None => return,
        },
        _ => return,
    };

    // Emission order per statement:
    // T103 (regexp pattern) → T106 (double-encode) → the sink loop
    // (T100/output/log, then T102, then T104/T105).

    let env = TaintScan {
        uses: &ssa_stmt.uses,
        taints,
        ssa,
    };

    // T103: tainted data in a regexp/regsub pattern position.
    emit_regexp_pattern_warnings(command, call_args, &env, span, registry, warnings);

    // T106: re-encoding an already-encoded tainted value.
    emit_double_encode_warnings(
        registry,
        command,
        &ssa_stmt.uses,
        taints,
        span,
        warnings,
        ssa,
    );

    let sink_call = SinkCall {
        command,
        args: call_args,
        registry,
    };
    // Primary sink classification (T100 code-exec / T101 + iRules output / log).
    if let Some((code, sink_label)) = classify_sink(registry, command, call_args, dialect) {
        emit_sink_warnings(&env, span, code, &sink_label, &sink_call, warnings);
    }

    // T102: option injection — only for Call statements, after the primary
    // sink (T100/output/log).
    if let Statement::Call { args, .. } = stmt {
        emit_option_injection(command, args, &env, span, registry, dialect, warnings);
    }

    // Additional registry-driven SSRF / cross-interp sinks (T104 / T105),
    // which can co-occur with the primary classification.
    for (code, sink_label) in classify_network_interp_sinks(registry, command, call_args) {
        emit_sink_warnings(&env, span, code, &sink_label, &sink_call, warnings);
    }
}

/// First positional (pattern) argument index of `regexp` / `regsub`,
/// after skipping option switches (`-start` consumes a value, `--`
/// terminates). `args` excludes the command name.
fn regexp_pattern_index(args: &[String]) -> Option<usize> {
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if a == "-start" && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    (i < args.len()).then_some(i)
}

/// Per-statement read context for the taint-warning emitters: the SSA
/// versions reaching the statement (`uses`), the taint lattice, and the SSA
/// function they resolve against. Bundled so the emitters stay within the
/// argument limit.
struct TaintScan<'a, S> {
    uses: &'a HashMap<Symbol, u32>,
    taints: &'a HashMap<ValueKey, TaintLattice, S>,
    ssa: &'a SsaFunction,
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
    let mut names: Vec<String> = arg_var_names(arg).into_iter().collect();
    names.sort_unstable();
    for var in names {
        let Some(sym) = ssa.var_symbol(&var) else {
            continue;
        };
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
        });
    }
}

/// Emit T100 warnings for every tainted use in an expression context.
fn emit_expr_warnings<S: std::hash::BuildHasher>(
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    span: Span,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) {
    for (&sym, &ver) in uses {
        let name = ssa.var_name(sym);
        if is_seeded_global_v0(name, ver) {
            continue;
        }
        let t = taints
            .get(&(sym, ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if t.is_tainted() {
            warnings.push(TaintWarning {
                span,
                variable: name.to_owned(),
                sink_command: "expr".to_owned(),
                code: DiagCode::T100,
                message: format!(
                    "Tainted variable ${name} flows into expr operand; \
                     numeric coercion may misinterpret value \
                     (use Tcl numeric-validation guards)"
                ),
                replacement: None,
            });
        }
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
fn sink_var_position_safe(
    registry: &CommandRegistry,
    code: DiagCode,
    command: &str,
    args: &[String],
    name: &str,
) -> bool {
    match code {
        // `puts ?-nonewline? ?channelId? string` — only the trailing
        // content arg is an output sink; a tainted channel id is a handle.
        DiagCode::T101 if command == "puts" => args
            .last()
            .is_none_or(|content| !arg_var_names(content).contains(name)),
        // T104 SSRF — only the network-address positional slots named by
        // `taint_network_sink_args`. `Some(&[])` (positions unspecified)
        // imposes no filter.
        DiagCode::T104 => {
            let Some(spec) = registry.get(command) else {
                return false;
            };
            let Some(positions) = spec.taint_network_sink_args else {
                return false;
            };
            if positions.is_empty() {
                return false;
            }
            let positionals = positional_arg_strings(spec, args);
            let in_network_slot = positions.iter().any(|&p| {
                positionals
                    .get(p as usize)
                    .is_some_and(|s| arg_var_names(s).contains(name))
            });
            !in_network_slot
        }
        _ => false,
    }
}

/// `true` when one of `args` is a `[list <head> …]` command substitution
/// whose constructed-list command word (`<head>`) is a literal known
/// registry command and `name` is referenced in that argument. The tainted
/// variable is then a *quoted argument* of the list, never the command
/// word, so `eval`/`uplevel`/`interp eval` of the list runs no injected
/// command. `[list $x …]` (variable head) and `[list]`-free args return
/// `false`.
fn list_wrapped_arg_command_is_literal(
    registry: &CommandRegistry,
    args: &[String],
    name: &str,
) -> bool {
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
        if arg_var_names(arg).contains(name) {
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
fn var_consumed_by_sanitiser(registry: &CommandRegistry, args: &[String], name: &str) -> bool {
    let mut seen = false;
    for arg in args {
        if !arg_var_names(arg).contains(name) {
            continue;
        }
        seen = true;
        let (residual, subs) = split_top_level_cmd_subs(arg);
        // A bare reference outside any command substitution reaches the sink.
        if arg_var_names(&residual).contains(name) {
            return false;
        }
        // Each top-level substitution that references `name` must be a sanitiser
        // (a sanitiser's result is a clean bounded value regardless of nesting).
        for sub in subs {
            if !arg_var_names(sub).contains(name) {
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
        // The value reaching the sink is clean when every appearance of `name`
        // in the sink arguments is consumed by an embedded sanitiser
        // substitution — `puts [string length $x]` outputs the integer length,
        // not `$x` (tclsh-verified). The expr-operand path applies this via
        // word_taint; mirror it here for the direct sink-argument path.
        if var_consumed_by_sanitiser(call.registry, call.args, name) {
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
            && irule3002_name_position_safe(call.command, call.args, name, t)
        {
            continue;
        }
        // Position-aware sink filter: a tainted variable only trips the
        // sink when it occupies a *dangerous* argument slot (the `puts`
        // content arg, a `taint_network_sink_args` network-address slot).
        if sink_var_position_safe(call.registry, code, call.command, call.args, name) {
            continue;
        }
        // `eval`/`uplevel`/`interp eval [list <known-cmd> $v …]`: the
        // command word of the constructed list is a literal known command,
        // so the tainted `$v` is a quoted argument, not the command word —
        // no code-injection vector LIST_CANONICAL
        // head-literal filter).
        if matches!(code, DiagCode::T100 | DiagCode::T105)
            && list_wrapped_arg_command_is_literal(call.registry, call.args, name)
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
            span,
            variable: name.to_owned(),
            sink_command: sink_label.to_owned(),
            code,
            message,
            replacement: None,
        });
        emitted.insert(sym);
    }
}

/// Emit T102 warnings for option injection into a `WARN_WITHOUT_TERMINATOR` command.
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
fn option_scan_region(
    args: &[String],
    scan_start: usize,
    options: &[tcl_registry::hover::OptionSpec],
) -> HashSet<usize> {
    let mut region: HashSet<usize> = HashSet::new();
    let mut i = scan_start;
    let n = args.len();
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

/// Emit `T102` (option injection) for tainted variables that sit in an
/// option-scanning position of a command declaring a `--` terminator.
///
/// The option-terminator profile
/// (`resolve_option_terminator`) supplies the subcommand-aware command
/// label and the scan start, `option_scan_region` filters positions, and
/// the `T102_SAFE` colour set mitigates.
fn emit_option_injection<S: std::hash::BuildHasher>(
    command: &str,
    args: &[String],
    env: &TaintScan<'_, S>,
    span: Span,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    warnings: &mut Vec<TaintWarning>,
) {
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

    let region = option_scan_region(args, profile.scan_start, profile.options);
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
        let mut names: Vec<String> = arg_var_names(arg).into_iter().collect();
        names.sort_unstable();
        for var in names {
            let Some(sym) = ssa.var_symbol(&var) else {
                continue;
            };
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
            warnings.push(TaintWarning {
                span,
                variable: var.clone(),
                sink_command: cmd_label.clone(),
                code: DiagCode::T102,
                message: format!(
                    "Tainted variable ${var} in option position of '{cmd_label}' \
                     without '--' terminator; risk of option injection"
                ),
                replacement: None,
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
    dialect: Option<&str>,
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
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
        };
        let ssa_eval = SsaStatement {
            statement: eval_call,
            uses: [(x, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
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
        };
        let ssa_sink = SsaStatement {
            statement: sink,
            uses: sink_uses
                .iter()
                .map(|&(n, v)| (ssa.intern_var(n), v))
                .collect(),
            defs: HashMap::new(),
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
        find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
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
    fn t106_double_encode_through_uri_encode() {
        // `set x [URI::encode $tainted]` stamps URL_ENCODED on x; passing
        // x back through `URI::encode` double-encodes → T106.
        use crate::cfg::{Function, Terminator};
        use crate::ssa::{SsaBlock, SsaFunction, SsaStatement};
        use tcl_lexer::Span;

        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_registry::dialects::DialectSet::IRULES);

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
        };
        let ssa_s1 = SsaStatement {
            statement: s1,
            uses: [(x, 1u32)].into_iter().collect(),
            defs: [(y, 1u32)].into_iter().collect(),
        };
        let ssa_s2 = SsaStatement {
            statement: s2,
            uses: [(y, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
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
            Some("f5-irules"),
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
        find_destructive_file_warnings(&cfg, &ssa, &taints, &sccp.executable_blocks, &registry)
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
            };
            let s1 = SsaStatement {
                statement: file_call(&["delete", "$p"]),
                uses: [(p, 1u32)].into_iter().collect(),
                defs: HashMap::new(),
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
                }]
            })
            .is_empty()
        );
    }

    #[test]
    fn w313_helpers_parse_vars_and_guards() {
        assert!(arg_var_names("$p").contains("p"));
        assert!(arg_var_names("${dir}/$file").contains("dir"));
        assert!(arg_var_names("${dir}/$file").contains("file"));
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
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
        };
        let ssa_regexp = SsaStatement {
            statement: regexp_call,
            uses: [(pattern, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
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
        };
        let ssa_regexp = SsaStatement {
            statement: regexp_call,
            uses: [(pattern, 1u32)].into_iter().collect(),
            defs: HashMap::new(),
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
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None, None, None);
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
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
        for dialect in [None, Some("f5-irules")] {
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
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, Some("f5-irules"));
        let mut out: Vec<TaintWarning> = Vec::new();
        for fu in cu.analysable_functions() {
            out.extend(find_taint_warnings(
                &fu.cfg,
                &fu.ssa,
                &fu.taints,
                &fu.sccp.executable_blocks,
                &registry,
                Some("f5-irules"),
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

    #[test]
    fn classify_irules_sink_http_respond() {
        let hit = classify_irules_sink("HTTP::respond", &[]);
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3001));
    }

    #[test]
    fn classify_irules_sink_http_header_insert() {
        let hit = classify_irules_sink(
            "HTTP::header",
            &["insert".to_owned(), "X-Foo".to_owned(), "bar".to_owned()],
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
        assert_eq!(hit.as_ref().unwrap().1, "HTTP::header insert");
    }

    #[test]
    fn classify_irules_sink_http_cookie_replace() {
        let hit = classify_irules_sink(
            "HTTP::cookie",
            &["replace".to_owned(), "sid".to_owned(), "val".to_owned()],
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some(DiagCode::Irule3002));
        assert_eq!(hit.as_ref().unwrap().1, "HTTP::cookie replace");
    }

    #[test]
    fn classify_irules_sink_http_header_remove_is_none() {
        let hit = classify_irules_sink("HTTP::header", &["remove".to_owned(), "X-Foo".to_owned()]);
        assert!(hit.is_none(), "remove subcommand must not emit IRULE3002");
    }

    #[test]
    fn classify_irules_sink_log_and_redirect() {
        assert_eq!(
            classify_irules_sink("log", &["local0.info".to_owned(), "x".to_owned()])
                .as_ref()
                .map(|(c, _)| *c),
            Some(DiagCode::Irule3003),
        );
        assert_eq!(
            classify_irules_sink("HTTP::redirect", &["https://evil".to_owned()])
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
            Some("tcl8.6"),
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
            Some("tcl8.6"),
        );
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T102),
            "no T102 when `--` precedes the tainted path, got {warnings:?}",
        );
    }

    #[test]
    fn irule_sinks_do_not_fire_without_dialect() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        // Without `with_interprocedural(Some("f5-irules"))`, no dialect
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
        let args = vec!["insert".to_owned(), "$name".to_owned(), "$value".to_owned()];
        let lat = TaintLattice::tainted().with(TaintColour::HEADER_TOKEN_SAFE);
        // Var `name` occupies arg-index 1 (name position) → suppressed.
        assert!(irule3002_name_position_safe(
            "HTTP::header",
            &args,
            "name",
            lat
        ));
        // Var `value` occupies arg-index 2 (value position) → not suppressed.
        assert!(!irule3002_name_position_safe(
            "HTTP::header",
            &args,
            "value",
            lat
        ));
        // Without HEADER_TOKEN_SAFE colour: never suppressed, even at name position.
        let plain = TaintLattice::tainted();
        assert!(!irule3002_name_position_safe(
            "HTTP::header",
            &args,
            "name",
            plain
        ));
        // Wrong subcommand: not suppressed.
        let rm_args = vec!["remove".to_owned(), "$name".to_owned()];
        assert!(!irule3002_name_position_safe(
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
        setter_warnings_for_dialect(source, Some("f5-irules"))
    }

    fn setter_warnings_for_dialect(source: &str, dialect: Option<&str>) -> Vec<TaintWarning> {
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
        // will light up once iRules source `taint_hints` reach the Rust
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
        let under_irules = setter_warnings_for_dialect("HTTP::uri foo", Some("f5-irules"));
        assert_eq!(under_irules.len(), 1);
        assert_eq!(under_irules[0].code, DiagCode::Irule3101);

        let under_none = setter_warnings_for_dialect("HTTP::uri foo", None);
        assert!(
            under_none.is_empty(),
            "no IRULE3101 under None dialect, got {under_none:?}"
        );

        let under_tcl = setter_warnings_for_dialect("HTTP::uri foo", Some("tcl"));
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
            tcl_registry::dialects::DialectSet::empty(),
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
        let irules = setter_warnings_for_dialect("HTTP::uri foo", Some("f5-irules"));
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
}
