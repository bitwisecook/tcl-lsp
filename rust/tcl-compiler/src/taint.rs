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
//!    nodes and variable copies.
//! 3. **`find_taint_warnings`** — sink check: emits **T100** when a
//!    tainted value reaches a code-execution sink (`eval`, `exec`,
//!    `uplevel`, `subst`, `expr`) and **T101** when it reaches an
//!    output sink (`puts`).
//!
//! ## What is not yet implemented
//!
//! - Inter-procedural summaries (C28): proc-to-proc taint transfer.
//! - Path-concat / URI-split heuristics.
//! - iRules-specific sink/source codes (IRULE3001–3004).
//! - T102 option-injection, T103 regex-injection, T104 SSRF, T105
//!   cross-interpreter injection — follow-up strips once the registry
//!   gains full taint-hint metadata.
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

#![allow(clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet};

use bitflags::bitflags;

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::Function as CfgFunction;
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
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
        let mitigations =
            (self.colours & other.colours) & !TaintColour::TAINTED;
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
fn is_taint_source(registry: &CommandRegistry, command: &str, args: &[&str]) -> bool {
    // Registry-driven: UNNORMALISED_HTTP_GETTER marks HTTP data getters.
    if let Some(spec) = registry.get(command) {
        if spec.traits.contains(Traits::UNNORMALISED_HTTP_GETTER) {
            return true;
        }
    }

    // Hardcoded core-Tcl sources (pending registry taint-hint metadata).
    match command {
        "gets" | "read" | "exec" | "socket" => true,
        "chan" => {
            // chan gets / chan read are sources; chan puts, configure, etc. are not.
            matches!(args.first().copied(), Some("gets") | Some("read"))
        }
        "encoding" => {
            // encoding convertfrom may decode attacker-controlled bytes.
            matches!(args.first().copied(), Some("convertfrom"))
        }
        _ => false,
    }
}

/// Return `true` when `command` is a sanitiser — its return value is
/// a fixed-type result that cannot carry taint through it.
///
/// Mirrors `_is_sanitiser` in Python: commands that return a numeric
/// type (INT or BOOLEAN) are sanitisers because their output is
/// type-determined, not content-determined.
fn is_sanitiser(registry: &CommandRegistry, command: &str) -> bool {
    let Some(spec) = registry.get(command) else {
        return false;
    };
    use tcl_registry::TclType;
    matches!(spec.return_type, Some(TclType::Int | TclType::Boolean))
}

// ---------------------------------------------------------------------------
// Taint propagation
// ---------------------------------------------------------------------------

