//! Taint analysis — data-flow from tainted sources to dangerous
//! sinks, tracked through a multi-colour lattice.
//!
//! Ported from `core/compiler/taint/` (C29). This module provides:
//!
//! 1. **`TaintColour`** / **`TaintLattice`** — the colour lattice
//!    (unchanged from the prior stub strip).
//! 2. **`propagate_taints`** — intra-procedural worklist that seeds
//!    taint from known source commands (`gets`, `read`, `exec`,
//!    `chan`, `encoding convertfrom`) and propagates through SSA phi
//!    nodes and variable copies. Optionally consumes a
//!    `rendered_props` map to enrich each lattice with colours
//!    derived from string content (`STARTS_WITH_SLASH` →
//!    `PATH_PREFIXED`, absence of `STARTS_WITH_DASH` →
//!    `NON_DASH_PREFIXED`), and an
//!    [`InterproceduralAnalysis`](crate::interprocedural::InterproceduralAnalysis)
//!    to transfer taint across proc boundaries via passthrough
//!    parameters.
//! 3. **`find_taint_warnings`** — sink check: emits **T100** when a
//!    tainted value reaches a code-execution sink (`eval`, `exec`,
//!    `uplevel`, `subst`, `expr`) and **T101** when it reaches an
//!    output sink (`puts`).
//!
//! ## What is not yet implemented
//!
//! - Path-concat / URI-split heuristics.
//! - iRules-specific sink codes (IRULE3001–3004): sources are
//!   dialect-driven via the `dialect` parameter, but the iRules sink
//!   categorisation lives in the broader compiler-checks wiring.
//! - T103 regex-injection, T104 SSRF, T105 cross-interpreter injection
//!   — follow-up strips once the registry gains full taint-hint metadata.
//!
//! ## Source commands (hardcoded pending registry metadata)
//!
//! The Python registry tags source commands via `taint_hints()` on
//! each `CommandSpec`. The Rust registry does not carry that metadata
//! yet. Until it does, sources are identified by a static list that
//! mirrors the Python `TAINT_HINTS` entries for core Tcl:
//!
//! | Command | Arity match | Reason |
//! |---------|------------|--------|
//! | `gets`  | any | Reads from stdin / channel |
//! | `read`  | any | Reads from channel |
//! | `exec`  | any | Returns shell-command stdout |
//! | `socket`| any | Opens a network channel |
//! | `chan gets` / `chan read` | any | Channel reads via subcommand |
//! | `encoding convertfrom` | any | Decodes attacker-controlled bytes |
//!
//! When the active `dialect` is `"f5-irules"` / `"irules"`, the
//! iRules HTTP/URI/IP/TCP/UDP/SSL/STREAM namespace-prefixed
//! getters are treated as sources in addition to the registry-driven
//! `UNNORMALISED_HTTP_GETTER` trait — see `is_irules_source`.

#![allow(clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet};

use bitflags::bitflags;

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::Function as CfgFunction;
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::rendered_properties::{RenderedProperties, RenderedValueProps};
use crate::sccp::{cfg_order, SccpResult};
use crate::ssa::{SsaFunction, SsaStatement, ValueKey};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};

// ---------------------------------------------------------------------------
// Colour lattice
// ---------------------------------------------------------------------------

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
    /// Matches the Python `_CRLF_SAFE = CRLF_FREE | IP_ADDRESS | PORT
    /// | FQDN`. `HEADER_TOKEN_SAFE` is deliberately **not** included —
    /// it only suppresses IRULE3002 in the header/cookie *name*
    /// position and is handled by the call-site-aware
    /// `irule3002_name_position_safe` helper. `HTML_ESCAPED` and
    /// `URL_ENCODED` are also excluded: both can still carry raw
    /// CR/LF octets (HTML-escape rewrites `<`/`>`/`&`; URL-encode
    /// rewrites `%`), so neither proves header-injection safety.
    pub const CRLF_SAFE: Self = Self::from_bits_truncate(
        Self::CRLF_FREE.bits()
            | Self::IP_ADDRESS.bits()
            | Self::PORT.bits()
            | Self::FQDN.bits(),
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

    /// Intersect mitigating colours (must-have), union taint bits
    /// (may-have). This implements the standard lattice join for
    /// taint analysis.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
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

// ---------------------------------------------------------------------------
// Diagnostic type
// ---------------------------------------------------------------------------

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
    pub code: String,
    /// Formatted message.
    pub message: String,
    /// Optional replacement text for a code-action fix. Currently
    /// always `None` for taint diagnostics — wired through ahead of
    /// the rich-fix work so the `PyO3` surface stays stable.
    pub replacement: Option<String>,
}

// ---------------------------------------------------------------------------
// Source-command classification
// ---------------------------------------------------------------------------

