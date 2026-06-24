//! Port of the Python dataflow analysis suites — def-use chains, the SSA
//! data-flow graph, and dead-store/dead-code elimination.
//!
//! Ported from three pytest files that drive the *Python* compiler:
//!   * `tests/test_def_use.py`        → `build_def_use_chains` over SSA
//!   * `tests/test_dataflow_graph.py` → `extract_dataflow_graph`
//!   * `tests/test_dce.py`            → `compiler.passes.dce.dce_module`
//!
//! The Rust pipeline driver is `CompilationUnit::build_for(src, registry,
//! false)`, which runs lower → CFG → SSA → def-use → SCCP per function. Each
//! [`FunctionUnit`] then carries `.cfg`, `.ssa`, `.def_use`, `.sccp`, `.types`
//! — the same artefacts the Python helpers (`build_cfg` + `build_ssa` +
//! `build_def_use_chains`) produce one at a time. The data-flow graph is built
//! by feeding those per-function units to `extract_dataflow_graph` as
//! [`FunctionInputs`] (the Rust API takes pre-built analyses, not a source
//! string the way Python's `extract_dataflow_graph(src)` does).
//!
//! ## Structural vs. semantic proof split
//!
//! Almost every assertion here is about compiler-internal IR structure — how
//! many SSA defs/uses a snippet produces, which def is a phi, which use is a
//! terminator/phi-incoming edge, whether a chain has zero uses. These facts are
//! properties of the SSA/def-use construction, **not** of any Tcl runtime
//! value, so they need no `tclsh` proof.
//!
//! A handful of assertions *do* rest on a Tcl semantic fact — e.g. "the store
//! to `::result` is observable outside the proc, so it is not a removable dead
//! store", or "`set y 42` as a proc's last command is the implicit return
//! value". Those Tcl facts were checked against real `tclsh8.6`/`tclsh9.0` via
//! `scripts/dev/tclsh_check.sh` and are cited inline next to the test with a
//! `// tclsh:` comment.
//!
//! ## DCE: Rust `liveness_dead_stores` vs. Python `dce_module`
//!
//! The two implementations answer **different questions**, so the DCE port is
//! against the Rust function the prompt names (`dead_stores::liveness_dead_stores`)
//! and `DefUseChain::is_dead`, *not* a line-for-line copy of the Python
//! `dce_module` expectations:
//!
//!   * Python `dce_module` answers *"can this assignment be deleted?"* — and so
//!     guards the **terminal** statement (implicit return), **multi-write**
//!     variables (its "only write" gate), **parameter** slots, and any RHS with
//!     side effects. It conservatively *keeps* stores for those reasons even
//!     when the value is never read back.
//!   * Rust `liveness_dead_stores` answers *"is this store's SSA value never
//!     read?"* — a pure liveness verdict (the def-use chain's "no uses"). It is
//!     the *prerequisite* to deletability, not deletability itself.
//!
//! Where the two agree (a store is both unread *and* safely droppable — the
//! pure-literal cases) the behaviour matches and is asserted directly. Where
//! Python keeps a store for a non-liveness reason (terminal/multi-write/param),
//! Rust's liveness pass still — correctly — reports the value unread; that is a
//! difference in *what the pass computes*, not a bug, and is noted at each site.
//! The genuinely-shared semantic invariants (globals observable ⇒ kept; a
//! command-substitution RHS ⇒ kept because of its side effect) hold in **both**
//! and are the tclsh-anchored ones.

use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::dataflow_graph::{
    extract_dataflow_graph, extract_function_dataflow, DataFlowGraph, EdgeKind, FunctionInputs,
};
use tcl_compiler::dead_stores::liveness_dead_stores;
use tcl_compiler::def_use::{DefKind, DefUseChain, UseKind};
use tcl_compiler::ssa::Version;
use tcl_registry::{registry_for_dialect, CommandRegistry};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    registry_for_dialect(TCL)
}

