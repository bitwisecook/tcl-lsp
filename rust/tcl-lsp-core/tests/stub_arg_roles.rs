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

//! A `# tcl-lsp: stub` declaration states the same kind of fact a registry
//! `CommandSpec` does, so its argument roles must reach the same consumers by
//! the same path: `arg_role_resolver` → `arg_roles` → the query every
//! consumer asks, answered by the document's own `DeclaredSurface` for the
//! names it declares.
//!
//! Two consumers are pinned here — the call-graph builder (a `script:body`
//! word is a script whose calls are edges of the caller) and the variable
//! analyser (a `var` word is a definition, so a later read is not W210) —
//! each beside the registry command whose behaviour the stub must match.
//!
//! The directive's flags state behavioural facts on the fields a catalogue
//! command states them on, so each flag is pinned the same way: beside the
//! registry command whose finding the flagged stub draws, and beside the same
//! stub without the flag, which draws nothing. A declaration that redeclares
//! a catalogued command answers alone — nearest wins.

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::graphs;
use tcl_registry::CommandRegistry;

const DIALECT: &str = "tcl8.6";

/// The call-graph edges of `source`, as `caller → callee` pairs.
fn edges(source: &str) -> Vec<(String, String)> {
    let registry = CommandRegistry::build_default();
    let graph = graphs::call_graph(
        source,
        &registry,
        tcl_registry::model::ingress::resolve_environment(DIALECT).analyser_profile(),
    );
    graph["edges"]
        .as_array()
        .expect("edges array")
        .iter()
        .map(|edge| {
            (
                edge["caller"].as_str().unwrap_or_default().to_owned(),
                edge["callee"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The `W210` (read before set) messages `source` draws.
fn w210(source: &str) -> Vec<String> {
    Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W210")
        .map(|d| d.message.clone())
        .collect()
}

/// Every diagnostic code `source` draws.
fn codes(source: &str) -> Vec<String> {
    Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// Whether the call graph of `source` records `from → to`.
fn calls(source: &str, from: &str, to: &str) -> bool {
    edges(source)
        .iter()
        .any(|(caller, callee)| caller == from && callee == to)
}

// ───────────────────────── ArgRole::Body → call graph ─────────────────────

/// The registry baseline: a shipped command whose spec marks an argument
/// `ArgRole::Body` makes the procedures that body calls edges of the caller.
#[test]
fn registry_body_role_is_a_call_graph_edge() {
    let source = "proc on_row {} { puts row }\nproc main {} {\n    time { on_row }\n}\n";
    assert!(
        calls(source, "::main", "::on_row"),
        "a registry body command's script is the caller's own calls; got {:?}",
        edges(source)
    );
}

/// The same fact declared by a stub reaches the same builder: `script:body`
/// makes `db_eval`'s second word a script, so `on_row` is reachable from
/// `main` exactly as it is through `time` above.
#[test]
fn stub_body_role_is_a_call_graph_edge() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub db_eval {sql script:body} -barrier\n",
        "# tcl-lsp: stubs-end\n",
        "proc on_row {} { puts row }\n",
        "proc main {} {\n",
        "    db_eval \"select 1\" { on_row }\n",
        "}\n",
    );
    assert!(
        calls(source, "::main", "::on_row"),
        "a stub-declared body must contribute the same edge a registry body does; got {:?}",
        edges(source)
    );
}

/// Without the role the word is an opaque string, so there is no edge — the
/// declaration is what does the work, not the braces.
#[test]
fn an_undeclared_body_word_is_not_a_call_graph_edge() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub db_eval {sql script}\n",
        "# tcl-lsp: stubs-end\n",
        "proc on_row {} { puts row }\n",
        "proc main {} {\n",
        "    db_eval \"select 1\" { on_row }\n",
        "}\n",
    );
    assert!(
        !calls(source, "::main", "::on_row"),
        "a role-less word is data, not a script; got {:?}",
        edges(source)
    );
}

/// A body that runs in another namespace is still left to the body unit that
/// owns it: walking `namespace eval ::b { helper }` here would invent an edge
/// to a same-named procedure in the *caller's* namespace (issues #977/#980).
#[test]
fn a_namespace_shifted_body_invents_no_caller_namespace_edge() {
    let source = concat!(
        "proc helper {} { puts global }\n",
        "namespace eval ::a {\n",
        "    proc helper {} { puts inner }\n",
        "    proc run_it {} { namespace eval ::b { helper } }\n",
        "}\n",
    );
    assert!(
        !calls(source, "::a::run_it", "::a::helper"),
        "a `namespace eval ::b` body does not call the caller's namespace; got {:?}",
        edges(source)
    );
}

/// The body word is walked as a script, not merely counted as one: a command
/// the workspace does not know, written inside it, draws the same W123 it
/// would inside a registry body command's script.
#[test]
fn a_stub_declared_body_is_analysed_as_a_script() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub db_eval {sql script:body}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    db_eval \"select 1\" { totally_unknown_cmd }\n",
        "}\n",
    );
    let found: Vec<String> = Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W123")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        found.len(),
        1,
        "the stubbed body's own commands are resolved; got {found:?}"
    );
    assert!(found[0].contains("totally_unknown_cmd"), "got {found:?}");
}