/// Return `true` when `command` is a known taint source — i.e. its
/// return value may carry attacker-influenced data.
///
/// Mirrors the Python `TAINT_HINTS` entries for core Tcl commands.
/// Commands with `UNNORMALISED_HTTP_GETTER` trait (iRules dialect) are
/// also included once the registry carries that flag on actual specs.
/// When `dialect` is `"f5-irules"` / `"irules"`, iRules namespace
/// prefixes (`HTTP::`, `URI::`, `IP::`, …) are also treated as
/// attacker-controlled sources.
fn is_taint_source(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: Option<&str>,
) -> bool {
    // Registry-driven: UNNORMALISED_HTTP_GETTER marks HTTP data getters.
    if let Some(spec) = registry.get(command) {
        if spec.traits.contains(Traits::UNNORMALISED_HTTP_GETTER) {
            return true;
        }
    }

    // Hardcoded core-Tcl sources (pending registry taint-hint metadata).
    let core_hit = match command {
        "gets" | "read" | "exec" | "socket" => true,
        "chan" => {
            // chan gets / chan read are sources; chan puts, configure, etc. are not.
            matches!(args.first().copied(), Some("gets" | "read"))
        }
        "encoding" => {
            // encoding convertfrom may decode attacker-controlled bytes.
            matches!(args.first().copied(), Some("convertfrom"))
        }
        _ => false,
    };
    if core_hit {
        return true;
    }

    // iRules-dialect sources: commands under attacker-controlled
    // namespaces that carry no UNNORMALISED_HTTP_GETTER trait yet.
    if is_irules_dialect(dialect) && is_irules_source(command) {
        return true;
    }

    false
}

/// True when the supplied `dialect` enables iRules-specific taint rules.
///
/// Exposed so adjacent check modules (`compiler_checks`, `irules_checks`)
/// can gate their iRules-only diagnostics on the same predicate.
#[must_use]
pub fn is_irules_dialect(dialect: Option<&str>) -> bool {
    matches!(dialect, Some("f5-irules" | "irules"))
}

/// Return `true` when `command` is an iRules namespace-prefixed getter
/// whose return value carries attacker-controlled data (an HTTP
/// header, URI segment, transport tuple, or stream chunk).
///
/// Supplements `UNNORMALISED_HTTP_GETTER` for registry entries that
/// don't yet carry the trait.
fn is_irules_source(command: &str) -> bool {
    // Any command under one of these namespaces is treated as a
    // source. Callers are expected to pass the literal namespace-
    // qualified form as typed in iRules source.
    const PREFIXES: &[&str] = &[
        "HTTP::", "URI::", "IP::", "TCP::", "UDP::", "SSL::", "STREAM::",
    ];
    PREFIXES.iter().any(|p| command.starts_with(p))
}

/// Return `true` when `command` (with optional subcommand in `args`) is a
/// sanitiser — its return value is a fixed numeric type that cannot carry
/// taint through it.
///
/// Mirrors `_is_sanitiser` in Python: commands (or subcommands) that return
/// `Int` or `Boolean` are sanitisers because their output is type-determined,
/// not content-determined.  Subcommand specs are checked first so that, e.g.,
/// `string length` and `string is integer` are recognised as sanitisers even
/// though `string` itself has no top-level return type.
fn is_sanitiser(registry: &CommandRegistry, command: &str, args: &[&str]) -> bool {
    use tcl_registry::TclType;
    fn is_fixed_numeric(t: Option<TclType>) -> bool {
        matches!(t, Some(TclType::Int | TclType::Boolean))
    }
    let Some(spec) = registry.get(command) else {
        return false;
    };
    if let Some(sub_name) = args.first().copied() {
        if let Some(sub) = spec.subcommand(sub_name) {
            if is_fixed_numeric(sub.return_type) {
                return true;
            }
        }
    }
    is_fixed_numeric(spec.return_type)
}

// ---------------------------------------------------------------------------
// Taint propagation
// ---------------------------------------------------------------------------

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
struct TaintCtx<'a> {
    registry: &'a CommandRegistry,
    interproc: Option<&'a InterproceduralAnalysis>,
    known_procs: Option<&'a HashSet<String>>,
    caller_qname: Option<&'a str>,
    dialect: Option<&'a str>,
}

/// Infer the taint of an argument word from already-known per-variable
/// taint values.
///
/// Handles pure variable references (`$x`), bracketed command
/// substitutions (`[cmd ...]`), and interpolated strings.
fn word_taint(
    word: &str,
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    ctx: TaintCtx<'_>,
) -> TaintLattice {
    let stripped = word.trim();

    // Pure variable reference — inherit taint directly.
    if is_pure_var_ref(stripped) {
        let name = normalise_var_name(stripped);
        return var_taint(name, uses, taints);
    }

    // Bracketed command substitution.
    if let Some((cmd, args)) = parse_command_substitution(stripped) {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        if is_sanitiser(ctx.registry, &cmd, &arg_refs) {
            return TaintLattice::clean();
        }
        if is_taint_source(ctx.registry, &cmd, &arg_refs, ctx.dialect) {
            return TaintLattice::tainted();
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
                t = t.join(word_taint(sub, uses, taints, ctx));
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
                t = t.join(var_taint(name, uses, taints));
            }
        }
        return t;
    }

    TaintLattice::clean()
}

/// When `command` resolves to an internal proc with a known
/// `return_passthrough_param`, return the taint of the corresponding
/// actual argument. Returns `None` when interprocedural summaries are
/// not available or the call doesn't resolve.
fn interproc_call_taint(
    command: &str,
    args: &[String],
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    ctx: TaintCtx<'_>,
) -> Option<TaintLattice> {
    let interproc = ctx.interproc?;
    let known = ctx.known_procs?;
    let caller = ctx.caller_qname.unwrap_or("::top");
    let target = crate::interprocedural::resolve_internal_call(command, caller, known)?;
    let summary = interproc.procedures.get(&target)?;
    let passthrough = summary.return_passthrough_param.as_ref()?;
    let idx = summary.params.iter().position(|p| p == passthrough)?;
    let actual = args.get(idx)?;
    Some(word_taint(actual, uses, taints, ctx))
}