/// `source → CompilationUnit` (lower → CFG → SSA → def-use → SCCP), the Rust
/// analogue of the Python `_build` helper's `lower_to_ir` + `build_cfg` +
/// `build_ssa` + `build_def_use_chains`.
fn build_cu(source: &str) -> CompilationUnit {
    CompilationUnit::build_for(source, registry(), false)
}

/// The top-level `FunctionUnit` (`::top`) — the Rust analogue of Python's
/// `build_cfg(mod).top_level` def-use result.
fn top_fu(cu: &CompilationUnit) -> &FunctionUnit {
    &cu.top_level
}

/// A specific procedure's `FunctionUnit`.
fn proc_fu<'a>(cu: &'a CompilationUnit, qname: &str) -> &'a FunctionUnit {
    cu.function(qname)
        .unwrap_or_else(|| panic!("function {qname} not found"))
}

/// Look up the def-use chain for `(name, version)` in a unit (Python
/// `du.chain_for(name, version)`).
fn chain<'a>(fu: &'a FunctionUnit, name: &str, version: Version) -> Option<&'a DefUseChain> {
    fu.def_use.chain_for(name, version)
}

/// All SSA versions defined for `name` in a unit (Python `du.reaching_defs`).
fn reaching_defs(fu: &FunctionUnit, name: &str) -> Vec<(String, Version)> {
    fu.def_use.reaching_defs(name)
}

/// Build the module data-flow graph from a (memory-SSA populated) unit — the
/// Rust analogue of Python `extract_dataflow_graph(src)`. Memory-SSA is needed
/// for the alias summaries; SCCP / types feed the node display columns.
fn build_graph(cu: &CompilationUnit) -> DataFlowGraph {
    let inputs: Vec<FunctionInputs> = cu
        .functions()
        .map(|fu| FunctionInputs {
            name: &fu.name,
            ssa: &fu.ssa,
            du: &fu.def_use,
            sccp: Some(&fu.sccp),
            mem: fu.memory_ssa.as_ref(),
            types: Some(&fu.types),
        })
        .collect();
    extract_dataflow_graph(&inputs)
}

/// Build the data-flow graph for `source`, with memory-SSA populated.
fn graph_for(source: &str) -> DataFlowGraph {
    let cu = build_cu(source).with_memory_ssa();
    build_graph(&cu)
}

// ===========================================================================
// test_def_use.py
// ===========================================================================

// -- TestDefUseBasic --

#[test]
fn def_use_linear_single_use() {
    // Python test_linear_single_use: `set x 1\nset y [expr {$x + 1}]` — x#1 has
    // exactly one OPERAND use (read by the expr in `set y`).
    // tclsh: set x 1; set y [expr {$x + 1}] ⇒ y == 2 (x is read once).
    let cu = build_cu("set x 1\nset y [expr {$x + 1}]");
    let fu = top_fu(&cu);
    let x = chain(fu, "x", 1).expect("x#1 chain");
    assert_eq!(x.use_count(), 1);
    assert_eq!(x.uses[0].kind, UseKind::Operand);
}

#[test]
fn def_use_dead_store() {
    // Python test_dead_store: `set x 1\nset x 2` — x#1 dead (overwritten before
    // any read), x#2 dead (never read). tclsh: `set x 1; set x 2; puts done`
    // prints `done`; neither stored 1 nor 2 is ever observed.
    let cu = build_cu("set x 1\nset x 2");
    let fu = top_fu(&cu);
    assert!(chain(fu, "x", 1).expect("x#1").is_dead());
    assert!(chain(fu, "x", 2).expect("x#2").is_dead());
}

#[test]
fn def_use_multiple_uses() {
    // Python test_multiple_uses: `set x 1\nset y $x\nset z $x` — x#1 read twice.
    let cu = build_cu("set x 1\nset y $x\nset z $x");
    let fu = top_fu(&cu);
    assert_eq!(chain(fu, "x", 1).expect("x#1").use_count(), 2);
}