// ───────────────────────── ArgRole::Expr → call graph ─────────────────────

/// The registry baseline: brace quoting suppresses substitution at the word
/// level, but the expression engine re-evaluates the operand, so a `[cmd …]`
/// inside `expr {…}` is a real call.
#[test]
fn registry_expr_role_is_a_call_graph_edge() {
    let source = "proc score {} { return 1 }\nproc main {} { expr {[score] > 0} }\n";
    assert!(
        calls(source, "::main", "::score"),
        "an expression operand's substitutions are calls; got {:?}",
        edges(source)
    );
}

/// The same fact declared by a stub: `cond:expr` makes the word an
/// expression, so the command substituted inside it is a call.
#[test]
fn stub_expr_role_is_a_call_graph_edge() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub assert_that {cond:expr}\n",
        "# tcl-lsp: stubs-end\n",
        "proc score {} { return 1 }\n",
        "proc main {} { assert_that {[score] > 0} }\n",
    );
    assert!(
        calls(source, "::main", "::score"),
        "a stub-declared expression operand's substitutions are calls; got {:?}",
        edges(source)
    );
}

/// A deferred script is reachable code too: the callback a registry command
/// schedules for later is an edge of the procedure that schedules it.
#[test]
fn a_deferred_registry_body_is_a_call_graph_edge() {
    let source = "proc on_row {} { puts row }\nproc main {} {\n    after 100 { on_row }\n}\n";
    assert!(
        calls(source, "::main", "::on_row"),
        "a scheduled script's calls are the scheduler's own; got {:?}",
        edges(source)
    );
}

/// An expression operand draws the expression diagnostics whichever source
/// declared it: an unbraced one is a double-substitution risk either way.
#[test]
fn a_stub_declared_expression_draws_the_unbraced_warning() {
    let registry = "proc main {x} { if $x { puts hi } }\n";
    assert!(
        codes(registry).contains(&"W100".to_owned()),
        "the registry baseline warns on an unbraced expression; got {:?}",
        codes(registry)
    );
    let stubbed = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub calculate {condition:expr}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {x} { calculate $x }\n",
    );
    assert!(
        codes(stubbed).contains(&"W100".to_owned()),
        "a stub-declared expression warns the same way; got {:?}",
        codes(stubbed)
    );
}

// ───────────────────── ArgRole::CommandPrefix → call graph ────────────────

/// The registry baseline: `lsort -command` names a callback the sort
/// invokes, so the procedure it names is an edge of the caller.
#[test]
fn registry_command_prefix_role_is_a_call_graph_edge() {
    let source = concat!(
        "proc compare {a b} { expr {$a - $b} }\n",
        "proc main {items} { return [lsort -command compare $items] }\n",
    );
    assert!(
        calls(source, "::main", "::compare"),
        "a registry callback position is an edge; got {:?}",
        edges(source)
    );
}

