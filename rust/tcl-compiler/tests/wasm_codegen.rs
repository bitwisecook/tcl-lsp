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

//! WASM backend tests: the greenfield eval-fallback emitter produces a
//! structurally valid module with the right shape (imports, data section,
//! eval-fallback calls, structured `if` / loops). Execution against a runtime is
//! currently (the Rust runtime's wasm32 ABI is still a stub), so these
//! assert the emitted structure; `wasmtime compile` (run below where the CLI is
//! present) validates the bytes for full structural validity.

use tcl_compiler::codegen::wasm::{
    RESERVED_DATA_BASE, SemanticOptimisationConfig, SemanticOptimisationPassId, WasmCompileOptions,
    WasmModule, compile_wasm as compile_wasm_unit,
};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// Lower Tcl source and run the greenfield WASM backend over its top level.
fn compile_wasm(source: &str) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(source, &registry, false);
    compile_wasm_unit(
        &unit,
        &registry,
        WasmCompileOptions::hosted()
            .for_eval_only_test_host()
            .with_data_base(0),
    )
    .into_module()
}

/// As [`compile_wasm`], but relocate the constant pool to `data_base`.
fn compile_wasm_based(source: &str, data_base: i64) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(source, &registry, false);
    compile_wasm_unit(
        &unit,
        &registry,
        WasmCompileOptions::hosted()
            .for_eval_only_test_host()
            .with_data_base(data_base),
    )
    .into_module()
}

fn compile_wasm_analysed(source: &str) -> WasmModule {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(source, &registry, false);
    compile_wasm_unit(
        &unit,
        &registry,
        WasmCompileOptions::hosted().with_semantic_optimisations(
            SemanticOptimisationConfig::new()
                .with_enabled(SemanticOptimisationPassId::LegacyAnalysisSpecialisation),
        ),
    )
    .into_module()
}

