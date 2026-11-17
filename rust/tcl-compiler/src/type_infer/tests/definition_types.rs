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

//! How single statements type their definitions: `incr`, destructuring writers, `catch`, value assignments and nested command results.

use super::*;

/// Evaluate one statement's def typing against empty context pieces and
/// return the lattice a def named `def_name` receives (`DefTyping::PerDef`
/// defs absent from the map widen to `Overdefined`, matching the
/// propagation pass).
fn eval_def(
    stmt: &Statement,
    registry: &CommandRegistry,
    ssa: &SsaFunction,
    def_name: &str,
) -> TypeLattice {
    let uses = HashMap::new();
    let types = HashMap::new();
    let values = HashMap::new();
    let known_classes: HashSet<String> = HashSet::new();
    let ctx = WordTypingCtx {
        preparations: &[],
        math: &[],
        tokens: stmt.tokens(),
        source: crate::ssa::SsaSourceView::unpositioned(ssa),
        uses: &uses,
        defs: None,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
        config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
        types: &types,
        values: &values,
        registry,
        known_classes: &known_classes,
        namespace: "::",
        ssa,
        numbers: numbers_of(registry),
        active_writes: &[],
        normal_results: None,
    };
    match evaluate_type_def(stmt, &ctx) {
        DefTyping::Uniform(t) => t,
        DefTyping::PerDef(map) => map
            .get(def_name)
            .cloned()
            .unwrap_or_else(TypeLattice::overdefined),
    }
}

#[test]
fn incr_infers_int() {
    let stmt = Statement::Incr {
        span: Span::new(0, 0),
        name: "n".to_owned(),
        name_braced: false,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::of(TclType::Int));
}

#[test]
fn lassign_destructure_defs_are_overdefined_not_command_return_type() {
    // TP: `lassign $pipe a b` writes destructured list *elements*;
    // `lassign`'s own `return_type` (List — the *leftover* elements) must
    // not be broadcast onto the targets. Pre-fix, both `a` and `b` were
    // typed LIST, so a later channel-position use (`puts $a ...`) would
    // falsely fire W126 ("has type LIST, not CHANNEL") — see FP-STY-04.
    // The typing now comes from the registry's `VarWriteTyping` for
    // `lassign` (`Destructured`), not a def-count heuristic.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "lassign".to_owned(),
        canonical_command: None,
        args: vec!["$pipe".to_owned(), "a".to_owned(), "b".to_owned()],
        defs: vec!["a".to_owned(), "b".to_owned()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::overdefined());
}

#[test]
fn lassign_single_destructure_def_is_overdefined_not_list() {
    // A *single*-target `lassign $l x` is the case a `defs.len() > 1`
    // heuristic misses — one write falls
    // through to `lassign`'s `List` return type, so `x` is typed LIST and
    // `expr {$x + 1}` fired a bogus S100. The registry's `Destructured`
    // typing widens it to OVERDEFINED regardless of target count.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "lassign".to_owned(),
        canonical_command: None,
        args: vec!["$point".to_owned(), "x".to_owned()],
        defs: vec!["x".to_owned()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::overdefined());
}

#[test]
fn single_def_command_still_uses_its_declared_return_type() {
    // TN control: a command that writes exactly one variable (`append`)
    // legitimately shares its return value with that variable — its
    // `VarWriteTyping` is the default `ReturnValue`, so it must keep taking
    // the declared return type (`String`), not widen to OVERDEFINED.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "append".to_owned(),
        canonical_command: None,
        args: vec!["result".to_owned(), "x".to_owned()],
        defs: vec!["result".to_owned()],
        reads: Vec::new(),
        reads_own_defs: true,
        safe_on_uninit: true,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::of(TclType::String));
}