/// The same fact declared by a stub: `cb:command_prefix` names the word
/// `on_event` invokes as a callback.
#[test]
fn stub_command_prefix_role_is_a_call_graph_edge() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub on_event {name cb:command_prefix}\n",
        "# tcl-lsp: stubs-end\n",
        "proc handler {} { puts hi }\n",
        "proc main {} { on_event click handler }\n",
    );
    assert!(
        calls(source, "::main", "::handler"),
        "a stub-declared callback position is an edge; got {:?}",
        edges(source)
    );
}

// ────────────────────────────── role vocabulary ───────────────────────────

/// Whether `source` leaves `myc` — the stubbed head every vocabulary case
/// declares — reported as an unknown command.
///
/// A `W123` naming something else is not the answer: a `body` role makes its
/// word a script, so the fixture's argument resolves as a command of its own.
fn head_is_unresolved(source: &str) -> bool {
    Analyser::new()
        .analyse(source, DIALECT)
        .diagnostics
        .iter()
        .any(|d| d.code.to_string() == "W123" && d.message.contains("myc"))
}

/// Every role word the registry knows is one the directive parser accepts:
/// a word in the table must not be rejected as a typo.
#[test]
fn every_registry_role_word_is_accepted_by_the_directive() {
    for word in [
        "body",
        "expr",
        "var",
        "var_read",
        "name",
        "pattern",
        "channel",
        "command_prefix",
        "value",
    ] {
        let source = format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub myc {{a:{word}}}\n# tcl-lsp: stubs-end\nmyc x\n"
        );
        assert!(
            !head_is_unresolved(&source),
            "`{word}` must declare the command; got {:?}",
            codes(&source)
        );
    }
}

/// A word the registry does not know is a typo, not a silent generic role:
/// the declaration is dropped and the command stays unresolved, which is
/// what tells the author their spelling is wrong.
#[test]
fn an_unknown_role_word_drops_the_declaration() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub myc {a:nonsense}\n",
        "# tcl-lsp: stubs-end\n",
        "myc x\n",
    );
    assert!(
        head_is_unresolved(source),
        "a typo'd role leaves the command unresolved; got {:?}",
        codes(source)
    );
}

// ────────────────────────── optional argument slots ───────────────────────

/// A declared position is not a call position. With the optional slot
/// omitted, the `var` role lands on the word the call actually passed.
#[test]
fn an_omitted_optional_slot_shifts_the_var_role() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch {?table? row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch out\n",
        "    puts $out\n",
        "}\n",
    );
    assert!(
        w210(source).is_empty(),
        "the write lands on the supplied word, not the declared index; got {:?}",
        w210(source)
    );
}

/// The same shape for a body role: the script is still found when the
/// optional slot before it is omitted.
#[test]
fn an_omitted_optional_slot_shifts_the_body_role() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub visit {?context? script:body}\n",
        "# tcl-lsp: stubs-end\n",
        "proc on_row {} { puts row }\n",
        "proc main {} { visit { on_row } }\n",
    );
    assert!(
        calls(source, "::main", "::on_row"),
        "the body lands on the supplied word; got {:?}",
        edges(source)
    );
}

/// Supplying the optional slot puts the role back where the declaration
/// writes it, so the shift tracks the call rather than being a fixed offset.
#[test]
fn a_supplied_optional_slot_keeps_the_declared_position() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch {?table? row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch t out\n",
        "    puts $out\n",
        "}\n",
    );
    assert!(
        w210(source).is_empty(),
        "with the optional word present the write is at index 1; got {:?}",
        w210(source)
    );
}

// ──────────────────────── ArgRole::VarWrite → W210 ────────────────────────

/// The registry baseline: `scan`'s output word is `ArgRole::VarWrite`, so the
/// variable it names is defined and reading it afterwards is not W210.
#[test]
fn registry_var_role_suppresses_w210() {
    let source = "proc main {} {\n    scan \"1\" %d out\n    puts $out\n}\n";
    assert!(
        w210(source).is_empty(),
        "a registry-declared variable write defines the name; got {:?}",
        w210(source)
    );
}