/// Infer the taint of an argument word from already-known per-variable
/// taint values.
///
/// Handles pure variable references (`$x`), bracketed command
/// substitutions (`[cmd ...]`), and interpolated strings.
fn word_taint(
    word: &str,
    uses: &HashMap<String, u32>,
    taints: &HashMap<ValueKey, TaintLattice>,
    registry: &CommandRegistry,
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
        if is_sanitiser(registry, &cmd) {
            return TaintLattice::clean();
        }
        if is_taint_source(registry, &cmd, &arg_refs) {
            return TaintLattice::tainted();
        }
        // Propagate from the arguments inside the command sub.
        let mut t = TaintLattice::clean();
        for arg in &args {
            t = t.join(word_taint(arg, uses, taints, registry));
        }
        return t;
    }

    // Interpolated string: scan for $var references.
    if stripped.contains('$') || stripped.contains('[') {
        let mut t = TaintLattice::clean();
        // Simple scan: extract $name tokens from the word.
        let mut rest = stripped;
        while let Some(pos) = rest.find('$') {
            rest = &rest[pos + 1..];
            // ${name} form.
            let name = if rest.starts_with('{') {
                if let Some(end) = rest.find('}') {
                    let n = &rest[1..end];
                    rest = &rest[end + 1..];
                    n
                } else {
                    break;
                }
            } else {
                // $name — grab identifier chars.
                let end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_' && c != ':')
                    .unwrap_or(rest.len());
                let n = &rest[..end];
                rest = &rest[end..];
                n
            };
            if !name.is_empty() {
                t = t.join(var_taint(name, uses, taints));
            }
        }
        return t;
    }

    TaintLattice::clean()
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
    registry: &CommandRegistry,
) -> TaintLattice {
    match stmt {
        // Constants are always clean.
        Statement::AssignConst { .. } => TaintLattice::clean(),

        // Expression: join taint from all used variables.
        Statement::AssignExpr { .. } => join_uses(uses, taints),

        // Value assignment: evaluate the RHS word.
        Statement::AssignValue { value, .. } => {
            word_taint(value, uses, taints, registry)
        }

        // incr propagates taint from the variable being incremented.
        Statement::Incr { name, .. } => {
            let base = normalise_var_name(name);
            var_taint(base, uses, taints)
        }

        // Generic call that defines variables.
        Statement::Call { command, args, defs, .. } if !defs.is_empty() => {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            if is_sanitiser(registry, command) {
                return TaintLattice::clean();
            }
            if is_taint_source(registry, command, &arg_refs) {
                return TaintLattice::tainted();
            }
            // Propagate from arguments.
            let mut t = TaintLattice::clean();
            for arg in args {
                t = t.join(word_taint(arg, uses, taints, registry));
            }
            t
        }

        // Barrier widens all defs to tainted (conservative: unknown
        // side effects may expose attacker data).
        Statement::Barrier { .. } => TaintLattice::clean(),

        _ => TaintLattice::clean(),
    }
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

/// Run intra-procedural taint propagation over one SSA function.
///
/// Returns a map from `(variable_name, ssa_version)` to its taint
/// lattice value. Entries absent from the map are implicitly clean.
///
/// Sources are identified by [`is_taint_source`]. Propagation follows
/// SSA phi-join semantics: a phi is tainted if any incoming path is
/// tainted (`join` unions the `TAINTED` bit).
#[must_use]
pub fn propagate_taints(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable_blocks: &HashSet<String>,
    registry: &CommandRegistry,
) -> HashMap<ValueKey, TaintLattice> {
    let preds = cfg.predecessors();
    let order = cfg_order(cfg);

    let mut taints: HashMap<ValueKey, TaintLattice> = HashMap::new();

    let mut changed = true;
    while changed {
        changed = false;
        for bn in &order {
            if !executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };

            // Phi nodes: join taint from all executable predecessors.
            for phi in &ssa_block.phis {
                let exec_preds = preds
                    .get(bn)
                    .map(|ps| {
                        ps.iter()
                            .filter(|p| executable_blocks.contains(*p))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if exec_preds.is_empty() {
                    continue;
                }

                let mut phi_taint = TaintLattice::clean();
                for pred in exec_preds {
                    let ver = phi.incoming.get(pred).copied().unwrap_or(0);
                    if ver == 0 {
                        continue;
                    }
                    phi_taint = phi_taint.join(
                        taints
                            .get(&(phi.name.clone(), ver))
                            .copied()
                            .unwrap_or(TaintLattice::clean()),
                    );
                }

                let key = (phi.name.clone(), phi.version);
                let old = taints.get(&key).copied().unwrap_or(TaintLattice::clean());
                let merged = old.join(phi_taint);
                if merged != old {
                    taints.insert(key, merged);
                    changed = true;
                }
            }

            // Statements.
            for ssa_stmt in &ssa_block.statements {
                let stmt = &ssa_stmt.statement;
                for (var, &ver) in &ssa_stmt.defs {
                    let inferred =
                        evaluate_taint_def(stmt, &ssa_stmt.uses, &taints, registry);
                    let key = (var.clone(), ver);
                    let old = taints.get(&key).copied().unwrap_or(TaintLattice::clean());
                    let merged = old.join(inferred);
                    if merged != old {
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
fn classify_sink(
    registry: &CommandRegistry,
    command: &str,
) -> Option<(&'static str, String)> {
    let Some(spec) = registry.get(command) else {
        return None;
    };

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

/// Extract the command name from a statement if it has one.
#[allow(dead_code)]
fn stmt_command(stmt: &Statement) -> Option<&str> {
    match stmt {
        Statement::Call { command, .. } | Statement::Barrier { command, .. } => Some(command),
        Statement::AssignValue { value, .. } => {
            // [cmd ...] on the RHS.
            let stripped = value.trim();
            if stripped.starts_with('[') && stripped.ends_with(']') {
                // Return the command name portion.
                let inner = stripped[1..stripped.len() - 1].trim();
                Some(inner.split_ascii_whitespace().next().unwrap_or(""))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Run sink detection over a single function's CFG.
///
/// For each SSA use of a tainted variable in a sink statement, emits
/// one `TaintWarning`.
#[must_use]
pub fn find_taint_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    taints: &HashMap<ValueKey, TaintLattice>,
    executable_blocks: &HashSet<String>,
    registry: &CommandRegistry,
) -> Vec<TaintWarning> {
    let mut warnings: Vec<TaintWarning> = Vec::new();

    for bn in executable_blocks {
        let Some(block) = cfg.blocks.get(bn) else {
            continue;
        };
        let Some(ssa_block) = ssa.blocks.get(bn) else {
            continue;
        };

        for (idx, ssa_stmt) in ssa_block.statements.iter().enumerate() {
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };

            let span = stmt_span(stmt);

            // AssignExpr / ExprEval: any tainted variable in the expression
            // is a T100 violation (direct expr injection).
            match stmt {
                Statement::AssignExpr { .. } | Statement::ExprEval { .. } => {
                    for (name, &ver) in &ssa_stmt.uses {
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
                    continue;
                }
                _ => {}
            }

            // For Call / Barrier / AssignValue (command sub): classify sink.
            let command = match stmt {
                Statement::Call { command, .. } | Statement::Barrier { command, .. } => {
                    command.as_str()
                }
                Statement::AssignValue { value, .. } => {
                    let stripped = value.trim();
                    if stripped.starts_with('[') && stripped.ends_with(']') {
                        let inner = stripped[1..stripped.len() - 1].trim();
                        inner.split_ascii_whitespace().next().unwrap_or("")
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            // T102: option injection.
            // Applies when the command has WARN_WITHOUT_TERMINATOR and a
            // tainted pure-variable-reference argument appears before any
            // `--` terminator.  Only checked for Call statements — Barrier
            // args are already evaluated and cannot inject options.
            if let Statement::Call { args, .. } = stmt {
                emit_option_injection(
                    command,
                    args,
                    &ssa_stmt.uses,
                    taints,
                    span,
                    registry,
                    &mut warnings,
                );
            }

            let Some((code, sink_label)) = classify_sink(registry, command) else {
                continue;
            };

            // Check each used variable for taint.
            let mut emitted: HashSet<String> = HashSet::new();
            for (name, &ver) in &ssa_stmt.uses {
                if ver == 0 {
                    continue;
                }
                if emitted.contains(name) {
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
                    _ => format!(
                        "Tainted variable ${name} flows into {sink_label}"
                    ),
                };
                warnings.push(TaintWarning {
                    span,
                    variable: name.clone(),
                    sink_command: sink_label.clone(),
                    code: code.to_owned(),
                    message,
                });
                emitted.insert(name.clone());
            }
        }
    }

    warnings
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
    let Some(spec) = registry.get(command) else { return };
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

/// Extract the source span from a statement.
#[inline]
fn stmt_span(stmt: &Statement) -> Span {
    stmt.span()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        use crate::cfg::{Block, Function, Terminator};
        use crate::ssa::{Phi, SsaBlock, SsaFunction, SsaStatement};
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
        cfg.blocks.get_mut("entry").unwrap().statements.push(stmt.clone());
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

        let exec: HashSet<String> = ["entry".to_string()].into_iter().collect();
        let taints = propagate_taints(&cfg, &ssa, &exec, &registry);
        assert!(
            taints
                .get(&("x".to_string(), 1))
                .map_or(false, |t| t.is_tainted()),
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
        cfg.blocks.get_mut("entry").unwrap().statements.push(assign.clone());
        cfg.blocks.get_mut("entry").unwrap().statements.push(eval_call.clone());
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

        let exec: HashSet<String> = ["entry".to_string()].into_iter().collect();
        let taints = propagate_taints(&cfg, &ssa, &exec, &registry);
        let warnings = find_taint_warnings(&cfg, &ssa, &taints, &exec, &registry);

        assert!(
            warnings.iter().any(|w| w.code == "T100" && w.variable == "x"),
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

        let exec: HashSet<String> = ["entry".to_string()].into_iter().collect();
        let taints = propagate_taints(&cfg, &ssa, &exec, &registry);
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
        cfg.blocks.get_mut("entry").unwrap().statements.push(assign.clone());
        cfg.blocks.get_mut("entry").unwrap().statements.push(regexp_call.clone());
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

        let exec: HashSet<String> = ["entry".to_string()].into_iter().collect();
        let taints = propagate_taints(&cfg, &ssa, &exec, &registry);
        let warnings = find_taint_warnings(&cfg, &ssa, &taints, &exec, &registry);

        assert!(
            warnings.iter().any(|w| w.code == "T102" && w.variable == "pattern"),
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
        cfg.blocks.get_mut("entry").unwrap().statements.push(assign.clone());
        cfg.blocks.get_mut("entry").unwrap().statements.push(regexp_call.clone());
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

        let exec: HashSet<String> = ["entry".to_string()].into_iter().collect();
        let taints = propagate_taints(&cfg, &ssa, &exec, &registry);
        let warnings = find_taint_warnings(&cfg, &ssa, &taints, &exec, &registry);

        let t102: Vec<_> = warnings.iter().filter(|w| w.code == "T102").collect();
        assert!(t102.is_empty(), "expected no T102 when '--' terminator present, got {t102:?}");
    }
}