#[test]
fn unannotated_multi_def_call_stays_overdefined_not_return_type() {
    // A call that writes SEVERAL variables under the default `ReturnValue`
    // typing must not broadcast its return type onto all of them: a `try`
    // returns what its body returns, not the message and options its
    // handlers bind, and typing `msg`/`result`/`opts` alike would wrongly
    // fire S100/W126. The default arm's multi-def guard keeps them
    // OVERDEFINED.
    let stmt = Statement::Call {
        span: Span::new(0, 0),
        command: "try".to_owned(),
        canonical_command: None,
        args: vec![
            "{set msg hello}".to_owned(),
            "on".to_owned(),
            "error".to_owned(),
            "{result opts}".to_owned(),
            "{}".to_owned(),
        ],
        defs: vec!["msg".to_owned(), "result".to_owned(), "opts".to_owned()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
    let t = eval_def(&stmt, &registry(), &ssa, "__def__");
    assert_eq!(t, TypeLattice::overdefined());
}

/// The variables `catch` writes hold the script's result and its options
/// dictionary, not the integer completion code it returns: its registry
/// entry declares them destructured, so a call that writes only the result
/// variable types it as nothing known either.
#[test]
fn a_catch_result_variable_is_overdefined_not_the_completion_code() {
    for (args, defs) in [
        (vec!["{foo}", "msg"], vec!["msg"]),
        (vec!["{foo}", "msg", "opts"], vec!["msg", "opts"]),
    ] {
        let stmt = Statement::Call {
            span: Span::new(0, 0),
            command: "catch".to_owned(),
            canonical_command: None,
            args: args.into_iter().map(str::to_owned).collect(),
            defs: defs.into_iter().map(str::to_owned).collect(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let t = eval_def(&stmt, &registry(), &ssa, "__def__");
        assert_eq!(t, TypeLattice::overdefined(), "{stmt:?}");
    }
}

/// End-to-end lattice checks for the registry-driven `VarWriteTyping`:
/// each destructuring writer types its side-effect target correctly,
/// distinct from its return type.
#[test]
fn var_write_typing_shapes_destructure_target_types() {
    use crate::compilation_unit::CompilationUnit;

    // Helper: does any version of `name` carry a KNOWN type `t`?
    fn any_known(fu: &crate::compilation_unit::FunctionUnit, name: &str, t: TclType) -> bool {
        fu.types.iter().any(|((sym, _), lat)| {
            fu.ssa.var_name(*sym) == name
                && lat.kind() == TypeKind::Known
                && lat.tcl_type() == Some(t)
        })
    }
    // Helper: is every version of `name` non-Known (OVERDEFINED/UNKNOWN)?
    fn none_known(fu: &crate::compilation_unit::FunctionUnit, name: &str) -> bool {
        fu.types
            .iter()
            .filter(|((sym, _), _)| fu.ssa.var_name(*sym) == name)
            .all(|(_, lat)| lat.kind() != TypeKind::Known)
    }

    // `lassign` element target — OVERDEFINED, never List.
    let cu = CompilationUnit::build_for("set p [list 1 2 3]\nlassign $p x", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        none_known(fu, "x"),
        "lassign target must not carry a Known type: {:?}",
        fu.types
    );

    // `regexp` capture — OVERDEFINED, never Int (the match count).
    let cu = CompilationUnit::build_for("regexp {(.)} $s c", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(none_known(fu, "c"), "regexp capture must not be Known Int");
    // Over exact operands the route writes the capture: the
    // String it matched, still never the count.
    let cu = CompilationUnit::build_for("regexp {(.)} abc c", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "c", TclType::String) && !any_known(fu, "c", TclType::Int),
        "a written regexp capture is the String it matched: {:?}",
        fu.types
    );

    // `scan` target — OVERDEFINED (format-dependent), never Int.
    let cu = CompilationUnit::build_for("scan $s %s word", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(none_known(fu, "word"), "scan target must not be Known Int");
    // Over exact operands the route writes the conversion: the
    // String `%s` built, still never the count.
    let cu = CompilationUnit::build_for("scan hello %s word", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "word", TclType::String) && !any_known(fu, "word", TclType::Int),
        "a written scan target is the String its conversion built: {:?}",
        fu.types
    );

    // `binary scan` target (subcommand-level typing) — OVERDEFINED.
    let cu = CompilationUnit::build_for("binary scan $d a3 chars", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        none_known(fu, "chars"),
        "binary scan target must not be Known Int"
    );

    // `gets chan line` — Fixed(String): the target is the read line, a
    // String, not the character count the two-arg form returns.
    let cu = CompilationUnit::build_for("set ch stdin; gets $ch ::line", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "::line", TclType::String),
        "gets target must be Known String: {:?}; escaping {:?}; operations {:?}",
        fu.types,
        crate::var_observability::analyse_var_observability(&fu.cfg, &registry())
            .escaping_var_names(),
        fu.ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .map(|statement| (
                statement
                    .defs
                    .keys()
                    .map(|symbol| fu.ssa.var_name(*symbol))
                    .collect::<Vec<_>>(),
                &statement.may_defs,
                crate::registry_invocation::resolved_statement_invocation(
                    &registry(),
                    None,
                    &statement.statement,
                )
                .map(|invocation| invocation.facts.operation),
            ))
            .collect::<Vec<_>>()
    );

    // `lpop listVar` — Fixed(List): the variable is left holding the
    // shortened list, not the popped element (String) it returns.
    let cu = CompilationUnit::build_for("set l [list 1 2 3]\nlpop l", &registry(), false);
    let fu = cu.function("::top").unwrap();
    assert!(
        any_known(fu, "l", TclType::List),
        "lpop target must be Known List, not the element return type: {:?}",
        fu.types
    );
}

/// `AssignValue` with a pure variable reference inherits the source type.
#[test]
fn assign_value_pure_var_ref_inherits_type() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for("set x 42\nset y $x", &registry(), false);
    let fu = cu.function("::top").unwrap();
    // x should be Int; y (which copies x) should also be Int.
    let x_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "x" && t.tcl_type() == Some(TclType::Int));
    let y_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "y" && t.tcl_type() == Some(TclType::Int));
    assert!(x_is_int, "expected x to be Int");
    assert!(y_is_int, "expected y to inherit Int type from x");
}