/// The same fact declared by a stub: `row:var` says `fetch_row` writes the
/// variable its second word names, so the later read is not read-before-set.
#[test]
fn stub_var_role_suppresses_w210() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch_row {table row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch_row t out\n",
        "    puts $out\n",
        "}\n",
    );
    assert!(
        w210(source).is_empty(),
        "a stub-declared variable write must suppress W210 as a registry one does; got {:?}",
        w210(source)
    );
}

/// The suppression is the declared role's, not a blanket amnesty: a variable
/// the stub never names is still read before it is set.
#[test]
fn stub_var_role_suppresses_only_the_name_it_declares() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub fetch_row {table row:var}\n",
        "# tcl-lsp: stubs-end\n",
        "proc main {} {\n",
        "    fetch_row t out\n",
        "    puts $other\n",
        "}\n",
    );
    let found = w210(source);
    assert_eq!(
        found.len(),
        1,
        "only the undeclared `other` is read before set; got {found:?}"
    );
    assert!(found[0].contains("other"), "got {found:?}");
}

// ─────────────────── declared roles → call-site evidence ──────────────────

// The interprocedural parameter seed folds a parameter every *caller*
// passes the same literal for, so a caller the scan cannot see is unsound.
// A callback registration and a script body are both callers, and a
// declaration names them exactly as a `CommandSpec` does.

/// `helper`, whose one ordinary caller passes `prod`, reached a second time
/// through `command` — a callback registration or a script body, depending on
/// the role the surface declares for it.
fn callback_registration(stub: &str, command: &str) -> String {
    format!(
        concat!(
            "{stub}",
            "proc helper {{mode}} {{\n",
            "    if {{$mode eq \"prod\"}} {{ set x 1 }} else {{ set x 2 }}\n",
            "}}\n",
            "proc main {{}} {{\n",
            "    helper prod\n",
            "    {command}\n",
            "}}\n",
        ),
        stub = stub,
        command = command,
    )
}

/// The registry baseline: a callback whose arguments the scan cannot read is
/// a caller all the same, so `mode` is not a compile-time constant.
#[test]
fn registry_command_prefix_callback_withholds_the_fold() {
    let source = callback_registration("", "after 0 helper");
    assert!(
        !codes(&source).contains(&"I230".to_owned()),
        "a registry callback registration must withhold the fold; got {:?}",
        codes(&source)
    );
}

#[test]
fn a_stub_declared_command_prefix_callback_withholds_the_fold() {
    let source = callback_registration(
        concat!(
            "# tcl-lsp: stubs-begin\n",
            "# tcl-lsp: stub register_cb {event cb:command_prefix}\n",
            "# tcl-lsp: stubs-end\n",
        ),
        "register_cb evt helper",
    );
    assert!(
        !codes(&source).contains(&"I230".to_owned()),
        "a stub-declared callback registration must withhold the fold as a registry one does; got {:?}",
        codes(&source)
    );
}

/// The registry baseline for the body half: a call site inside a script
/// argument is a call site.
#[test]
fn registry_body_call_site_withholds_the_fold() {
    let source = callback_registration("", "catch { helper dev }");
    assert!(
        !codes(&source).contains(&"I230".to_owned()),
        "a call site inside a registry body must withhold the fold; got {:?}",
        codes(&source)
    );
}

#[test]
fn a_stub_declared_body_call_site_withholds_the_fold() {
    let source = callback_registration(
        concat!(
            "# tcl-lsp: stubs-begin\n",
            "# tcl-lsp: stub my_eval {script:body}\n",
            "# tcl-lsp: stubs-end\n",
        ),
        "my_eval { helper dev }",
    );
    assert!(
        !codes(&source).contains(&"I230".to_owned()),
        "a call site inside a stub-declared body must withhold the fold as a registry one does; got {:?}",
        codes(&source)
    );
}