#[test]
fn def_use_reaching_defs() {
    // Python test_reaching_defs: three sequential `set x` ⇒ three SSA defs of x.
    let cu = build_cu("set x 1\nset x 2\nset x 3");
    let fu = top_fu(&cu);
    assert_eq!(reaching_defs(fu, "x").len(), 3);
}

#[test]
fn def_use_dead_chains_property() {
    // Python test_dead_chains_property: `set x 1\nset y 2` — both unused, so the
    // dead-chain set has two entries. (Python `du.dead_chains`; Rust
    // `dead_chains()`.)
    let cu = build_cu("set x 1\nset y 2");
    let fu = top_fu(&cu);
    assert_eq!(fu.def_use.dead_chains().len(), 2);
}

#[test]
fn def_use_total_counts() {
    // Python test_total_counts: 2 defs (x, y), 1 use (x in the expr).
    let cu = build_cu("set x 1\nset y [expr {$x + 1}]");
    let fu = top_fu(&cu);
    assert_eq!(fu.def_use.total_defs(), 2);
    assert_eq!(fu.def_use.total_uses(), 1);
}

// -- TestDefUsePhi --

#[test]
fn def_use_if_merge_creates_phi_chain() {
    // Python test_if_merge_creates_phi_chain: an if/else both assigning `a`,
    // read after the merge, creates a phi def of `a` that itself has ≥1 use
    // (from `set b [expr {$a + 0}]`).
    let cu = build_cu("if {$cond} {set a 1} else {set a 2}\nset b [expr {$a + 0}]");
    let fu = top_fu(&cu);
    let phi_defs: Vec<_> = fu
        .def_use
        .chains
        .iter()
        .filter(|(k, c)| k.0 == "a" && c.definition.kind == DefKind::Phi)
        .collect();
    assert!(!phi_defs.is_empty(), "expected ≥1 phi def for `a`");
    let (_, phi_chain) = phi_defs[0];
    assert!(phi_chain.use_count() >= 1);
}

#[test]
fn def_use_phi_incoming_edges_are_uses() {
    // Python test_phi_incoming_edges_are_uses: with `a` read after the merge
    // (`puts $a`), both branch defs (a#2, a#3) feed the phi via PHI_INCOMING
    // uses ⇒ at least two phi-incoming uses across the reaching defs.
    let cu = build_cu("if {$cond} {set a 1} else {set a 2}\nputs $a");
    let fu = top_fu(&cu);
    let phi_uses = reaching_defs(fu, "a")
        .iter()
        .filter_map(|k| chain(fu, &k.0, k.1))
        .flat_map(|c| c.uses.iter())
        .filter(|u| u.kind == UseKind::PhiIncoming)
        .count();
    assert!(phi_uses >= 2, "expected phi-incoming uses for both branches");
}

// -- TestDefUseTerminator --

#[test]
fn def_use_branch_condition_is_use() {
    // Python test_branch_condition_is_use: `set cond 1\nif {$cond} {...}` —
    // cond#1 is read by the branch, so it is live and has a TERMINATOR use.
    let cu = build_cu("set cond 1\nif {$cond} { set x 1 }");
    let fu = top_fu(&cu);
    let cond = chain(fu, "cond", 1).expect("cond#1");
    assert!(!cond.is_dead(), "cond#1 read by branch — not dead");
    assert!(cond.use_count() >= 1);
    assert!(cond.uses.iter().any(|u| u.kind == UseKind::Terminator));
}

#[test]
fn def_use_while_condition_is_use() {
    // Python test_while_condition_is_use: the phi-merged version of `i` is read
    // by the `while {$i < 10}` condition ⇒ a TERMINATOR use exists.
    let cu = build_cu("set i 0\nwhile {$i < 10} {incr i}");
    let fu = top_fu(&cu);
    let has_terminator = reaching_defs(fu, "i")
        .iter()
        .filter_map(|k| chain(fu, &k.0, k.1))
        .flat_map(|c| c.uses.iter())
        .any(|u| u.kind == UseKind::Terminator);
    assert!(has_terminator, "expected a TERMINATOR use for the loop condition");
}