#[test]
fn default_compilation_keeps_legacy_analysis_specialisations_disabled() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for("set x 1\n", &registry, false);
    let mut module =
        compile_wasm_unit(&unit, &registry, WasmCompileOptions::hosted()).into_module();
    let wat = module.to_wat();

    assert!(wat.contains(r#""tcl_eval_code""#), "{wat}");
    assert!(!wat.contains(r#""tcl_codegen_var_set""#), "{wat}");
}

#[test]
fn analysis_specialisation_requires_its_explicit_pass() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for("set x 1\n", &registry, false);
    let mut module = compile_wasm_unit(
        &unit,
        &registry,
        WasmCompileOptions::hosted()
            .with_semantic_optimisation(SemanticOptimisationPassId::LegacyAnalysisSpecialisation),
    )
    .into_module();
    let wat = module.to_wat();

    assert!(wat.contains(r#""tcl_codegen_var_set""#), "{wat}");
}

/// Import function lines in declaration order (the WASM function-index order).
fn import_lines(wat: &str) -> Vec<&str> {
    wat.lines()
        .filter(|line| line.contains("(func $imp_"))
        .collect()
}

/// How many `call` instructions target the host import `name`.
fn import_calls(wat: &str, name: &str) -> usize {
    let Some(index) = import_lines(wat)
        .iter()
        .position(|line| line.contains(&format!("\"{name}\"")))
    else {
        return 0;
    };
    let target = format!("call {index}");
    wat.lines().filter(|line| line.trim() == target).count()
}

/// How many `call` instructions target the module's own function `name`.
///
/// Defined functions follow every import, in emission order, so the call index
/// is the import count plus the function's position among the definitions.
fn function_calls(wat: &str, name: &str) -> usize {
    let imports = import_lines(wat).len();
    let needle = format!("(func ${name} ");
    let Some(position) = wat
        .lines()
        .filter(|line| line.contains("(func $") && !line.contains("(func $imp_"))
        .position(|line| line.contains(&needle))
    else {
        return 0;
    };
    let target = format!("call {}", imports + position);
    wat.lines().filter(|line| line.trim() == target).count()
}

/// How many `tcl_eval_code` source-span fallbacks the module still performs.
fn eval_fallbacks(wat: &str) -> usize {
    import_calls(wat, "tcl_eval_code")
}

#[test]
fn analysed_add_program_uses_direct_tcl_object_operations() {
    let source = r"proc add {b c} {
    return [expr {$b + $c}]
}

set e 2
set f 4
puts [add $e $f]
";
    let mut module = compile_wasm_analysed(source);
    let wat = module.to_wat();

    assert!(
        wat.contains(r#"(func $::add (export "::add") (param $b i32) (param $c i32) (result i32)"#),
        "{wat}"
    );
    assert!(wat.contains(r#""tcl_codegen_local_bind""#), "{wat}");
    assert!(wat.contains(r#""tcl_codegen_expr_add""#), "{wat}");
    assert!(wat.contains(r#""tcl_codegen_puts""#), "{wat}");
    assert!(wat.contains(r#""tcl_codegen_proc_register""#), "{wat}");
    assert_eq!(eval_fallbacks(&wat), 0, "{wat}");
    // Every statement has a proven direct form, so nothing needs the generic
    // prebuilt-argv path either.
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 0, "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// A rebound command must never become a direct call to the procedure's own
/// generated function. It stays on runtime dispatch — which the general tier
/// now reaches through a compiled argv rather than by re-evaluating source.
#[test]
fn analysed_direct_call_declines_when_binding_lattice_is_opaque() {
    let source = r"proc add {b c} {return [expr {$b + $c}]}
rename add old_add
puts [add 2 4]
";
    let mut module = compile_wasm_analysed(source);
    let wat = module.to_wat();

    assert_eq!(
        function_calls(&wat, "::add"),
        0,
        "the rebound call must not bind to the generated procedure:\n{wat}"
    );
    assert_eq!(
        import_calls(&wat, "tcl_codegen_puts"),
        0,
        "the rebound call must not take the direct channel-write path:\n{wat}"
    );
    // `rename …`, `puts …`, and the nested `[add 2 4]` all dispatch through the
    // runtime's own command resolution.
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 3, "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn analysed_assignment_preserves_dynamic_array_index_substitution() {
    let mut module = compile_wasm_analysed("set x foo\nset a($x) 1\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""set a($x) 1""#),
        "the substituted target must retain source evaluation:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn analysed_assignment_preserves_a_rebound_set_command() {
    let source = r"proc replacement args {return replacement}
proc rebind {} {
    rename set original_set
    rename replacement set
}
rebind
set x 1
";
    let mut module = compile_wasm_analysed(source);
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""set x 1""#),
        "a potentially rebound set must retain source evaluation:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn analysed_direct_proc_requires_original_expr_and_return_bindings() {
    for (name, rebind) in [
        ("expr", "rename expr original_expr\nrename replacement expr"),
        (
            "return",
            "rename return original_return\nrename replacement return",
        ),
    ] {
        let source = format!(
            "proc replacement args {{list replacement}}\n\
             proc rebind {{}} {{{rebind}}}\n\
             proc add {{a b}} {{return [expr {{$a + $b}}]}}\n\
             rebind\n\
             puts [add 2 4]\n"
        );
        let mut module = compile_wasm_analysed(&source);
        let wat = module.to_wat();

        assert!(
            !wat.contains(
                r#"(func $::add (export "::add") (param $a i32) (param $b i32) (result i32)"#
            ),
            "a potentially rebound {name} must disable the direct procedure:\n{wat}"
        );
        assert_eq!(
            function_calls(&wat, "::add"),
            0,
            "the call must not bind to a generated procedure when {name} can change:\n{wat}"
        );
        assert!(
            import_calls(&wat, "tcl_invoke_argv") > 0,
            "the call must reach runtime dispatch when {name} can change:\n{wat}"
        );
        assert_eq!(&module.to_bytes()[0..4], b"\0asm");
    }
}

#[test]
fn analysed_direct_call_preserves_static_and_dynamic_execution_traces() {
    for (name, trace_setup) in [
        ("static", "trace add execution add enter callback"),
        (
            "dynamic",
            "set traced_command add\ntrace add execution $traced_command enter callback",
        ),
    ] {
        let source = format!(
            "proc callback args {{}}\n\
             proc add {{a b}} {{return [expr {{$a + $b}}]}}\n\
             {trace_setup}\n\
             puts [add 1 2]\n"
        );
        let mut module = compile_wasm_analysed(&source);
        let wat = module.to_wat();

        assert_eq!(
            function_calls(&wat, "::add"),
            0,
            "a {name} execution trace must not bind to the generated procedure:\n{wat}"
        );
        assert!(
            import_calls(&wat, "tcl_invoke_argv") > 0,
            "a {name} execution trace must retain runtime dispatch:\n{wat}"
        );
        assert_eq!(&module.to_bytes()[0..4], b"\0asm");
    }
}

/// Issue #1376 — a clause word's token span already *excludes* its closing
/// brace, so stripping a trailing `}` off the slice could only ever delete a
/// byte of real content. Every clause whose last inner character is `}` was
/// truncated: `${name}`, a trailing braced word, a nested dict/list literal.
/// The truncated text then reaches `tcl_expr_bool` / `tcl_eval_code`, which
/// raises an unbalanced-brace parse error on code the user wrote correctly.
///
/// Each vector below asserts the *exact* interned clause text, so a
/// reintroduced strip (or an over-eager one, e.g. dropping the opener twice)
/// fails by name rather than by a downstream runtime error.
#[test]
fn clause_text_keeps_a_trailing_brace_in_a_while_condition() {
    let mut module = compile_wasm("while {${x}} {puts hi}\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""${x}""#),
        "the `while` condition must be interned whole, not truncated to `${{x`:\n{wat}"
    );
    assert!(
        !wat.contains(r#""${x""#),
        "the truncated form must not appear at all:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

#[test]
fn clause_text_keeps_a_trailing_brace_in_an_if_condition() {
    let mut module = compile_wasm("if {$x eq {a}} {puts a}\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""$x eq {a}""#),
        "the `if` condition must keep its nested braced word:\n{wat}"
    );
    assert!(
        !wat.contains(r#""$x eq {a""#),
        "the truncated form must not appear at all:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// The `for`-*next* clause is the residual half of #1376: `Statement::For`
/// carries `condition_base` but no `next_base`, so this site is driven from
/// the lowerer's `raw_args[2]` instead.
#[test]
fn clause_text_keeps_a_trailing_brace_in_a_for_step() {
    let mut module = compile_wasm("for {set i 0} {$i<2} {set x ${i}} {puts a}\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""set x ${i}""#),
        "the `for` step clause must be interned whole:\n{wat}"
    );
    assert!(
        !wat.contains(r#""set x ${i""#),
        "the truncated form must not appear at all:\n{wat}"
    );
    // The init and condition clauses stay intact alongside it.
    assert!(wat.contains(r#""set i 0""#), "{wat}");
    assert!(wat.contains(r#""$i<2""#), "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// Control: a condition whose last inner character is `]`, not `}`, was never
/// affected — it must stay byte-identical, proving the fix did not start
/// mangling the unaffected majority.
#[test]
fn clause_text_leaves_a_bracket_terminated_condition_alone() {
    let mut module = compile_wasm("while {[string match {a} $x]} {puts hi}\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""[string match {a} $x]""#),
        "an unaffected condition must round-trip byte-for-byte:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// A nested *dict/list* literal is the third shape #1376 named. Distinct from
/// the `${x}` and trailing-braced-word vectors above because the trailing `}`
/// closes a word the condition itself opened.
#[test]
fn clause_text_keeps_a_trailing_brace_from_a_nested_list_literal() {
    let mut module = compile_wasm("while {[llength {a b}]} {puts hi}\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""[llength {a b}]""#),
        "the nested list literal must keep its closing brace:\n{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// A braced word is one unsubstituted literal argv value: it is materialised
/// straight from the constant pool, never evaluated as a command.
#[test]
fn analysed_puts_preserves_braced_command_text_as_literal() {
    let mut module = compile_wasm_analysed("puts {[add 2 4]}\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#"(data (i32.const 1048580) "[add 2 4]")"#),
        "a braced command-shaped word must be interned as its literal value:\n{wat}"
    );
    assert_eq!(function_calls(&wat, "::add"), 0, "{wat}");
    // One invocation only: the braced word is data, not a nested command.
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 1, "{wat}");
    assert_eq!(import_calls(&wat, "tcl_obj_new_string_owned"), 2, "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// The general tier's normal leaf-command path: one transient call frame, one
/// owned word per argv slot, one `tcl_invoke_argv`, and no source-span eval.
#[test]
fn analysed_leaf_command_compiles_to_a_prebuilt_argv() {
    let mut module = compile_wasm_analysed("string tolower ABC\n");
    let wat = module.to_wat();

    assert_eq!(eval_fallbacks(&wat), 0, "{wat}");
    assert_eq!(
        import_calls(&wat, "tcl_codegen_call_frame_alloc"),
        1,
        "{wat}"
    );
    assert_eq!(
        import_calls(&wat, "tcl_codegen_call_frame_free"),
        1,
        "{wat}"
    );
    assert_eq!(import_calls(&wat, "tcl_obj_new_string_owned"), 3, "{wat}");
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 1, "{wat}");
    // Three argv words plus the completion's owned result and options.
    assert_eq!(import_calls(&wat, "tcl_obj_release"), 5, "{wat}");
    // Three argv pointers and two adopted handles, then one completion triple.
    assert!(wat.contains("i32.const 20\n"), "{wat}");
    assert!(wat.contains("i32.const 32\n"), "{wat}");
    for word in ["string", "tolower", "ABC"] {
        assert!(wat.contains(&format!("\"{word}\"")), "{wat}");
    }
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// Every owned value a statement creates is released on the one cleanup path,
/// and the release count scales with the words the statement evaluates.
#[test]
fn analysed_leaf_command_releases_every_word_it_creates() {
    for (source, words) in [("list a\n", 2), ("list a b\n", 3), ("list a b c d\n", 5)] {
        let mut module = compile_wasm_analysed(source);
        let wat = module.to_wat();
        assert_eq!(
            import_calls(&wat, "tcl_obj_release"),
            words + 2,
            "{source}:\n{wat}"
        );
        assert_eq!(
            import_calls(&wat, "tcl_codegen_call_frame_free"),
            1,
            "{wat}"
        );
    }
}

/// A `$var` word is a compiled variable read, not a re-evaluated source span.
#[test]
fn analysed_leaf_command_compiles_a_scalar_variable_word() {
    let mut module = compile_wasm_analysed("string length $value\n");
    let wat = module.to_wat();

    assert_eq!(eval_fallbacks(&wat), 0, "{wat}");
    assert_eq!(import_calls(&wat, "tcl_codegen_var_get"), 1, "{wat}");
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 1, "{wat}");
    assert!(wat.contains(r#""value""#), "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// `$a(b)` is an array-element read; `${a(b)}` names a scalar. The two have the
/// same compatibility spelling, so the emitted read proves they stayed apart.
#[test]
fn analysed_leaf_command_separates_array_and_scalar_variable_spellings() {
    let mut element = compile_wasm_analysed("string length $a(b)\n");
    let element = element.to_wat();
    assert_eq!(
        import_calls(&element, "tcl_codegen_var_get_element"),
        1,
        "{element}"
    );
    assert_eq!(
        import_calls(&element, "tcl_codegen_var_get"),
        0,
        "{element}"
    );

    let mut scalar = compile_wasm_analysed("string length ${a(b)}\n");
    let scalar = scalar.to_wat();
    assert_eq!(
        import_calls(&scalar, "tcl_codegen_var_get_element"),
        0,
        "{scalar}"
    );
    assert_eq!(import_calls(&scalar, "tcl_codegen_var_get"), 1, "{scalar}");
}

/// A quoted or compound word is evaluated part by part and joined by the
/// runtime, never by string work in the emitter.
#[test]
fn analysed_leaf_command_compiles_a_compound_word() {
    let mut module = compile_wasm_analysed("string length \"hi $name!\"\n");
    let wat = module.to_wat();

    assert_eq!(eval_fallbacks(&wat), 0, "{wat}");
    assert_eq!(import_calls(&wat, "tcl_codegen_word_concat"), 1, "{wat}");
    assert_eq!(import_calls(&wat, "tcl_codegen_var_get"), 1, "{wat}");
    assert!(wat.contains(r#""hi ""#), "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// A `[nested command]` word is the same compiled argv invocation, with the
/// completion's owned result becoming the enclosing word's value.
#[test]
fn analysed_leaf_command_compiles_a_nested_command_word() {
    let mut module = compile_wasm_analysed("string length [string tolower ABC]\n");
    let wat = module.to_wat();

    assert_eq!(eval_fallbacks(&wat), 0, "{wat}");
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 2, "{wat}");
    // One frame carries both invocations' argv and completion storage.
    assert_eq!(
        import_calls(&wat, "tcl_codegen_call_frame_alloc"),
        1,
        "{wat}"
    );
    assert_eq!(
        import_calls(&wat, "tcl_codegen_call_frame_free"),
        1,
        "{wat}"
    );
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// A word shape the emitter cannot prove keeps the whole statement on the
/// source-span eval fallback — the truncate-and-retry path, not a half-built
/// argv.
#[test]
fn analysed_leaf_command_declines_argument_expansion() {
    let mut module = compile_wasm_analysed("set args {a b}\nstring length {*}$args\n");
    let wat = module.to_wat();

    assert!(
        wat.contains(r#""string length {*}$args""#),
        "expansion must keep the source-span fallback:\n{wat}"
    );
    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 0, "{wat}");
    assert_eq!(eval_fallbacks(&wat), 1, "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// A backslash-substituted word is not modelled by the emitter, so the
/// statement declines cleanly rather than emitting the unprocessed bytes.
#[test]
fn analysed_leaf_command_declines_backslash_substitution() {
    let mut module = compile_wasm_analysed("string length a\\tb\n");
    let wat = module.to_wat();

    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 0, "{wat}");
    assert_eq!(eval_fallbacks(&wat), 1, "{wat}");
    assert_eq!(&module.to_bytes()[0..4], b"\0asm");
}

/// Abrupt completion still propagates through the compiled path: a leaf
/// command inside a loop keeps its `break`/`continue` re-entry, and the
/// cleanup that precedes it is unconditional.
#[test]
fn analysed_leaf_command_keeps_the_completion_dispatch_after_cleanup() {
    let mut module = compile_wasm_analysed("while {$go} {string length abc}\n");
    let wat = module.to_wat();

    assert_eq!(import_calls(&wat, "tcl_invoke_argv"), 1, "{wat}");
    // The frame is freed before any completion branch can leave the statement.
    let free = wat
        .lines()
        .position(|line| line.trim() == format!("call {}", frame_free_index(&wat)))
        .expect("frame free is emitted");
    let dispatch = wat
        .lines()
        .position(|line| line.trim() == "i32.eq")
        .expect("the completion dispatch tests break/continue");
    assert!(free < dispatch, "cleanup must precede the dispatch:\n{wat}");
}

/// The import index of `tcl_codegen_call_frame_free`.
fn frame_free_index(wat: &str) -> usize {
    import_lines(wat)
        .iter()
        .position(|line| line.contains("\"tcl_codegen_call_frame_free\""))
        .expect("the frame-free import is declared")
}

/// With a non-zero data base, both the data segments and the `i32.const` offsets
/// the module hands to `tcl_obj_new_string` are relocated into the runtime's
/// reserved region — so the emitted constant pool does not collide with the
/// runtime's shadow stack in the shared-memory whole-program link.
#[test]
fn data_pool_relocates_to_reserved_base() {
    let mut m = compile_wasm_based("set x 5\nputs $x\n", RESERVED_DATA_BASE);
    let wat = m.to_wat();
    // First string sits at the base, the second right after it (base + 7).
    assert!(
        wat.contains(&format!(
            "(data (i32.const {RESERVED_DATA_BASE}) \"set x 5\")"
        )),
        "{wat}"
    );
    let second = RESERVED_DATA_BASE + i64::try_from("set x 5".len()).unwrap();
    assert!(
        wat.contains(&format!("(data (i32.const {second}) \"puts $x\")")),
        "{wat}"
    );
    // The body pushes the *based* offset (not 0) before calling tcl_obj_new_string.
    assert!(
        wat.contains(&format!("i32.const {RESERVED_DATA_BASE}")),
        "{wat}"
    );
    // Still a valid module, and offset 0 no longer carries data.
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
    assert!(!wat.contains("(data (i32.const 0)"), "{wat}");
}

/// A user-defined `proc` is emitted as its own exported WASM function, its body
/// driven through the same structured walk as `::top`. (The `proc` definition
/// itself still eval-fallbacks in `::top` to register it at run time, until
/// call-dispatch is wired.)
#[test]
fn procs_emit_their_own_functions() {
    let mut m = compile_wasm("proc add {a b} {return [expr {$a + $b}]}\nadd 1 2\n");
    let wat = m.to_wat();
    // Both the top-level entry and the proc body are exported functions.
    assert!(wat.contains(r#"(export "::top")"#), "{wat}");
    assert!(wat.contains(r#"(export "::add")"#), "{wat}");
    // The proc body's command is interned (from the `::add` walk).
    assert!(wat.contains(r#""return [expr {$a + $b}]""#), "{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// A proc body's control flow is **structured inside its own function**, not an
/// opaque eval-fallback of the whole body.
#[test]
fn proc_body_control_flow_is_structured() {
    let mut m = compile_wasm("proc choose {x} {if {$x} {puts a} else {puts b}}\n");
    let wat = m.to_wat();
    assert!(wat.contains(r#"(export "::choose")"#), "{wat}");
    assert!(
        wat.contains("\n        if"),
        "expected structured if in the proc body:\n{wat}"
    );
    // The condition + both arms are interned (driven through the walk).
    assert!(wat.contains(r#""$x""#), "{wat}");
    assert!(
        wat.contains(r#""puts a""#) && wat.contains(r#""puts b""#),
        "{wat}"
    );
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// Namespace-scoped procs (created at run time inside `namespace eval`, not at
/// load) are not emitted as load-time functions — mirroring the bytecode backend.
#[test]
fn namespace_scoped_procs_are_not_emitted() {
    let mut m = compile_wasm("namespace eval ns {\n  proc p {} {puts hi}\n}\n");
    let wat = m.to_wat();
    assert!(
        !wat.contains(r#"(export "::ns::p")"#),
        "a namespace-scoped proc must not become a load-time function:\n{wat}"
    );
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// A linear top-level script: each command is eval-fallback'd in order, results
/// discarded; the command texts live in the data section.
#[test]
fn linear_top_level_eval_fallback() {
    let mut m = compile_wasm("set x 5\nputs $x\n");
    let wat = m.to_wat();
    // The eval-fallback import boundary is declared.
    assert!(wat.contains(r#""tcl_obj_new_string""#), "{wat}");
    assert!(wat.contains(r#""tcl_eval_code""#), "{wat}");
    assert!(wat.contains(r#""tcl_expr_bool""#), "{wat}");
    assert!(wat.contains(r#""memory""#), "{wat}");
    // Both commands' source text is interned in the data section.
    assert!(wat.contains(r#""set x 5""#), "{wat}");
    assert!(wat.contains(r#""puts $x""#), "{wat}");
    // Exported top-level entry.
    assert!(wat.contains(r#"(export "::top")"#), "{wat}");
    // Two eval-fallback sequences (box → eval_code → dispatch) ⇒ two `call 1`
    // (`tcl_eval_code`, import index 1).
    assert_eq!(m.to_wat().matches("\n        call 1").count(), 2, "{wat}");
    // Valid module header.
    let bytes = m.to_bytes();
    assert_eq!(&bytes[0..4], b"\0asm", "wasm magic");
}

/// An `if`/`else` is recovered as **structured** WASM control flow, with the
/// clean (brace-stripped) condition text interned for `tcl_expr_bool`.
#[test]
fn if_else_is_structured() {
    let mut m = compile_wasm("if {1} {puts a} else {puts b}\n");
    let wat = m.to_wat();
    assert!(
        wat.contains("\n        if"),
        "expected structured if:\n{wat}"
    );
    assert!(wat.contains("else"), "expected else arm:\n{wat}");
    assert!(wat.contains("end"), "expected end:\n{wat}");
    // The condition is interned brace-stripped (`1`, not `{1`).
    assert!(wat.contains(r#"(data (i32.const 0) "1")"#), "{wat}");
    // Both arms' bodies are present in the data section.
    assert!(wat.contains(r#""puts a""#), "{wat}");
    assert!(wat.contains(r#""puts b""#), "{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// A `while` loop becomes a structured `block`/`loop` with a `br_if` guard.
#[test]
fn while_loop_is_structured() {
    let mut m = compile_wasm("while {$i < 10} {puts $i}\n");
    let wat = m.to_wat();
    assert!(
        wat.contains("\n        block"),
        "expected break block:\n{wat}"
    );
    assert!(wat.contains("loop"), "expected loop:\n{wat}");
    assert!(wat.contains("br_if"), "expected guard br_if:\n{wat}");
    // Condition interned brace-stripped; body interned.
    assert!(wat.contains(r#""$i < 10""#), "{wat}");
    assert!(wat.contains(r#""puts $i""#), "{wat}");
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// `break` / `continue` inside a loop are realised as structured branches, not
/// eval-fallback'd commands.
#[test]
fn break_continue_are_structured() {
    let mut m = compile_wasm("while {1} {if {$x} {break} else {continue}}\n");
    let wat = m.to_wat();
    // Two unconditional branches (break + continue); the guard uses br_if.
    assert!(
        wat.contains("\n            br ") || wat.contains("\n                br "),
        "{wat}"
    );
    // `break`/`continue` must NOT be interned as eval-fallback command text.
    assert!(
        !wat.contains(r#""break""#),
        "break should be structural, not eval:\n{wat}"
    );
    assert!(
        !wat.contains(r#""continue""#),
        "continue should be structural:\n{wat}"
    );
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

/// `foreach` (whose iteration eval-fallback can't realise structurally) degrades
/// to a single whole-command eval-fallback.
#[test]
fn foreach_is_opaque_eval_fallback() {
    let mut m = compile_wasm("foreach x {a b c} {puts $x}\n");
    let wat = m.to_wat();
    // The whole `foreach …` command is interned and eval'd as one unit; there is
    // no structured `loop` recovered for it.
    assert!(wat.contains(r#""foreach x {a b c} {puts $x}""#), "{wat}");
    assert!(
        !wat.contains("\n            loop"),
        "foreach must not structure:\n{wat}"
    );
    assert_eq!(&m.to_bytes()[0..4], b"\0asm");
}

fn have_wasmtime() -> bool {
    std::process::Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Validate one emitted module with `wasmtime compile`.
fn validate(name: &str, module: &mut WasmModule) {
    let tmp = std::env::temp_dir();
    let wasm = tmp.join(format!("tcl_wasm_stage2_{name}.wasm"));
    let cwasm = tmp.join(format!("tcl_wasm_stage2_{name}.cwasm"));
    std::fs::write(&wasm, module.to_bytes()).expect("write wasm");
    let status = std::process::Command::new("wasmtime")
        .arg("compile")
        .arg(&wasm)
        .arg("-o")
        .arg(&cwasm)
        .output()
        .expect("run wasmtime");
    assert!(
        status.status.success(),
        "{name}.wasm failed wasmtime validation:\n{}\n--- wat ---\n{}",
        String::from_utf8_lossy(&status.stderr),
        module.to_wat(),
    );
    let _ = std::fs::remove_file(&wasm);
    let _ = std::fs::remove_file(&cwasm);
}

/// The compiled prebuilt-argv path is **structurally valid WASM** across every
/// word shape it accepts, including the abrupt-completion branches out of a
/// part-built argv.
#[test]
fn wasmtime_validates_analysed_argv_modules() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping structural validation");
        return;
    }
    for (name, src) in [
        ("argv-literal", "string tolower ABC\n"),
        ("argv-braced", "puts {a b c}\n"),
        ("argv-scalar", "string length $value\n"),
        ("argv-element", "string length $a(b)\n"),
        ("argv-compound", "string length \"hi $name!\"\n"),
        ("argv-nested", "string length [string tolower ABC]\n"),
        (
            "argv-deep-nested",
            "string length [string tolower [string trim \" ABC \"]]\n",
        ),
        ("argv-in-if", "if {$x} {string length $value}\n"),
        (
            "argv-in-loop",
            "while {$go} {string length $value\nlappend out $value}\n",
        ),
        (
            "argv-in-loop-break",
            "while {1} {if {$done} {break}\nstring length [set v]}\n",
        ),
        (
            "argv-in-for",
            "for {set i 0} {$i < 3} {incr i} {lappend out [string index $s $i]}\n",
        ),
        ("argv-mixed-decline", "string length {*}$args\nlist a b\n"),
        // A *partially emitted* direct attempt that then declines, with the
        // argv path picking the statement up. The direct proc-call
        // specialisation boxes the literal `2` before it reaches `$a(x)`, a
        // word it does not handle. Unless that prefix is rolled back it stays
        // in the body with its operand stranded, and wasmtime rejects the
        // module with "values remaining on stack at end of block". Reported on
        // #1413; a behavioural test cannot catch it because the statement's
        // result is still correct — only validation sees the malformed body.
        (
            "argv-after-partially-emitted-direct-decline",
            "proc add {b c} {return [expr {$b + $c}]}\nset a(x) 4\nputs [add 2 $a(x)]\n",
        ),
    ] {
        validate(name, &mut compile_wasm_analysed(src));
    }
}

/// Every emitted module is **structurally valid WASM** — confirmed by
/// `wasmtime compile` (which fully validates before native compilation). Skips
/// gracefully where the `wasmtime` CLI isn't available.
#[test]
fn wasmtime_validates_emitted_modules() {
    if !have_wasmtime() {
        eprintln!("wasmtime CLI unavailable; skipping structural validation");
        return;
    }
    let tmp = std::env::temp_dir();
    for (name, src) in [
        ("linear", "set x 5\nputs $x\n"),
        ("if-else", "if {1} {puts a} else {puts b}\n"),
        ("if-noelse", "if {1} {puts a}\nputs after\n"),
        (
            "elseif",
            "if {$a} {puts a} elseif {$b} {puts b} else {puts c}\n",
        ),
        ("nested-if", "if {1} {if {2} {puts a}}\nputs done\n"),
        ("while", "while {$i < 10} {puts $i}\n"),
        ("while-break", "while {1} {if {$done} {break}\nputs x}\n"),
        (
            "while-continue",
            "while {$i} {if {$skip} {continue}\nputs $i}\n",
        ),
        ("for", "for {set i 0} {$i < 10} {incr i} {puts $i}\n"),
        (
            "for-break",
            "for {set i 0} {1} {incr i} {if {$i} {break}}\n",
        ),
        (
            "nested-loop",
            "while {$a} {for {set i 0} {$i<3} {incr i} {if {$i} {break} else {continue}}}\n",
        ),
        ("return-mid", "if {$x} {return 1}\nputs after\n"),
        ("foreach-opaque", "foreach x {a b c} {puts $x}\n"),
        // Multi-function modules: a proc body becomes its own function alongside
        // `::top`; both must validate.
        (
            "proc",
            "proc add {a b} {return [expr {$a + $b}]}\nadd 1 2\n",
        ),
        (
            "proc-cf",
            "proc choose {x} {if {$x} {puts a} else {puts b}}\nchoose 1\n",
        ),
    ] {
        let mut m = compile_wasm(src);
        let wasm = tmp.join(format!("tcl_wasm_stage2_{name}.wasm"));
        let cwasm = tmp.join(format!("tcl_wasm_stage2_{name}.cwasm"));
        std::fs::write(&wasm, m.to_bytes()).expect("write wasm");
        let status = std::process::Command::new("wasmtime")
            .arg("compile")
            .arg(&wasm)
            .arg("-o")
            .arg(&cwasm)
            .output()
            .expect("run wasmtime");
        assert!(
            status.status.success(),
            "{name}.wasm failed wasmtime validation:\n{}\n--- wat ---\n{}",
            String::from_utf8_lossy(&status.stderr),
            m.to_wat(),
        );
        let _ = std::fs::remove_file(&wasm);
        let _ = std::fs::remove_file(&cwasm);
    }
}
