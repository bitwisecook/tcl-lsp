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

    /// Colours that mitigate CRLF / header / log injection.
    pub const CRLF_SAFE: Self = Self::from_bits_truncate(
        Self::CRLF_FREE.bits()
            | Self::IP_ADDRESS.bits()
            | Self::PORT.bits()
            | Self::FQDN.bits()
            | Self::HEADER_TOKEN_SAFE.bits()
            | Self::HTML_ESCAPED.bits()
            | Self::URL_ENCODED.bits(),
    );
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
fn is_irules_dialect(dialect: Option<&str>) -> bool {
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
fn classify_sink(registry: &CommandRegistry, command: &str) -> Option<(&'static str, String)> {
    let spec = registry.get(command)?;

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

    None
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
            emit_statement_warnings(ssa_stmt, taints, registry, &mut warnings);
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

    // For Call / Barrier / AssignValue (command sub): classify sink.
    let command = match stmt {
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => command.as_str(),
        Statement::AssignValue { value, .. } => {
            let stripped = value.trim();
            if stripped.starts_with('[') && stripped.ends_with(']') {
                let inner = stripped[1..stripped.len() - 1].trim();
                inner.split_ascii_whitespace().next().unwrap_or("")
            } else {
                return;
            }
        }
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

    let Some((code, sink_label)) = classify_sink(registry, command) else {
        return;
    };

    emit_sink_warnings(&ssa_stmt.uses, taints, span, code, &sink_label, warnings);
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
            });
        }
    }
}

/// Emit one warning per tainted use flowing into a classified sink.
///
/// Deduplicates on variable name so the same variable appearing multiple
/// times in `uses` only produces one warning.
fn emit_sink_warnings(
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    span: Span,
    code: &str,
    sink_label: &str,
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
        let message = match code {
            "T100" => format!(
                "Tainted variable ${name} flows into {sink_label}; \
                 possible code injection"
            ),
            "T101" => format!(
                "Tainted variable ${name} flows into {sink_label}; \
                 output may contain injected content"
            ),
            _ => format!("Tainted variable ${name} flows into {sink_label}"),
        };
        warnings.push(TaintWarning {
            span,
            variable: name.clone(),
            sink_command: sink_label.to_owned(),
            code: code.to_owned(),
            message,
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
        });
        emitted.insert(var.to_owned());
    }
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
    fn crlf_safe_mask_includes_crlf_free() {
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::CRLF_FREE));
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::HEADER_TOKEN_SAFE));
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
        let warnings = find_taint_warnings(&cfg, &ssa, &taints, &sccp.executable_blocks, &registry);

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
        let warnings = find_taint_warnings(&cfg, &ssa, &taints, &sccp.executable_blocks, &registry);

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
        let warnings = find_taint_warnings(&cfg, &ssa, &taints, &sccp.executable_blocks, &registry);

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
}