// -- TestDefUseLoop --

#[test]
fn def_use_loop_phi() {
    // Python test_loop_phi: `set i 0\nwhile {$i < 10} {incr i}` has ≥2 defs of i
    // (the initial store + the loop phi + the incr). tclsh: loop ends with i==10.
    let cu = build_cu("set i 0\nwhile {$i < 10} {incr i}");
    let fu = top_fu(&cu);
    assert!(
        reaching_defs(fu, "i").len() >= 2,
        "expected ≥2 defs for i, got {}",
        reaching_defs(fu, "i").len()
    );
}

// -- TestDefUseProc --

#[test]
fn def_use_proc_parameters() {
    // Python test_proc_parameters: in `proc foo {x y} { set z [expr {$x + $y}]
    // return $z }`, params x and y are version-0 chains each read exactly once
    // (in the `$x + $y` expr). tclsh: foo 3 4 ⇒ 7 (each param read once).
    let cu = build_cu("proc foo {x y} { set z [expr {$x + $y}]\n return $z }");
    let fu = proc_fu(&cu, "::foo");
    assert_eq!(fu.def_use.uses_of("x", 0).len(), 1);
    assert_eq!(fu.def_use.uses_of("y", 0).len(), 1);
    // Both are genuine parameter defs (version 0, read-before-set).
    assert_eq!(chain(fu, "x", 0).unwrap().definition.kind, DefKind::Parameter);
    assert_eq!(chain(fu, "y", 0).unwrap().definition.kind, DefKind::Parameter);
}

// -- TestDefUseResultMethods --

#[test]
fn def_use_is_dead_method() {
    // Python test_is_dead_method: `set x 1\nset y $x` — x read (not dead), y
    // never read (dead). Uses the `DefUseResult::is_dead(name, version)` method.
    let cu = build_cu("set x 1\nset y $x");
    let fu = top_fu(&cu);
    assert!(!fu.def_use.is_dead("x", 1));
    assert!(fu.def_use.is_dead("y", 1));
}

#[test]
fn def_use_is_dead_nonexistent_key() {
    // Python test_is_dead_nonexistent_key: a missing key is reported dead (the
    // `is_none_or(is_dead)` contract).
    let cu = build_cu("set x 1");
    let fu = top_fu(&cu);
    assert!(fu.def_use.is_dead("nonexistent", 99));
}

#[test]
fn def_use_chain_for_returns_none() {
    // Python test_chain_for_returns_none: a missing key yields None.
    let cu = build_cu("set x 1");
    let fu = top_fu(&cu);
    assert!(chain(fu, "nonexistent", 99).is_none());
}

#[test]
fn def_use_has_phi_use() {
    // Python test_has_phi_use: with `a` read after the merge, at least one branch
    // def has a phi use (`DefUseChain::has_phi_use`).
    let cu = build_cu("if {$cond} {set a 1} else {set a 2}\nputs $a");
    let fu = top_fu(&cu);
    let has_phi = reaching_defs(fu, "a")
        .iter()
        .filter_map(|k| chain(fu, &k.0, k.1))
        .any(DefUseChain::has_phi_use);
    assert!(has_phi);
}

// -- TestDefUseNestedControlFlow --

#[test]
fn def_use_if_inside_while() {
    // Python test_if_inside_while: nested if-in-while gives ≥2 defs of i (init +
    // loop phi) and ≥2 defs of x (the two branch stores).
    let cu = build_cu("set i 0\nwhile {$i < 10} {\nif {$i > 5} {set x 1} else {set x 2}\nincr i\n}");
    let fu = top_fu(&cu);
    assert!(reaching_defs(fu, "i").len() >= 2);
    assert!(reaching_defs(fu, "x").len() >= 2);
}

#[test]
fn def_use_foreach_loop() {
    // Python test_foreach_loop: `foreach item {a b c} { set result $item }` has
    // ≥1 def of the loop variable `item`. tclsh: after the loop item == c.
    let cu = build_cu("foreach item {a b c} { set result $item }");
    let fu = top_fu(&cu);
    assert!(reaching_defs(fu, "item").len() >= 1);
}

