//! Use-site shimmer detection (S100 / S101).
//!
//! A use-site shimmer occurs when a variable holds a value of type A but
//! is passed to a command argument position that expects type B.  Tcl
//! silently converts the internal representation at runtime — e.g. a
//! `String` to `Int` for `incr`, or a `String` to `List` for `llength` —
//! potentially invalidating cached intreps on shared references.
//!
//! Diagnostics:
//! - **S100**: shimmer outside a loop.
//! - **S101**: shimmer inside a loop (higher severity — converts on every
//!   iteration).
//!
//! This pass covers:
//! - [`Statement::Call`] arguments tagged `shimmers=true` in the registry.
//! - [`Statement::Incr`] — always reads its variable as `Int`.

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, TclType};

use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::Function as CfgFunction;
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::{TypeKind, TypeLattice};
use crate::value_shapes::is_pure_var_ref;

use super::graph::loop_body_blocks;
use super::hints::{arg_shimmer_type, is_numeric_compatible};
use super::span::def_range_map;
use super::{type_name, ShimmerWarning};

/// Find use-site shimmer warnings for a function.
///
/// Walks every executable block in CFG order, checks each statement's
/// argument words against the registry's `arg_types`, and emits a
/// [`ShimmerWarning`] for each type mismatch where the variable's known
/// type differs from what the command requires.
#[must_use]
pub(crate) fn find_use_site_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<String>,
    registry: &CommandRegistry,
    values: &HashMap<ValueKey, LatticeValue>,
) -> Vec<ShimmerWarning> {
    let loop_blocks = loop_body_blocks(cfg);
    let def_map = def_range_map(ssa);
    let mut out: Vec<ShimmerWarning> = Vec::new();

    for block_name in cfg_order(cfg) {
        if !executable_blocks.contains(&block_name) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_name) else {
            continue;
        };
        let in_loop = loop_blocks.contains(&block_name);
        // Per-block coercion ledger: once a use coerces `(var, ver)` to a
        // target intrep, the runtime representation has already changed, so a
        // later use to the *same* target in the same block is not a second
        // shimmer. Mirrors Python `shimmer.py`'s `already_coerced`.
        let mut already_coerced: HashSet<(String, u32, TclType)> = HashSet::new();
        for ss in &ssa_block.statements {
            check_statement(
                &ss.statement,
                &ss.uses,
                types,
                registry,
                in_loop,
                &def_map,
                values,
                &mut already_coerced,
                &mut out,
            );
        }
    }

    out
}

/// True when `value` is a SCCP CONST string whose text is a hex /
/// octal / binary integer literal (`0xff`, `0o15`, `0b1010`, optionally
/// signed). These spellings classify as `String` by `literal_type` (the
/// canonical stringified intrep differs from the source text) but
/// promote cleanly to `Int` at the first arithmetic op — not a real
/// shimmer. Mirrors Python's `_value_is_int_literal_string`.
fn value_is_int_literal_string(value: Option<&LatticeValue>) -> bool {
    let Some(LatticeValue::Const(ConstValue::String(text))) = value else {
        return false;
    };
    let sign_stripped = text.strip_prefix(['+', '-']).unwrap_or(text);
    let bytes = sign_stripped.as_bytes();
    // Need at least a `0x`-style prefix plus one body digit.
    if bytes.len() < 3 || bytes[0] != b'0' {
        return false;
    }
    let body = &sign_stripped[2..];
    match bytes[1] {
        b'x' | b'X' => body.bytes().all(|c| c.is_ascii_hexdigit()),
        b'o' | b'O' => body.bytes().all(|c| (b'0'..=b'7').contains(&c)),
        b'b' | b'B' => body.bytes().all(|c| c == b'0' || c == b'1'),
        _ => false,
    }
}