/// TP control: the declaration withholds the fold only where it names a
/// caller. A stub whose callback is some *other* procedure leaves `helper`'s
/// callers as uniform as they were, so the fold still happens.
#[test]
fn a_stub_callback_naming_another_proc_leaves_the_fold_standing() {
    let source = concat!(
        "# tcl-lsp: stubs-begin\n",
        "# tcl-lsp: stub register_cb {event cb:command_prefix}\n",
        "# tcl-lsp: stubs-end\n",
        "proc helper {mode} {\n",
        "    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n",
        "}\n",
        "proc other {} { puts other }\n",
        "proc main {} {\n",
        "    helper prod\n",
        "    register_cb evt other\n",
        "}\n",
    );
    assert!(
        codes(source).contains(&"I230".to_owned()),
        "a callback naming another proc must not withhold helper's fold; got {:?}",
        codes(source)
    );
}

// ─────────────────────── flags → catalogue fields ────────────────────────

// Each flag lands on the field its catalogue counterpart states the same
// fact on, so each pair below is a registry command's finding, the stub
// that states the same fact drawing it, and the same stub without the flag
// drawing nothing — a stub that states no behaviour reads exactly as an
// undeclared command.

/// `source` behind an inline stub block declaring `stub`.
fn stubbed(stub: &str, source: &str) -> String {
    format!("# tcl-lsp: stubs-begin\n# tcl-lsp: stub {stub}\n# tcl-lsp: stubs-end\n{source}")
}

/// Every optimisation code `source` draws, over a unit built as the server
/// builds one: the document's own declarations reach the lowering and the
/// interprocedural summary.
fn optimisation_codes(source: &str) -> Vec<String> {
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    let profile = tcl_registry::model::ingress::resolve_environment(DIALECT).analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for(DIALECT).commands();
    let declared = tcl_compiler::analyser::utils::document_declared_surface(source, None, DIALECT);
    let unit = CompilationUnit::build_with_options(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_profile(Some(profile)),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: Some(&declared),
        },
    )
    .with_interprocedural(registry, Some(profile));
    tcl_compiler::optimiser::optimise_unit(&unit, registry, Some(profile))
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect()
}

/// `label` returns what `inner` makes of its argument, and `main` drops the
/// result of calling it — removable exactly when `inner` is pure.
fn pure_wrapper(inner: &str) -> String {
    format!(
        "proc label {{x}} {{ return [{inner} $x] }}\n\
         proc main {{}} {{\n    set a [label abc]\n    return 1\n}}\n"
    )
}

/// `-pure` is `Traits::PURE`: the interprocedural summary classifies the
/// call as a pure one, so `label` is pure and the unused result of calling
/// it goes (O126) as it does over `string length`.
#[test]
fn a_pure_stub_keeps_its_caller_pure() {
    let registry = optimisation_codes(&pure_wrapper("string length"));
    assert!(
        registry.contains(&"O126".to_owned()),
        "the registry baseline: a pure wrapper's unused result goes; got {registry:?}"
    );
    let flagged = optimisation_codes(&stubbed("mypure {x} -pure", &pure_wrapper("mypure")));
    assert!(
        flagged.contains(&"O126".to_owned()),
        "a `-pure` stub is pure to the summary; got {flagged:?}"
    );
    let flagless = optimisation_codes(&stubbed("mypure {x}", &pure_wrapper("mypure")));
    assert!(
        !flagless.contains(&"O126".to_owned()),
        "without `-pure` the call may do anything, so the result stays; got {flagless:?}"
    );
}

/// `set x 1`, then `command` naming `x`, then a read of `x`.
fn store_then(command: &str) -> String {
    format!("proc p {{}} {{\n    set x 1\n    {command}\n    return $x\n}}\n")
}