// ===========================================================================
// test_dataflow_graph.py
// ===========================================================================

// -- TestDataFlowGraphExtraction --

#[test]
fn dataflow_simple_linear() {
    // Python test_simple_linear: ≥1 function, ≥2 defs, ≥1 use.
    let g = graph_for("set x 1\nset y [expr {$x + 1}]");
    assert!(!g.functions.is_empty());
    assert!(g.total_defs() >= 2);
    assert!(g.total_uses() >= 1);
}

#[test]
fn dataflow_dead_definition_detected() {
    // Python test_dead_definition_detected: the never-read `unused` is a dead
    // node. tclsh: `set unused 42` alone produces no observable effect — the
    // value is never read.
    let g = graph_for("set unused 42");
    let top = &g.functions[0];
    let dead_vars: Vec<&str> = top
        .nodes
        .iter()
        .filter(|n| n.is_dead)
        .map(|n| n.name.as_str())
        .collect();
    assert!(dead_vars.contains(&"unused"), "dead vars: {dead_vars:?}");
}

#[test]
fn dataflow_proc_extraction() {
    // Python test_proc_extraction: top + ::foo ⇒ ≥2 functions.
    let g = graph_for("proc foo {x} { return $x }\nset r [foo 42]");
    assert!(g.functions.len() >= 2);
}

#[test]
fn dataflow_alias_detection() {
    // Python test_alias_detection: `proc bar {} { global gvar\n set gvar 42 }`
    // registers a `gvar` alias (the global binding). The Rust memory-SSA places
    // the local name `gvar` and the Global location `gvar` in one alias set.
    let g = graph_for("proc bar {} { global gvar\n set gvar 42 }");
    let bar = g
        .functions
        .iter()
        .find(|f| f.function_name.contains("bar"))
        .expect("bar function present");
    assert!(
        bar.aliases.iter().any(|a| a.local_name == "gvar"),
        "aliases: {:?}",
        bar.aliases
    );
}

#[test]
fn dataflow_phi_edges() {
    // Python test_phi_edges: if/else both writing `a`, read after via `set b $a`
    // ⇒ ≥2 PHI edges (one per branch feeding the phi).
    let g = graph_for("if {$cond} {set a 1} else {set a 2}\nset b $a");
    let top = &g.functions[0];
    let phi_edges = top
        .edges
        .iter()
        .filter(|e| e.edge_kind == EdgeKind::Phi)
        .count();
    assert!(phi_edges >= 2, "expected ≥2 phi edges, got {phi_edges}");
}

// -- TestDataFlowGraphEdgeKinds --

#[test]
fn dataflow_alias_info_present() {
    // Python test_alias_info_present: the bar alias carries non-empty local/
    // target kinds.
    let g = graph_for("proc bar {} { global gvar\n set gvar 42 }");
    let bar = g
        .functions
        .iter()
        .find(|f| f.function_name.contains("bar"))
        .expect("bar function present");
    assert!(!bar.aliases.is_empty());
    let alias = &bar.aliases[0];
    assert!(!alias.local_kind.is_empty());
    assert!(!alias.target_kind.is_empty());
}

#[test]
fn dataflow_total_aliases_property() {
    // Python test_total_aliases_property: ≥1 alias across the module.
    let g = graph_for("proc bar {} { global gvar\n set gvar 42 }");
    assert!(g.total_aliases() >= 1);
}

#[test]
fn dataflow_multi_proc_module() {
    // Python test_multi_proc_module: top + foo + bar ⇒ ≥3 functions.
    let g = graph_for("proc foo {x} { return $x }\nproc bar {y} { return $y }");
    assert!(g.functions.len() >= 3);
}