/// Check one command invocation's arguments for an intrep mismatch
/// against the variables' known types. Used for both a top-level
/// [`Statement::Call`] and a `[cmd …]` substitution lifted out of a
/// [`Statement::AssignValue`] value (`set b [lindex $x 0]`), mirroring
/// Python `shimmer.py`'s `_check_args_for_shimmer` dispatch over
/// `IRCall` and `IRAssignValue`.
#[allow(clippy::too_many_arguments)]
fn check_invocation(
    command: &str,
    args: &[String],
    span: Span,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    registry: &CommandRegistry,
    in_loop: bool,
    def_map: &HashMap<ValueKey, Span>,
    already_coerced: &mut HashSet<(String, u32, TclType)>,
    out: &mut Vec<ShimmerWarning>,
) {
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    for (i, word) in args.iter().enumerate() {
        let Some(expected) = arg_shimmer_type(registry, command, &arg_refs, i) else {
            continue;
        };
        // Only flag pure variable references — complex words may produce
        // the right type via their own evaluation.
        let stripped = word.trim();
        if !is_pure_var_ref(stripped) {
            continue;
        }
        let var = normalise_var_name(stripped).to_owned();
        let Some(&ver) = uses.get(&var) else { continue };
        if ver == 0 {
            continue;
        }
        let lattice = types
            .get(&(var.clone(), ver))
            .cloned()
            .unwrap_or_else(TypeLattice::unknown);
        if lattice.kind != TypeKind::Known {
            continue;
        }
        let Some(current) = lattice.tcl_type else {
            continue;
        };
        if current == expected || is_numeric_compatible(current, expected) {
            continue;
        }
        // A prior use in this block already coerced `(var, ver)` to this
        // intrep — the runtime representation has already changed, so this
        // is not a second shimmer.
        let coercion_key = (var.clone(), ver, expected);
        if !already_coerced.insert(coercion_key) {
            continue;
        }
        let related: Vec<(Span, String)> = def_map
            .get(&(var.clone(), ver))
            .map(|&sp| vec![(sp, "value defined here".to_owned())])
            .unwrap_or_default();
        let code = if in_loop { "S101" } else { "S100" }.to_owned();
        out.push(ShimmerWarning {
            span,
            variable: var.clone(),
            from_type: current,
            to_type: expected,
            command: command.to_owned(),
            in_loop,
            code: code.clone(),
            message: format!(
                "{code}: variable '{var}' has {from} intrep \
                 but '{cmd}' expects {to} at arg {i}",
                from = type_name(current),
                cmd = command,
                to = type_name(expected),
            ),
            related,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn check_statement(
    stmt: &Statement,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    registry: &CommandRegistry,
    in_loop: bool,
    def_map: &HashMap<ValueKey, Span>,
    values: &HashMap<ValueKey, LatticeValue>,
    already_coerced: &mut HashSet<(String, u32, TclType)>,
    out: &mut Vec<ShimmerWarning>,
) {
    match stmt {
        Statement::Call { command, args, .. } => {
            check_invocation(
                command,
                args,
                stmt.span(),
                uses,
                types,
                registry,
                in_loop,
                def_map,
                already_coerced,
                out,
            );
        }

        // A command substitution lifted into an assignment value
        // (`set b [lindex $x 0]`) reads its arguments just like a direct
        // call. Mirrors Python `shimmer.py`'s `IRAssignValue` arm, which
        // parses the value and runs the same arg-shimmer check.
        Statement::AssignValue { value, .. } => {
            if let Some((command, args)) =
                crate::value_shapes::parse_command_substitution(value.trim())
            {
                check_invocation(
                    &command,
                    &args,
                    stmt.span(),
                    uses,
                    types,
                    registry,
                    in_loop,
                    def_map,
                    already_coerced,
                    out,
                );
            }
        }

        Statement::Incr { name, .. } => {
            // `incr` always reads its variable as Int.
            // Handled as a special IR node (not a Call) — check explicitly.
            let var = normalise_var_name(name).to_owned();
            let Some(&ver) = uses.get(&var) else { return };
            if ver == 0 {
                return;
            }
            let lattice = types
                .get(&(var.clone(), ver))
                .cloned()
                .unwrap_or_else(TypeLattice::unknown);
            if lattice.kind != TypeKind::Known {
                return;
            }
            let Some(current) = lattice.tcl_type else {
                return;
            };
            if is_numeric_compatible(current, TclType::Int) {
                return;
            }
            // A hex / octal / binary literal string (`0x80`, `0b1010`)
            // classifies as String but promotes cleanly to Int at the
            // `incr` — not a real shimmer (SYNC-MAY31-1e; avoids the
            // spurious S101 on the tcllib `variable n 0x80; … incr n`
            // idiom).
            if value_is_int_literal_string(values.get(&(var.clone(), ver))) {
                return;
            }
            let related: Vec<(Span, String)> = def_map
                .get(&(var.clone(), ver))
                .map(|&sp| vec![(sp, "value defined here".to_owned())])
                .unwrap_or_default();
            let code = if in_loop { "S101" } else { "S100" }.to_owned();
            out.push(ShimmerWarning {
                span: stmt.span(),
                variable: var.clone(),
                from_type: current,
                to_type: TclType::Int,
                command: "incr".to_owned(),
                in_loop,
                code: code.clone(),
                message: format!(
                    "{code}: variable '{var}' has {from} intrep but 'incr' expects int",
                    from = type_name(current),
                ),
                related,
            });
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// A String variable passed to `incr` triggers S100.
    #[test]
    fn shimmer_detected_for_string_used_with_incr() {
        let cu = CompilationUnit::build_for("set x \"hello\"\nincr x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings
            .iter()
            .find(|w| w.command == "incr" && w.from_type == TclType::String);
        assert!(
            w.is_some(),
            "expected incr/String shimmer, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().to_type, TclType::Int);
    }

    /// SYNC-MAY31-1e: a hex literal string (`0x80`) classifies as
    /// String but promotes cleanly to Int at `incr` — no shimmer.
    #[test]
    fn no_shimmer_for_hex_literal_string_used_with_incr() {
        let cu = CompilationUnit::build_for("set n 0x80\nincr n", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
        assert!(
            incr_shimmers.is_empty(),
            "0x80 promotes cleanly to int — no incr shimmer expected: {incr_shimmers:?}",
        );
    }

    /// An Int variable passed to `incr` has no shimmer.
    #[test]
    fn no_shimmer_for_int_used_with_incr() {
        let cu = CompilationUnit::build_for("set x 5\nincr x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let incr_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "incr").collect();
        assert!(
            incr_shimmers.is_empty(),
            "unexpected incr shimmer for Int: {incr_shimmers:?}"
        );
    }

    /// An Int variable passed to `lindex` should trigger a shimmer
    /// (Int → List at arg 0).
    #[test]
    fn shimmer_detected_for_int_used_with_lindex() {
        let cu = CompilationUnit::build_for("set x 5\nlindex $x 0", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings.iter().find(|w| w.command == "lindex");
        assert!(
            w.is_some(),
            "expected lindex shimmer for Int var, got: {warnings:?}"
        );
        assert_eq!(w.unwrap().from_type, TclType::Int);
        assert_eq!(w.unwrap().to_type, TclType::List);
    }

    /// A `foreach` element variable is typed String (list elements
    /// stringify); using it as a list intrep inside the loop body via a
    /// `[lindex $x 0]` command substitution re-thunks each iteration —
    /// per-iteration shimmer (S101). The substitution lives in an
    /// `AssignValue` value, so this exercises the `AssignValue` arm.
    #[test]
    fn shimmer_detected_for_foreach_var_used_as_list_in_cmd_sub() {
        let src = "proc f {l} {\n    foreach x $l {\n        set b [lindex $x 0]\n    }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::f").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        let w = warnings.iter().find(|w| w.command == "lindex");
        assert!(
            w.is_some(),
            "expected lindex S101 for foreach var, got: {warnings:?}"
        );
        let w = w.unwrap();
        assert_eq!(w.code, "S101");
        assert_eq!(w.from_type, TclType::String);
        assert_eq!(w.to_type, TclType::List);
    }

    /// Variables with Unknown type do not produce false-positive shimmers.
    #[test]
    fn no_shimmer_for_unknown_type() {
        // `set x $other` — type of x is Unknown (other has no known type).
        let cu = CompilationUnit::build_for("set x $other\nlindex $x 0", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let warnings = find_use_site_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &registry(),
            &fu.sccp.values,
        );
        // x has Unknown type; should not produce a shimmer.
        let lindex_shimmers: Vec<_> = warnings.iter().filter(|w| w.command == "lindex").collect();
        assert!(
            lindex_shimmers.is_empty(),
            "unexpected shimmer for Unknown type: {lindex_shimmers:?}"
        );
    }
}