/// An aliased `set` (`interp alias {} myset {} set`) keeps its runtime
/// `Call` shape, but its single def takes the *value word's* intrep — the
/// canonical-command value-passthrough. `myset x 5` types x as Int, exactly
/// as `set x 5` would, rather than an opaque OVERDEFINED `Call`.
#[test]
fn aliased_set_call_types_def_from_value() {
    use crate::compilation_unit::CompilationUnit;
    let cu = CompilationUnit::build_for(
        "interp alias {} myset {} set\nproc ::g {} { myset x 5\n return $x }",
        &registry(),
        false,
    );
    let fu = cu.function("::g").unwrap();
    let x_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "x" && t.tcl_type() == Some(TclType::Int));
    assert!(
        x_is_int,
        "aliased `myset x 5` should type x as Int (value passthrough): {:?}",
        fu.types
            .iter()
            .map(|((n, v), t)| (fu.ssa.var_name(*n), *v, t.to_string()))
            .collect::<Vec<_>>()
    );
}

/// `AssignValue` with a command substitution uses the command's return type.
#[test]
fn assign_value_command_sub_uses_return_type() {
    use crate::compilation_unit::CompilationUnit;
    // `llength` returns Int per the registry.
    let cu =
        CompilationUnit::build_for("set lst {a b c}\nset n [llength $lst]", &registry(), false);
    let fu = cu.function("::top").unwrap();
    let n_is_int = fu
        .types
        .iter()
        .any(|((name, _), t)| fu.ssa.var_name(*name) == "n" && t.tcl_type() == Some(TclType::Int));
    assert!(n_is_int, "expected n to be Int (llength return type)");
}

#[test]
fn normal_result_representation_does_not_require_an_opcode() {
    use crate::compilation_unit::CompilationUnit;
    let registry =
        CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules());
    let source = "when HTTP_REQUEST {set input {a b}; set size [llength $input]}";
    let cu = CompilationUnit::build_for_profile(
        source,
        &registry,
        false,
        tcl_dialect::DialectProfile::irules(),
    );
    let function = cu.function("::when::HTTP_REQUEST").expect("handler");
    assert!(
        function.types.iter().any(|((symbol, _), value)| {
            function.ssa.var_name(*symbol) == "size" && value.tcl_type() == Some(TclType::Int)
        }),
        "successful producer: {:?}",
        function.types
    );
    let shadow = CompilationUnit::build_for_profile(
        "proc llength {args} {return arbitrary}; when HTTP_REQUEST {set input {a b}; set size [llength $input]}",
        &registry,
        false,
        tcl_dialect::DialectProfile::irules(),
    );
    let shadow = shadow.function("::when::HTTP_REQUEST").expect("handler");
    assert!(
        !shadow.types.iter().any(|((symbol, _), value)| {
            shadow.ssa.var_name(*symbol) == "size" && value.tcl_type() == Some(TclType::Int)
        }),
        "custom producer: {:?}",
        shadow.types
    );
}