#[test]
fn dataflow_extract_function_dataflow_directly() {
    // Python test_extract_function_dataflow_directly: build SSA + analysis for a
    // single function and call `extract_function_dataflow("::top", …)` — the
    // resulting graph is named `::top` with ≥2 defs (x, y).
    let cu = build_cu("set x 1\nset y $x");
    let fu = top_fu(&cu);
    let fg = extract_function_dataflow(
        "::top",
        &fu.ssa,
        &fu.def_use,
        Some(&fu.sccp),
        fu.memory_ssa.as_ref(),
        Some(&fu.types),
    );
    assert_eq!(fg.function_name, "::top");
    assert!(fg.total_defs >= 2);
}

#[test]
fn dataflow_prebuilt_cu() {
    // Python test_prebuilt_cu: passing a pre-built compilation unit to
    // `extract_dataflow_graph` yields the same ≥2 defs. The Rust API *only*
    // consumes pre-built per-function inputs, so this is the same path as
    // `build_graph` but spelled out: reuse one `CompilationUnit`.
    let cu = build_cu("set x 1\nset y $x").with_memory_ssa();
    let g = build_graph(&cu);
    assert!(g.total_defs() >= 2);
}

// -- TestDataFlowGraphSerialisation (extraction-shape analogues) --
//
// The Python suite's `dataflow_graph_to_dict` / `dataflow_graph_to_mermaid` /
// `_sanitise_mermaid_id` are presentation-layer helpers that have NO equivalent
// in the Rust `dataflow_graph` module (the Rust `DataFlowGraph` is the
// structural value; JSON/mermaid rendering lives in separate tooling). So the
// node/edge *shape* invariants those Python tests check are asserted here
// directly against the structural graph instead.

#[test]
fn dataflow_graph_has_nodes_and_edges() {
    // Python test_to_dict_has_nodes_and_edges: ≥2 nodes, ≥1 edge, and each node
    // exposes a name + version (the Rust `DataFlowNode.name` / `.version`).
    let g = graph_for("set x 1\nset y [expr {$x + 1}]");
    let func = &g.functions[0];
    assert!(func.nodes.len() >= 2);
    assert!(!func.edges.is_empty());
    // Every node carries a name and a (u32) version field.
    assert!(func.nodes.iter().all(|n| !n.name.is_empty()));
    let _ = func.nodes[0].version; // version is always present (a u32).
}

#[test]
fn dataflow_empty_source() {
    // Python test_empty_source: empty input ⇒ exactly one function (the top
    // level) with zero defs. (Python reports functionCount == 1; the Rust
    // module aggregator emits one `::top` function for an empty unit.)
    let g = graph_for("");
    assert_eq!(g.functions.len(), 1);
    assert_eq!(g.total_defs(), 0);
}

// ===========================================================================
// test_dce.py  →  dead_stores::liveness_dead_stores  +  DefUseChain::is_dead
//
// See the module-level note: the Rust liveness pass answers "is the value
// unread", the Python `dce_module` answers "is the assignment deletable". The
// shared, tclsh-anchored invariants (globals/side-effects kept) hold in both;
// the terminal/multi-write/param divergences are noted at each site.
// ===========================================================================

/// Variable names `liveness_dead_stores` flags as dead stores in `qname`.
fn dead_store_vars(source: &str, qname: &str) -> Vec<String> {
    let cu = build_cu(source);
    let fu = proc_fu(&cu, qname);
    liveness_dead_stores(fu, registry())
        .into_iter()
        .map(|d| d.variable)
        .collect()
}

// -- shared invariants that match Python dce_module exactly --