/// `-mutator` is the read-modify-write shape `lappend` states: a read of the
/// target before the write, so the store feeding it is live (no O109).
#[test]
fn a_mutator_stub_keeps_the_store_it_reads() {
    let registry = optimisation_codes(&store_then("lappend x 2"));
    assert!(
        !registry.contains(&"O109".to_owned()),
        "the registry baseline: `lappend` reads the store it extends; got {registry:?}"
    );
    let flagged = optimisation_codes(&stubbed("mymut {v:var} -mutator", &store_then("mymut x")));
    assert!(
        !flagged.contains(&"O109".to_owned()),
        "a `-mutator` stub reads its target before writing it; got {flagged:?}"
    );
    let flagless = optimisation_codes(&stubbed("mymut {v:var}", &store_then("mymut x")));
    assert!(
        flagless.contains(&"O109".to_owned()),
        "without `-mutator` the declared write kills the store; got {flagless:?}"
    );
}

/// The minified form of `source`, local names compacted.
fn compacted(source: &str) -> String {
    let registry = CommandRegistry::build_default();
    let profile = tcl_registry::model::ingress::resolve_environment(DIALECT).analyser_profile();
    tcl_lsp_core::minify::minify_tcl_compact(source, profile, false, &registry).0
}

/// A local set, `command` run, and the local read back.
fn local_around(command: &str) -> String {
    format!("proc main {{}} {{\n    set local 1\n    {command}\n    return $local\n}}\n")
}

/// `-barrier` is `Traits::CREATES_DYNAMIC_BARRIER`: the scope the command runs
/// in may be observed by name, so the minifier leaves its locals' names
/// alone, as it does around `vwait`.
#[test]
fn a_barrier_stub_fences_its_scope_from_renaming() {
    let registry = compacted(&local_around("vwait ::done"));
    assert!(
        registry.contains("$local"),
        "the registry baseline: a dynamic barrier fences the scope; got {registry:?}"
    );
    let flagged = compacted(&stubbed("spy {} -barrier", &local_around("spy")));
    assert!(
        flagged.contains("$local"),
        "a `-barrier` stub fences the scope it runs in; got {flagged:?}"
    );
    let flagless = compacted(&stubbed("spy {}", &local_around("spy")));
    assert!(
        !flagless.contains("$local"),
        "without `-barrier` the local is compacted; got {flagless:?}"
    );
}

/// `-loop` is `Traits::HAS_LOOP_BODY`: with a declared condition and body, the
/// command is a conditional loop to W240 / W241, as `while` is.
#[test]
fn a_loop_stub_is_checked_as_a_loop() {
    let body = |call: &str| format!("proc main {{}} {{\n    set n 0\n    {call}\n}}\n");
    assert!(
        codes(&body("while 1 {incr n}")).contains(&"W241".to_owned()),
        "the registry baseline: a constant-true loop with no exit is W241"
    );
    let flagged = |call: &str| codes(&stubbed("spin {cond:expr body:body} -loop", &body(call)));
    assert!(
        flagged("spin 1 {incr n}").contains(&"W241".to_owned()),
        "a `-loop` stub's constant-true condition with no exit is W241; got {:?}",
        flagged("spin 1 {incr n}")
    );
    assert!(
        flagged("spin 0 {incr n}").contains(&"W240".to_owned()),
        "a `-loop` stub's constant-false condition is W240; got {:?}",
        flagged("spin 0 {incr n}")
    );
    assert!(
        !flagged("spin 1 {incr n; break}").contains(&"W241".to_owned()),
        "a body that leaves the loop is not provably infinite"
    );
    let flagless = codes(&stubbed(
        "spin {cond:expr body:body}",
        &body("spin 1 {incr n}"),
    ));
    assert!(
        !flagless.iter().any(|code| code == "W240" || code == "W241"),
        "without `-loop` the body may run once, so nothing is claimed; got {flagless:?}"
    );
}