/// Look up taint for a named variable at its current SSA version.
fn var_taint(
    name: &str,
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
) -> TaintLattice {
    let ver = uses.get(name).copied().unwrap_or(0);
    if ver == 0 {
        // Version 0 means the variable may be read from enclosing scope.
        taints
            .get(&(name.to_owned(), 0))
            .copied()
            .unwrap_or(TaintLattice::clean())
    } else {
        taints
            .get(&(name.to_owned(), ver))
            .copied()
            .unwrap_or(TaintLattice::clean())
    }
}

/// Determine the taint produced by a statement's definition(s).
fn evaluate_taint_def(
    stmt: &Statement,
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    ctx: TaintCtx<'_>,
) -> TaintLattice {
    match stmt {
        // Expression: join taint from all used variables.
        Statement::AssignExpr { .. } => join_uses(uses, taints),

        // Value assignment: evaluate the RHS word.
        Statement::AssignValue { value, .. } => word_taint(value, uses, taints, ctx),

        // incr propagates taint from the variable being incremented.
        Statement::Incr { name, .. } => {
            let base = normalise_var_name(name);
            var_taint(base, uses, taints)
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
            if is_taint_source(ctx.registry, command, &arg_refs, ctx.dialect) {
                return TaintLattice::tainted();
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

/// Join taint from all SSA uses in a statement.
fn join_uses(
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
) -> TaintLattice {
    let mut t = TaintLattice::clean();
    for (name, &ver) in uses {
        if ver > 0 {
            t = t.join(
                taints
                    .get(&(name.clone(), ver))
                    .copied()
                    .unwrap_or(TaintLattice::clean()),
            );
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
    let mut visited: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { command, .. } = stmt {
                if let Some(target) = crate::interprocedural::resolve_internal_call(
                    command,
                    ssa.name.as_str(),
                    &known,
                ) {
                    if let Some(summary) = ia.procedures.get(&target) {
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
fn collect_global_reads(ssa: &SsaFunction) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();
    let mut consider = |name: &str| {
        if name.starts_with("::") {
            out.insert(name.to_owned());
        }
    };
    for block in ssa.blocks.values() {
        for name in block.entry_versions.keys() {
            consider(name);
        }
        for phi in &block.phis {
            if phi.incoming.values().any(|&v| v == 0) {
                consider(&phi.name);
            }
        }
        for stmt in &block.statements {
            for (name, &ver) in &stmt.uses {
                if ver == 0 {
                    consider(name);
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
#[must_use]
pub fn propagate_taints(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
    rendered_props: Option<&HashMap<ValueKey, RenderedValueProps>>,
    interproc: Option<&InterproceduralAnalysis>,
    dialect: Option<&str>,
) -> HashMap<ValueKey, TaintLattice> {
    let preds = cfg.predecessors();
    let order = cfg_order(cfg);

    // Precompute the set of known procedure names once so per-call
    // resolution in `interproc_call_taint` is O(1) rather than
    // O(procedures) per call site.
    let known_procs: Option<HashSet<String>> =
        interproc.map(|ia| ia.procedures.keys().cloned().collect());
    let ctx = TaintCtx {
        registry,
        interproc,
        known_procs: known_procs.as_ref(),
        caller_qname: Some(ssa.name.as_str()),
        dialect,
    };

    let mut taints: HashMap<ValueKey, TaintLattice> = HashMap::new();

    // Seed: when a callee reachable from the current function writes
    // to global scope, taint version-0 reads of global/namespace
    // variables that this function actually touches. Scoping to
    // reachable callees prevents an unrelated helper proc's global
    // writes from polluting functions that never invoke it; scanning
    // *every* block's entry_versions (plus statement uses / phi
    // incomings) ensures we discover globals even when the entry
    // block has no seeded versions.
    if let Some(ia) = interproc {
        if reachable_writes_global(ssa, cfg, ia) {
            let globals = collect_global_reads(ssa);
            for name in globals {
                taints.entry((name, 0)).or_insert(TaintLattice::tainted());
            }
        }
    }

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

            // Phi nodes: join taint from edge-executable predecessors only.
            // Using executable_edges (not just executable_blocks) ensures
            // taint does not flow through SCCP-proven dead branches.
            for phi in &ssa_block.phis {
                let exec_preds = preds
                    .get(bn)
                    .map(|ps| {
                        ps.iter()
                            .filter(|p| {
                                sccp.executable_edges
                                    .contains(&((*p).to_owned(), bn.clone()))
                            })
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
                        .get(&(phi.name.clone(), ver))
                        .copied()
                        .unwrap_or(TaintLattice::clean());
                    phi_taint = Some(match phi_taint {
                        Some(existing) => existing.join(incoming),
                        None => incoming,
                    });
                }

                let Some(phi_taint) = phi_taint else { continue };
                let key = (phi.name.clone(), phi.version);
                let merged = match taints.get(&key) {
                    Some(&old) => old.join(phi_taint),
                    None => phi_taint,
                };
                if taints.get(&key) != Some(&merged) {
                    taints.insert(key, merged);
                    changed = true;
                }
            }

            // Statements.
            for ssa_stmt in &ssa_block.statements {
                let stmt = &ssa_stmt.statement;
                for (var, &ver) in &ssa_stmt.defs {
                    let mut inferred = evaluate_taint_def(stmt, &ssa_stmt.uses, &taints, ctx);
                    // Enrich the inferred taint with rendered-property
                    // colours when available.
                    if let Some(rp) = rendered_props {
                        if let Some(p) = rp.get(&(var.clone(), ver)) {
                            inferred = colour_from_rendered(inferred, *p);
                        }
                    }
                    let key = (var.clone(), ver);
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
        }
    }

    taints
}

// ---------------------------------------------------------------------------
// Sink detection
// ---------------------------------------------------------------------------

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
) -> Option<(&'static str, String)> {
    if let Some(spec) = registry.get(command) {
        // T100: dangerous code-execution sinks.
        if spec.traits.contains(Traits::EVALUATES_CODE) {
            return Some(("T100", command.to_owned()));
        }
        // expr, subst, exec also carry TAINT_SINK but not EVALUATES_CODE.
        if spec.traits.contains(Traits::TAINT_SINK) {
            // puts → T101 (output, not code execution).
            if command == "puts" {
                return Some(("T101", "puts".to_owned()));
            }
            // Everything else with TAINT_SINK is T100.
            return Some(("T100", command.to_owned()));
        }
    }

    // iRules-dialect sinks. Kept after registry-driven T100/T101 so
    // shared commands (currently none) would prefer the generic
    // classification.
    if is_irules_dialect(dialect) {
        if let Some(hit) = classify_irules_sink(command, args) {
            return Some(hit);
        }
    }

    None
}

/// Classify iRules-specific output sinks.
///
/// Mirrors `_classify_sink` in `core/compiler/taint/_sinks.py`:
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
/// command list with registry lookups (matching the Python path via
/// `REGISTRY.classify_taint_sinks`).
fn classify_irules_sink(command: &str, args: &[String]) -> Option<(&'static str, String)> {
    match command {
        "HTTP::respond" => Some(("IRULE3001", command.to_owned())),
        "HTTP::header" | "HTTP::cookie" => {
            let sub = args.first().map(String::as_str);
            if matches!(sub, Some("insert" | "replace")) {
                Some(("IRULE3002", format!("{command} {}", sub.unwrap())))
            } else {
                None
            }
        }
        "HTTP::redirect" => Some(("IRULE3004", command.to_owned())),
        "log" => Some(("IRULE3003", command.to_owned())),
        _ => None,
    }
}

/// Return `true` when a tainted value `lat` is mitigated for the given
/// iRules sink code. Mirrors `_should_suppress_sink_warning` in
/// Python for the IRULE3001/3002/3003/3004 branches.
///
/// For IRULE3002 in the name-position (arg-index 1 of
/// `HTTP::header`/`HTTP::cookie` `insert`/`replace`), the
/// `HEADER_TOKEN_SAFE` colour is an additional mitigation. That extra
/// check is handled at the call site because it needs the per-use arg
/// index; the function signature here is deliberately kept narrow.
fn irules_sink_suppressed(code: &str, lat: TaintLattice) -> bool {
    if !lat.is_tainted() {
        return false;
    }
    match code {
        "IRULE3001" => lat.colours.intersects(TaintColour::HTML_ESCAPED),
        "IRULE3002" | "IRULE3003" => lat.colours.intersects(TaintColour::CRLF_SAFE),
        "IRULE3004" => lat.colours.intersects(TaintColour::REDIRECT_SAFE),
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

/// Run sink detection over a single function.
///
/// For each SSA use of a tainted variable in a sink statement, emits
/// one `TaintWarning`. Iterates blocks in `cfg_order` for deterministic
/// diagnostic ordering (matching the other shimmer/taint passes).
#[must_use]
pub fn find_taint_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice>,
    executable_blocks: &HashSet<String>,
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
            emit_statement_warnings(ssa_stmt, taints, registry, dialect, &mut warnings);
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
fn emit_statement_warnings(
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice>,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    warnings: &mut Vec<TaintWarning>,
) {
    let stmt = &ssa_stmt.statement;
    let span = stmt.span();

    // AssignExpr / ExprEval: any tainted variable in the expression
    // is a T100 violation (direct expr injection).
    if matches!(
        stmt,
        Statement::AssignExpr { .. } | Statement::ExprEval { .. }
    ) {
        emit_expr_warnings(&ssa_stmt.uses, taints, span, warnings);
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

    // T102: option injection — only for Call statements.
    if let Statement::Call { args, .. } = stmt {
        emit_option_injection(
            command,
            args,
            &ssa_stmt.uses,
            taints,
            span,
            registry,
            warnings,
        );
    }

    let Some((code, sink_label)) = classify_sink(registry, command, call_args, dialect) else {
        return;
    };

    emit_sink_warnings(
        &ssa_stmt.uses,
        taints,
        span,
        code,
        &sink_label,
        &SinkCall {
            command,
            args: call_args,
        },
        warnings,
    );
}

/// Emit T100 warnings for every tainted use in an expression context.
fn emit_expr_warnings(
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    span: Span,
    warnings: &mut Vec<TaintWarning>,
) {
    for (name, &ver) in uses {
        if ver == 0 {
            continue;
        }
        let t = taints
            .get(&(name.clone(), ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if t.is_tainted() {
            warnings.push(TaintWarning {
                span,
                variable: name.clone(),
                sink_command: "expr".to_owned(),
                code: "T100".to_owned(),
                message: format!(
                    "Tainted variable ${name} used in expr; \
                     possible code injection"
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
}

fn emit_sink_warnings(
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    span: Span,
    code: &str,
    sink_label: &str,
    call: &SinkCall<'_>,
    warnings: &mut Vec<TaintWarning>,
) {
    let mut emitted: HashSet<String> = HashSet::new();
    for (name, &ver) in uses {
        if ver == 0 || emitted.contains(name) {
            continue;
        }
        let t = taints
            .get(&(name.clone(), ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if !t.is_tainted() {
            continue;
        }
        // Per-code mitigation suppression (IRULE3001–3004).
        if irules_sink_suppressed(code, t) {
            continue;
        }
        if code == "IRULE3002" && irule3002_name_position_safe(call.command, call.args, name, t) {
            continue;
        }
        let message = match code {
            "T100" => format!(
                "Tainted variable ${name} flows into {sink_label}; \
                 possible code injection"
            ),
            "T101" => format!(
                "Tainted variable ${name} flows into {sink_label}; \
                 output may contain injected content"
            ),
            "IRULE3001" => format!(
                "Tainted variable ${name} in HTTP response body ({sink_label}); \
                 risk of XSS or content injection"
            ),
            "IRULE3002" => format!(
                "Tainted variable ${name} in HTTP header/cookie value ({sink_label}); \
                 risk of header injection"
            ),
            "IRULE3003" => format!(
                "Tainted variable ${name} in log output ({sink_label}); \
                 risk of log injection or log forging"
            ),
            "IRULE3004" => format!(
                "Tainted variable ${name} in redirect URL ({sink_label}); \
                 risk of open redirect"
            ),
            _ => format!("Tainted variable ${name} flows into {sink_label}"),
        };
        warnings.push(TaintWarning {
            span,
            variable: name.clone(),
            sink_command: sink_label.to_owned(),
            code: code.to_owned(),
            message,
            replacement: None,
        });
        emitted.insert(name.clone());
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
fn emit_option_injection(
    command: &str,
    args: &[String],
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    span: Span,
    registry: &CommandRegistry,
    warnings: &mut Vec<TaintWarning>,
) {
    let Some(spec) = registry.get(command) else {
        return;
    };
    if !spec.traits.contains(Traits::WARN_WITHOUT_TERMINATOR) {
        return;
    }
    // Find the position of a `--` terminator, if present.
    let terminator_pos = args.iter().position(|a| a == "--");
    let mut emitted: HashSet<String> = HashSet::new();
    for (i, arg) in args.iter().enumerate() {
        if terminator_pos.is_some_and(|tp| i >= tp) {
            // This arg is after `--`; option injection is not possible.
            break;
        }
        let stripped = arg.trim();
        if !is_pure_var_ref(stripped) {
            continue;
        }
        let var = normalise_var_name(stripped);
        let Some(&ver) = uses.get(var) else { continue };
        if ver == 0 {
            continue;
        }
        let t = taints
            .get(&(var.to_owned(), ver))
            .copied()
            .unwrap_or(TaintLattice::clean());
        if !t.is_tainted() {
            continue;
        }
        if emitted.contains(var) {
            continue;
        }
        warnings.push(TaintWarning {
            span,
            variable: var.to_owned(),
            sink_command: command.to_owned(),
            code: "T102".to_owned(),
            message: format!(
                "T102: tainted variable ${var} passed to '{command}' without \
                 '--' option terminator; value starting with '-' causes option injection"
            ),
            replacement: None,
        });
        emitted.insert(var.to_owned());
    }
}

// ---------------------------------------------------------------------------
// IRULE3101 — setter-constraint violations
// ---------------------------------------------------------------------------

/// Command table for setter-constraint checks.
///
/// Entries are `(command, required_prefix, code, message)`. Matches
/// `TAINT_HINTS[cmd].setter_constraints` in Python for `HTTP::uri` /
/// `HTTP::path`. Both constrain `arg_index = 0` today.
const SETTER_CONSTRAINTS: &[(&str, &str, &str, &str)] = &[
    (
        "HTTP::uri",
        "/",
        "IRULE3101",
        "HTTP::uri value must start with '/'",
    ),
    (
        "HTTP::path",
        "/",
        "IRULE3101",
        "HTTP::path value must start with '/'",
    ),
];

/// Find setter-constraint violations (IRULE3101) — ports
/// `_find_setter_constraint_violations` in Python. Currently constrains
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
pub fn find_setter_constraint_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice>,
    executable_blocks: &HashSet<String>,
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
            for (cmd_name, prefix, code, message) in SETTER_CONSTRAINTS {
                if command != cmd_name {
                    continue;
                }
                // arg_index = 0 for the built-in table.
                let Some(arg_val) = args.first() else {
                    continue;
                };
                let stripped = arg_val.trim();

                // Literal: neither `$` nor `[`.
                if !stripped.starts_with('$') && !stripped.contains('[') {
                    if !stripped.starts_with(prefix) {
                        out.push(TaintWarning {
                            span,
                            variable: String::new(),
                            sink_command: (*cmd_name).to_owned(),
                            code: (*code).to_owned(),
                            message: (*message).to_owned(),
                            replacement: None,
                        });
                    }
                    continue;
                }

                // Pure variable reference: check SSA-resolved taint colour.
                if is_pure_var_ref(stripped) {
                    let var_name = normalise_var_name(stripped);
                    let ver = ssa_stmt.uses.get(var_name).copied().unwrap_or(0);
                    let t = taints
                        .get(&(var_name.to_owned(), ver))
                        .copied()
                        .unwrap_or(TaintLattice::clean());
                    if t.is_tainted() && t.colours.intersects(safe_path_colours) {
                        continue;
                    }
                    out.push(TaintWarning {
                        span,
                        variable: var_name.to_owned(),
                        sink_command: (*cmd_name).to_owned(),
                        code: (*code).to_owned(),
                        message: (*message).to_owned(),
                        replacement: None,
                    });
                    continue;
                }

                // Dynamic (interpolation, command sub, mixed) — always warn.
                out.push(TaintWarning {
                    span,
                    variable: String::new(),
                    sink_command: (*cmd_name).to_owned(),
                    code: (*code).to_owned(),
                    message: (*message).to_owned(),
                    replacement: None,
                });
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sccp::SccpResult;

    fn simple_sccp(blocks: &[&str]) -> SccpResult {
        SccpResult {
            values: HashMap::new(),
            executable_blocks: blocks.iter().copied().map(String::from).collect(),
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
        let a = TaintLattice {
            colours: TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::PATH_PREFIXED,
        };
        let b = TaintLattice {
            colours: TaintColour::CRLF_FREE | TaintColour::NON_DASH_PREFIXED,
        };
        let j = a.join(b);
        assert!(j.colours.contains(TaintColour::TAINTED));
        assert!(j.colours.contains(TaintColour::CRLF_FREE));
        assert!(!j.colours.contains(TaintColour::PATH_PREFIXED));
        assert!(!j.colours.contains(TaintColour::NON_DASH_PREFIXED));
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
    fn crlf_safe_mask_matches_python() {
        // Mirror Python's `_CRLF_SAFE = CRLF_FREE | IP_ADDRESS | PORT | FQDN`.
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
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let mut cfg = Function::new("::top", "entry");
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(stmt.clone());
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let ssa_stmt = SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: [("x".to_string(), 1u32)].into_iter().collect(),
        };
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert(
            "entry".into(),
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&["entry"]);
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None);
        assert!(
            taints
                .get(&("x".to_string(), 1))
                .is_some_and(|t| t.is_tainted()),
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
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let eval_call = Statement::Call {
            span: Span::new(13, 20),
            command: "eval".into(),
            args: vec!["$x".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };

        let mut cfg = Function::new("::top", "entry");
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(assign.clone());
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(eval_call.clone());
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let ssa_assign = SsaStatement {
            statement: assign,
            uses: HashMap::new(),
            defs: [("x".to_owned(), 1u32)].into_iter().collect(),
        };
        let ssa_eval = SsaStatement {
            statement: eval_call,
            uses: [("x".to_owned(), 1u32)].into_iter().collect(),
            defs: HashMap::new(),
        };

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert(
            "entry".into(),
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_assign, ssa_eval],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&["entry"]);
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None);
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
                .any(|w| w.code == "T100" && w.variable == "x"),
            "expected T100 for tainted $x passed to eval, got {warnings:?}"
        );
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
            value: "hello".into(),
        };
        let ssa_stmt = SsaStatement {
            statement: stmt.clone(),
            uses: HashMap::new(),
            defs: [("x".to_owned(), 1u32)].into_iter().collect(),
        };

        let mut cfg = Function::new("::top", "entry");
        cfg.blocks.get_mut("entry").unwrap().statements.push(stmt);
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert(
            "entry".into(),
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&["entry"]);
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None);
        assert!(
            taints
                .get(&("x".to_string(), 1))
                .map_or(true, |t| !t.is_tainted()),
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
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let regexp_call = Statement::Call {
            span: Span::new(26, 50),
            command: "regexp".into(),
            args: vec!["$pattern".into(), "haystack_value".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };

        let mut cfg = Function::new("::top", "entry");
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(assign.clone());
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(regexp_call.clone());
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let ssa_assign = SsaStatement {
            statement: assign,
            uses: HashMap::new(),
            defs: [("pattern".to_owned(), 1u32)].into_iter().collect(),
        };
        let ssa_regexp = SsaStatement {
            statement: regexp_call,
            uses: [("pattern".to_owned(), 1u32)].into_iter().collect(),
            defs: HashMap::new(),
        };

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert(
            "entry".into(),
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_assign, ssa_regexp],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&["entry"]);
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None);
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
                .any(|w| w.code == "T102" && w.variable == "pattern"),
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
            value: "[gets stdin]".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        // regexp -- $pattern $haystack  (safe: -- terminates option parsing)
        let regexp_call = Statement::Call {
            span: Span::new(26, 55),
            command: "regexp".into(),
            args: vec!["--".into(), "$pattern".into(), "haystack_value".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };

        let mut cfg = Function::new("::top", "entry");
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(assign.clone());
        cfg.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(regexp_call.clone());
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });

        let ssa_assign = SsaStatement {
            statement: assign,
            uses: HashMap::new(),
            defs: [("pattern".to_owned(), 1u32)].into_iter().collect(),
        };
        let ssa_regexp = SsaStatement {
            statement: regexp_call,
            uses: [("pattern".to_owned(), 1u32)].into_iter().collect(),
            defs: HashMap::new(),
        };

        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert(
            "entry".into(),
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![ssa_assign, ssa_regexp],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let sccp = simple_sccp(&["entry"]);
        let taints = propagate_taints(&cfg, &ssa, &sccp, &registry, None, None, None);
        let warnings = find_taint_warnings(
            &cfg,
            &ssa,
            &taints,
            &sccp.executable_blocks,
            &registry,
            None,
        );

        let t102: Vec<_> = warnings.iter().filter(|w| w.code == "T102").collect();
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
            .any(|((name, _ver), t)| name == "out" && t.is_tainted());
        assert!(
            out_tainted,
            "expected 'out' to be tainted via interpolated string embedding tainted $x"
        );
    }

    /// iRules-dialect: `HTTP::uri` is a taint source when dialect is
    /// enabled, and clean when it is not.
    #[test]
    fn irules_http_uri_is_source_under_dialect() {
        use crate::compilation_unit::CompilationUnit;

        let registry = CommandRegistry::build_default();

        // Without the dialect: HTTP::uri is unknown → not a source.
        let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry, false);
        let fu = cu.function("::top").unwrap();
        assert!(
            !fu.taints
                .iter()
                .any(|((n, _), t)| n == "u" && t.is_tainted()),
            "without iRules dialect, HTTP::uri should not be a source",
        );

        // With the dialect: `with_interprocedural("f5-irules")` rebuilds
        // taint, which is where the dialect takes effect.
        let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry, false)
            .with_interprocedural(&registry, Some("f5-irules"));
        let fu = cu.function("::top").unwrap();
        assert!(
            fu.taints
                .iter()
                .any(|((n, _), t)| n == "u" && t.is_tainted()),
            "under f5-irules, HTTP::uri should be a taint source",
        );
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
                .any(|((n, _), t)| n == "out" && t.is_tainted()),
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
            .any(|((n, _), t)| n == "local" && t.is_tainted());
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
            .find(|((n, _), _)| n == "path")
            .expect("path taint entry");
        assert!(
            entry.1.colours.contains(TaintColour::PATH_PREFIXED),
            "expected PATH_PREFIXED colour on /-prefixed literal",
        );
    }

    // -----------------------------------------------------------------------
    // IRULE3001–3004 sink classifier + end-to-end detection
    // -----------------------------------------------------------------------

    fn irules_warnings_for(source: &str) -> Vec<TaintWarning> {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, Some("f5-irules"));
        let mut out: Vec<TaintWarning> = Vec::new();
        for fu in cu.functions() {
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
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some("IRULE3001"));
    }

    #[test]
    fn classify_irules_sink_http_header_insert() {
        let hit = classify_irules_sink(
            "HTTP::header",
            &["insert".to_owned(), "X-Foo".to_owned(), "bar".to_owned()],
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some("IRULE3002"));
        assert_eq!(hit.as_ref().unwrap().1, "HTTP::header insert");
    }

    #[test]
    fn classify_irules_sink_http_cookie_replace() {
        let hit = classify_irules_sink(
            "HTTP::cookie",
            &["replace".to_owned(), "sid".to_owned(), "val".to_owned()],
        );
        assert_eq!(hit.as_ref().map(|(c, _)| *c), Some("IRULE3002"));
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
            Some("IRULE3003"),
        );
        assert_eq!(
            classify_irules_sink("HTTP::redirect", &["https://evil".to_owned()])
                .as_ref()
                .map(|(c, _)| *c),
            Some("IRULE3004"),
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
            w.iter().any(|x| x.code == "IRULE3001"),
            "expected IRULE3001, got {w:?}"
        );
    }

    #[test]
    fn irule3001_no_warning_for_literal_body() {
        let w = irules_warnings_for("HTTP::respond 200 content \"hello\"");
        assert!(
            w.iter().all(|x| x.code != "IRULE3001"),
            "expected no IRULE3001 on literal body, got {w:?}"
        );
    }

    #[test]
    fn irule3002_fires_on_tainted_header_value() {
        let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header insert X-Echo $v");
        assert!(
            w.iter().any(|x| x.code == "IRULE3002"),
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
            w.iter().any(|x| x.code == "IRULE3002"),
            "expected IRULE3002 inside command-sub sink, got {w:?}"
        );
    }

    #[test]
    fn irule3002_skipped_on_remove_subcommand() {
        let w = irules_warnings_for("set v [HTTP::header X-Src]\nHTTP::header remove $v");
        assert!(
            w.iter().all(|x| x.code != "IRULE3002"),
            "remove subcommand must not fire IRULE3002, got {w:?}"
        );
    }

    #[test]
    fn irule3003_fires_on_tainted_log() {
        let w = irules_warnings_for("set u [HTTP::uri]\nlog local0.info $u");
        assert!(
            w.iter().any(|x| x.code == "IRULE3003"),
            "expected IRULE3003, got {w:?}"
        );
    }

    #[test]
    fn irule3004_fires_on_tainted_redirect() {
        let w = irules_warnings_for("set target [HTTP::header Location]\nHTTP::redirect $target");
        assert!(
            w.iter().any(|x| x.code == "IRULE3004"),
            "expected IRULE3004, got {w:?}"
        );
    }

    #[test]
    fn irule3004_redirect_safe_suppresses_via_lattice() {
        // Direct lattice check: tainted + REDIRECT_SAFE must suppress.
        // (Latent as an end-to-end test until iRules sources are tagged
        // with PATH_PREFIXED from their `taint_hints` — `HTTP::path` in
        // Python carries PATH_PREFIXED on the getter form.)
        let lat = TaintLattice::tainted().with(TaintColour::PATH_PREFIXED);
        assert!(irules_sink_suppressed("IRULE3004", lat));
        let lat = TaintLattice::tainted().with(TaintColour::PATH_NORMALISED);
        assert!(irules_sink_suppressed("IRULE3004", lat));
        // Plain tainted should not suppress.
        assert!(!irules_sink_suppressed(
            "IRULE3004",
            TaintLattice::tainted()
        ));
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
            warnings.iter().all(|w| !w.code.starts_with("IRULE")),
            "no IRULE warnings without dialect, got {warnings:?}"
        );
    }

    #[test]
    fn irules_sink_suppressed_html_escaped_only_mitigates_3001() {
        let tainted_html = TaintLattice::tainted().with(TaintColour::HTML_ESCAPED);
        // IRULE3001 (HTTP response body) — HTML_ESCAPED directly mitigates.
        assert!(irules_sink_suppressed("IRULE3001", tainted_html));
        // IRULE3002/3003 (header / log) — HTML_ESCAPED does NOT prove
        // CRLF-injection safety (the escape rewrites `<`/`>`/`&` but
        // leaves raw CR/LF untouched). Python parity: `_CRLF_SAFE`
        // excludes `HTML_ESCAPED`.
        assert!(!irules_sink_suppressed("IRULE3002", tainted_html));
        assert!(!irules_sink_suppressed("IRULE3003", tainted_html));
        // IRULE3004 (redirect) — also not mitigated by HTML_ESCAPED.
        assert!(!irules_sink_suppressed("IRULE3004", tainted_html));

        // `CRLF_FREE` does suppress IRULE3002/3003 (the one mitigation
        // Python accepts in the value position).
        let tainted_crlf_free = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
        assert!(irules_sink_suppressed("IRULE3002", tainted_crlf_free));
        assert!(irules_sink_suppressed("IRULE3003", tainted_crlf_free));
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

    // -----------------------------------------------------------------------
    // IRULE3101 — setter-constraint violations
    // -----------------------------------------------------------------------

    /// Default helper: run the setter check under the `f5-irules` dialect
    /// (which is the only dialect that can surface IRULE3101 post-internal-gate).
    fn setter_warnings_for(source: &str) -> Vec<TaintWarning> {
        setter_warnings_for_dialect(source, Some("f5-irules"))
    }

    fn setter_warnings_for_dialect(source: &str, dialect: Option<&str>) -> Vec<TaintWarning> {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default();
        let mut cu = CompilationUnit::build_for(source, &registry, false);
        if dialect.is_some() {
            cu = cu.with_interprocedural(&registry, dialect);
        }
        let mut out: Vec<TaintWarning> = Vec::new();
        for fu in cu.functions() {
            out.extend(find_setter_constraint_warnings(
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
        assert_eq!(w[0].code, "IRULE3101");
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
        assert_eq!(bad[0].code, "IRULE3101");
        let good = setter_warnings_for("HTTP::path /bar");
        assert!(good.is_empty());
    }

    #[test]
    fn irule3101_generic_taint_warns() {
        // `HTTP::header X-Foo` is a tainted source; reusing it as a path
        // has generic taint (no PATH_PREFIXED / _NORMALISED / _BOUNDED).
        let w = setter_warnings_for("set v [HTTP::header X-Foo]\nHTTP::uri $v");
        assert!(
            w.iter().any(|x| x.code == "IRULE3101"),
            "generic taint must fire IRULE3101, got {w:?}"
        );
    }

    #[test]
    fn irule3101_pure_var_ref_always_warns_without_safe_colour() {
        // Matches Python parity: a plain `$p` setter value (no taint +
        // no provable path colour) cannot be proved `/`-prefixed by the
        // static analyser, so IRULE3101 fires. Latent suppression paths
        // via tainted-with-PATH_PREFIXED / _NORMALISED / _BOUNDED colours
        // will light up once iRules source `taint_hints` reach the Rust
        // lattice.
        let w = setter_warnings_for("set p /safe\nHTTP::uri $p");
        assert!(
            w.iter().any(|x| x.code == "IRULE3101"),
            "pure var-ref setter value must warn without tainted-safe-colour, got {w:?}"
        );
    }

    #[test]
    fn irule3101_dynamic_command_sub_warns() {
        // RHS is a command sub `[foo]` → hits the dynamic branch, which
        // always warns. The Python literal-check also bails on `[`.
        let w = setter_warnings_for("HTTP::uri [something]");
        assert!(
            w.iter().any(|x| x.code == "IRULE3101"),
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
        assert_eq!(under_irules[0].code, "IRULE3101");

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
}