#[test]
fn dce_unused_literal_store_is_dead() {
    // Python test_unused_literal_store_removed: `set tmp 5` is never read, so it
    // is a pure dead store. Rust's liveness pass flags `tmp` (and, being a pure
    // literal store of a proc-local, it is also safely droppable — the case
    // where the two analyses agree). tclsh: `set tmp 5; puts hi` ⇒ prints `hi`;
    // the 5 is never observed.
    let dead = dead_store_vars("proc f {} {\n  set tmp 5\n  puts \"hi\"\n}\n", "::f");
    assert!(dead.contains(&"tmp".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_used_var_not_dead() {
    // Python test_used_var_kept: `set x 5; puts $x` — x is read, so it is NOT a
    // dead store in either pass.
    let dead = dead_store_vars("proc f {} {\n  set x 5\n  puts $x\n}\n", "::f");
    assert!(!dead.contains(&"x".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_var_used_in_braced_substitution_not_dead() {
    // Python test_var_used_in_braced_substitution_kept: `${x}` form keeps x
    // alive. The braced var-ref is a read, so x has a use and is not dead.
    let dead = dead_store_vars("proc f {} {\n  set x 5\n  puts \"value=${x}\"\n}\n", "::f");
    assert!(!dead.contains(&"x".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_var_used_in_expr_not_dead() {
    // Python test_var_used_in_expr_kept: `expr {$x + 1}` reads x ⇒ kept.
    let dead = dead_store_vars(
        "proc f {} {\n  set x 5\n  set y [expr {$x + 1}]\n  puts $y\n}\n",
        "::f",
    );
    assert!(!dead.contains(&"x".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_global_write_not_a_dead_store() {
    // Python test_global_write_kept: writing `::result` is observable from
    // outside the proc, so it must NOT be a removable dead store. Rust's
    // liveness pass suppresses globals (ns != LOCAL_NS) and does not flag it —
    // the analyses agree here, and the reason is a Tcl semantic fact.
    // tclsh: `proc f {} { set ::result hello }; f; puts $::result` ⇒ `hello`.
    let dead = dead_store_vars("proc f {} {\n  set ::result \"hello\"\n}\n", "::f");
    assert!(
        !dead.iter().any(|v| v.contains("result")),
        "global ::result must not be flagged dead: {dead:?}"
    );
}

#[test]
fn dce_command_subst_rhs_not_a_dead_store() {
    // Python TestSideEffects::test_iassign_value_with_command_subst_kept:
    // `set tmp [info commands]` — tmp is unread but the RHS has observable side
    // effects, so the store must stay. Rust's dead_stores filter explicitly
    // excludes an `AssignValue` whose value contains `[` (a command subst), so
    // `tmp` is NOT flagged — matching Python. tclsh: `set tmp [info commands]`
    // runs `info commands` regardless of whether tmp is later read.
    let dead = dead_store_vars("proc f {} {\n  set tmp [info commands]\n  puts ok\n}\n", "::f");
    assert!(
        !dead.contains(&"tmp".to_string()),
        "command-subst RHS must not be flagged dead: {dead:?}"
    );
}

#[test]
fn dce_pure_literal_assign_still_dead() {
    // Python TestSideEffects::test_pure_literal_assign_still_removed: a plain
    // `set tmp 5` (no RHS side effect) is still a dead store — the sanity check
    // that the side-effect gate didn't go too wide.
    let dead = dead_store_vars("proc f {} {\n  set tmp 5\n  puts ok\n}\n", "::f");
    assert!(dead.contains(&"tmp".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_non_terminal_dead_set_still_dead() {
    // Python TestImplicitReturn::test_non_terminal_dead_set_still_removed: in
    // `proc p {} { set tmp 5; set y 42 }`, the earlier `set tmp 5` is a dead
    // store. Rust flags `tmp`. (Rust ALSO flags `y` — see the divergence note in
    // `dce_terminal_set_is_unread_but_implicit_return` — but the Python-shared
    // invariant is that the non-terminal `tmp` is dead, which holds.)
    let dead = dead_store_vars("proc p {} {\n  set tmp 5\n  set y 42\n}\n", "::p");
    assert!(dead.contains(&"tmp".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_dead_store_inside_if_clause() {
    // Python TestNestedRecursion::test_dead_store_inside_if_clause_removed:
    // `if {1} { set y 42 }` with y unread — the liveness pass sees into the
    // branch body and flags `y`. tclsh: `if {1} { set y 42 }; puts ok` ⇒ `ok`;
    // the 42 is never read.
    let dead = dead_store_vars("proc f {} {\n  if {1} { set y 42 }\n  puts ok\n}\n", "::f");
    assert!(dead.contains(&"y".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_dead_store_inside_for_body() {
    // Python test_dead_store_inside_for_body_removed: a dead `set y 42` inside a
    // `for` body is flagged.
    let dead = dead_store_vars(
        "proc f {} {\n  for {set i 0} {$i < 3} {incr i} { set y 42 }\n  puts ok\n}\n",
        "::f",
    );
    assert!(dead.contains(&"y".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_used_var_inside_nested_if_not_dead() {
    // Python test_used_var_inside_nested_if_kept: `set y 42` inside an if AND
    // `puts $y` outside — y has a read, so it is not a dead store.
    let dead = dead_store_vars("proc f {} {\n  if {1} { set y 42 }\n  puts $y\n}\n", "::f");
    assert!(!dead.contains(&"y".to_string()), "dead: {dead:?}");
}

#[test]
fn dce_no_dead_stores_when_none() {
    // Python TestPurity::test_no_change_when_no_dead_stores: a body with no dead
    // store yields an empty dead-store set.
    let dead = dead_store_vars("proc f {} { puts \"hi\" }\n", "::f");
    assert!(dead.is_empty(), "expected no dead stores, got {dead:?}");
}

#[test]
fn dce_top_level_unused_is_dead() {
    // The top-level analogue of an unused store: `set unused 42` at module level
    // is flagged dead by the liveness pass.
    let cu = build_cu("set unused 42");
    let dead: Vec<String> = liveness_dead_stores(top_fu(&cu), registry())
        .into_iter()
        .map(|d| d.variable)
        .collect();
    assert!(dead.contains(&"unused".to_string()), "dead: {dead:?}");
}

// -- divergence-documenting tests (Rust liveness ≠ Python dce_module gate) --

#[test]
fn dce_terminal_set_is_unread_but_implicit_return() {
    // DIVERGENCE (different question, NOT a bug). Python
    // TestImplicitReturn::test_terminal_set_kept_as_implicit_return KEEPS
    // `set y 42` in `proc p {} { set y 42 }` because it is the proc's implicit
    // return value (deleting it changes the returned value). Rust's
    // `liveness_dead_stores` answers the *liveness* question — `y`'s SSA value is
    // never read by another statement — and so reports it dead. Both are
    // internally correct: the value IS unread, AND deleting the store WOULD be
    // unsound. This test pins Rust's actual (liveness) verdict and documents the
    // gap. tclsh PROVES the return semantics: `proc p {} { set y 42 }; puts [p]`
    // ⇒ `42` (set returns the stored value; it is the proc's result), so a
    // *deletion* pass must keep it — `liveness_dead_stores` is a detector, not a
    // rewriter, hence the safe divergence.
    let dead = dead_store_vars("proc p {} { set y 42 }\n", "::p");
    assert!(
        dead.contains(&"y".to_string()),
        "liveness reports the unread terminal store; dead: {dead:?}"
    );
}

#[test]
fn dce_multiple_writes_both_unread_are_dead() {
    // DIVERGENCE (different question, NOT a bug). Python
    // test_multiple_writes_kept KEEPS both `set tmp 5` and `set tmp 6` because
    // its "only write" gate declines multi-write variables. Rust's liveness pass
    // flags BOTH (`tmp#1` is overwritten before any read; `tmp#2` is never
    // read), which is the correct *liveness* verdict. tclsh: `set tmp 5; set tmp
    // 6; puts hi` ⇒ `hi`; neither 5 nor 6 is ever observed.
    let dead = dead_store_vars("proc f {} {\n  set tmp 5\n  set tmp 6\n  puts \"hi\"\n}\n", "::f");
    assert_eq!(
        dead.iter().filter(|v| *v == "tmp").count(),
        2,
        "both unread writes of tmp are dead by liveness; dead: {dead:?}"
    );
}