/// `helper`'s one literal caller passes `prod`; `main` also calls whatever
/// `cb` holds, having run `alias` over it. `cb` holds `other` unless the
/// alias binds it to a variable the scan cannot see.
fn aliased_dispatch(alias: &str) -> String {
    format!(
        "proc helper {{mode}} {{\n    if {{$mode eq \"prod\"}} {{ set x 1 }} else {{ set x 2 }}\n}}\n\
         proc other {{m}} {{ puts $m }}\n\
         proc main {{}} {{\n    helper prod\n    set cb other\n    {alias}\n    $cb dev\n}}\n"
    )
}

/// `-scope_alias` is `Traits::CREATES_SCOPE_ALIAS`: the names the command
/// takes are bound to cells another body may write, as `upvar` binds them, so
/// `cb` no longer holds a known literal and `$cb dev` may be a call of
/// `helper` — the fold is withheld (no I230).
#[test]
fn a_scope_alias_stub_aliases_its_local() {
    let registry = codes(&aliased_dispatch("upvar 1 outer cb"));
    assert!(
        !registry.contains(&"I230".to_owned()),
        "the registry baseline: `upvar` makes `cb` unknown; got {registry:?}"
    );
    let flagged = codes(&stubbed(
        "link_var {other local} -scope_alias",
        &aliased_dispatch("link_var outer cb"),
    ));
    assert!(
        !flagged.contains(&"I230".to_owned()),
        "a `-scope_alias` stub makes `cb` unknown as `upvar` does; got {flagged:?}"
    );
    let flagless = codes(&stubbed(
        "link_var {other local}",
        &aliased_dispatch("link_var outer cb"),
    ));
    assert!(
        flagless.contains(&"I230".to_owned()),
        "without `-scope_alias`, `cb` still holds `other` and the fold stands; got {flagless:?}"
    );
}

/// `run` evaluated inside a safe interpreter.
fn in_safe_interp(run: &str) -> String {
    format!("set s [interp create -safe]\ninterp eval $s {{{run}}}\n")
}

/// `-unsafe` is `Traits::UNSAFE` with `Traits::SAFE_INTERP_HIDDEN`, as `exec`
/// states them: inside a safe interpreter the command is hidden, so calling
/// it is W129.
#[test]
fn an_unsafe_stub_is_hidden_in_a_safe_interpreter() {
    let registry = codes(&in_safe_interp("exec ls"));
    assert!(
        registry.contains(&"W129".to_owned()),
        "the registry baseline: `exec` is hidden in a safe interpreter; got {registry:?}"
    );
    let flagged = codes(&stubbed(
        "run_shell {cmd} -unsafe",
        &in_safe_interp("run_shell ls"),
    ));
    assert!(
        flagged.contains(&"W129".to_owned()),
        "an `-unsafe` stub is hidden as `exec` is; got {flagged:?}"
    );
    let flagless = codes(&stubbed("run_shell {cmd}", &in_safe_interp("run_shell ls")));
    assert!(
        !flagless.contains(&"W129".to_owned()),
        "without `-unsafe` nothing says the command is hidden; got {flagless:?}"
    );
}

// ─────────────────────────────── nearest wins ─────────────────────────────

/// A stub that redeclares a catalogued command answers alone: `after ms
/// script` runs its script, but a document declaring `after {ms script}`
/// states that its second word is a value, and the catalogue's `Body` role it
/// omits is not assigned — so `on_row` is no longer a callee of `main`. The
/// same stub stating the role keeps the edge.
#[test]
fn a_stub_that_redeclares_a_catalogued_command_answers_nearest_wins() {
    let source = "proc on_row {} { puts row }\nproc main {} {\n    after 100 { on_row }\n}\n";
    assert!(
        calls(source, "::main", "::on_row"),
        "the catalogue's `after` runs its script; got {:?}",
        edges(source)
    );
    let narrowed = stubbed("after {ms script}", source);
    assert!(
        !calls(&narrowed, "::main", "::on_row"),
        "the declaration's roles answer, not the catalogue's as well; got {:?}",
        edges(&narrowed)
    );
    let restated = stubbed("after {ms script:body}", source);
    assert!(
        calls(&restated, "::main", "::on_row"),
        "a declaration stating the role keeps it; got {:?}",
        edges(&restated)
    );
}
