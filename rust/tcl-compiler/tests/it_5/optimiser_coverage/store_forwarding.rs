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

//! O102: forwarding a stored value to its read, and what blocks it: traces, `upvar` aliases, `uplevel #0` and interpreter barriers.

use super::*;

// O102 — fold [expr {...}] command substitution

#[test]
fn o102_expr_command_substitution() {
    // tclsh: 3+4 == 7. A *pure-literal* expr subst is folded by the constant
    // folder (O101); O102 is reserved for an expr that consumed a *propagated*
    // constant. Both produce the same value.
    assert!(optimised("set v [expr {3 + 4}]", TCL).contains("set v 7"));
    assert!(opt_fires("set v [expr {3 + 4}]", TCL, "O101"));

    // Propagated constant into the expr subst ⇒ O102 (+ O100 propagation). tclsh:
    // a=5 ⇒ 5*2 == 10.
    let prop = "set a 5\nset v [expr {$a * 2}]";
    assert!(optimised(prop, TCL).contains("set v 10"));
    assert!(opt_fires(prop, TCL, "O102"));
    assert!(opt_fires(prop, TCL, "O100"));

    // Dynamic expr ([clock seconds]) does not fold via O102.
    assert!(opt_absent(
        "set v [expr {[clock seconds] + 1}]",
        TCL,
        "O102"
    ));
}

// O102 — variable-trace / aliasing / frame-crossing safety
//
// Regression coverage for a confirmed silent miscompile: `run_load_forwarding`
// (O102) and its SCCP-sourced sibling paths (O100/O101/O103, sharing the same
// `constants` map — see `propagation::run_function`) forwarded a variable's
// literal or SCCP-derived value into a later read with no notion that a
// `trace add variable` handler can run arbitrary code on read/write, that a
// write handler can rewrite the stored value, or that the value is aliased
// into another frame (upvar/global/variable) a called proc could write
// through. Each `tclsh:` value was checked against `tclsh` 8.6.14 — trace,
// `interp eval`, `uplevel`, and `unset` semantics are unchanged across
// 8.4-9.0, so the same value holds under every dialect this suite targets.

#[test]
fn o102_read_trace_via_helper_proc_blocks_forward() {
    // FN-would-be-TP-if-safe / actual TP for the fix: a read trace
    // installed by a *called* proc — not lexically between the def and
    // the use — must still block forwarding. Before the fix this
    // rewrote to `puts 5`, silently dropping the trace's `puts "read
    // trace fired"` side effect. tclsh: prints "read trace fired" then
    // "5" — `puts $x` must survive verbatim so the trace still fires at
    // runtime.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nsetup\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_read_trace_installed_before_def_blocks_forward() {
    // The trace-install call can precede the `set` textually too — no
    // statement sits *between* the def and the use, so a same-block
    // intervening scan alone would miss this; the module-wide
    // `traced_variables` fact (position-independent, mirroring
    // `traced_commands`) is what catches it.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nsetup\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
}

#[test]
fn o102_write_trace_mutating_value_blocks_forward() {
    // TP for the fix: a write-trace handler can rewrite the value being
    // stored, so the literal text at the `set` is not the runtime value.
    // tclsh: the handler increments on every write, so `x` ends up `6`,
    // not the literal `5` written in source.
    let src = "proc onwrite {name1 name2 op} {\n    upvar 1 $name1 v\n    set v [expr {$v + 1}]\n}\ntrace add variable x write onwrite\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_legacy_trace_variable_form_blocks_forward() {
    // The deprecated `trace variable name ops command` spelling (8.4-8.6
    // only) must gate the forward exactly like the modern `trace add
    // variable` form — no per-form gap in the registry-driven fact.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\ntrace variable x r onread\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
}

#[test]
fn o102_abbreviated_trace_add_variable_blocks_forward() {
    // C Tcl's `Tcl_GetIndexFromObj` accepts a unique prefix of the trace
    // type word, so `trace add var x read h` installs the same variable
    // trace as the full `variable` spelling (tclsh 8.6.14: prints "read
    // trace fired" then "5"). The registry's `arg_role_resolver` for
    // `trace add`/`remove` must not hand-match only the literal word
    // `"variable"`: doing so would record no `traced_variables` fact for
    // this abbreviated form and still forward to `puts 5`.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\ntrace add var x read onread\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_transitive_trace_taint_through_expr_blocks_forward() {
    // TP for the deeper, transitive fix: `a` carries a read trace, and
    // `v`'s SCCP-constant value is *computed from* `a` inside
    // `[expr {...}]}`. Filtering only the directly-traced name (`a`)
    // is not enough — `v` must inherit the taint through the SSA def/use
    // graph, or `set v [expr {$a * 2}]` still folds to `set v 10` and
    // `puts $v` still forwards, dropping the read trace exactly as
    // before. tclsh: prints "read trace fired" then "10".
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\nproc setup {} {\n    trace add variable ::a read onread\n}\nset a 5\nsetup\nset v [expr {$a * 2}]\nputs $v\n";
    assert!(opt_absent(src, TCL, "O100"));
    assert!(opt_absent(src, TCL, "O101"));
    assert!(opt_absent(src, TCL, "O102"));
    let out = optimised(src, TCL);
    assert!(out.contains("set v [expr {$a * 2}]"));
    assert!(out.contains("puts $v"));
}

#[test]
fn o102_interp_eval_self_target_barrier_blocks_forward() {
    // TP: `interp eval {}` targets *this* interpreter (the empty path is
    // the self-reference), so its body runs in the caller's own global
    // scope — unlike a named sub-interpreter, it really can rewrite `x`.
    // tclsh: prints `99`, not the literal `5` written earlier.
    let src = "set x 5\ninterp eval {} {\n    set x 99\n}\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_sub_interpreter_isolation_no_longer_over_blocked() {
    // Precision guard: a *named* sub-interpreter's variables are isolated
    // from the caller's — `interp eval slave {set x 99}` does not touch
    // the master's `x` (tclsh: prints `99` inside the slave block, then
    // `5` for the master, unaffected) — and `interp eval` with a named
    // (non-self) target is not a `Statement::Barrier`/`UpFrame`, so
    // `has_intervening_barrier` correctly does not gate on it.
    // `run_load_forwarding` must gate on a real frame-crossing barrier,
    // not decline unconditionally on any impure intervening call — see
    // `o102_side_effecting_intervening_call_is_still_forwarded` for the
    // general case this generalises. The nested literal body's own `$x`
    // reference (lexically inside the slave's script, a separate
    // namespace) still only earns a hint (not a precise applicable
    // rewrite), so the source is unchanged either way — this test only
    // locks in that O102 does not unconditionally decline here.
    let src = "set x 5\ninterp create slave\ninterp eval slave {\n    set x 99\n    puts $x\n}\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_upvar_aliased_variable_blocks_forward() {
    // TP: `x` is written through an `upvar` alias from a called proc — a
    // memory-SSA-aliased name, so a call between a literal def and a
    // later use can rewrite it without a new SSA version appearing in
    // *this* function. (Here the literal `set x 5` is genuinely dead —
    // O109 removes it — but O102 must not offer to forward its value.)
    let src = "proc mutate {} {\n    upvar 1 x v\n    set v 99\n}\nset x 5\nmutate\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
}

#[test]
fn o102_upvar_dynamic_local_alias_blocks_forward() {
    // TP: `upvar 1 x $dst` has a dynamic *local* side, so the
    // per-local alias maps cannot key it — but the caller-side name is the
    // literal `x`, and the callee really does rewrite it. tclsh 9.0.4:
    // prints `2`, so folding the later `$x` read to the stale `1` would be
    // a silent miscompile.
    let src = "proc p {dst} {\n    upvar 1 x $dst\n    set $dst 2\n}\nproc c {} {\n    set x 1\n    p x\n    puts $x\n}\nc\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 1"),
        "the caller's x must not fold across `p x`: {out}"
    );
}

#[test]
fn o102_upvar_dynamic_local_alias_param_source_blocks_forward() {
    // TP: `upvar 1 $name $dst` — both the caller-side name
    // (the value of `name`) and the local key are call-site data, but the
    // caller-side name IS resolvable from the argument. tclsh 9.0.4:
    // prints `9`.
    let src = "proc r {name dst} {\n    upvar 1 $name $dst\n    set $dst 9\n}\nproc c3 {} {\n    set t 0\n    r t alias\n    puts $t\n}\nc3\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 0"),
        "the caller's t must not fold across `r t alias`: {out}"
    );
}

#[test]
fn o102_upvar_dynamic_local_alias_does_not_block_unrelated_forward() {
    // FP guard: the widening is per-name — an unrelated
    // local `y` in the same caller must still forward. tclsh 9.0.4: prints
    // `5` then `2`.
    let src = "proc p {dst} {\n    upvar 1 x $dst\n    set $dst 2\n}\nproc c4 {} {\n    set x 1\n    set y 5\n    p x\n    puts $y\n    puts $x\n}\nc4\n";
    let out = optimised(src, TCL);
    assert!(
        out.contains("puts 5") || out.contains("puts $y"),
        "y is untouched by p and may fold: {out}"
    );
    assert!(
        !out.contains("puts 1"),
        "x must still not fold across the call: {out}"
    );
}

#[test]
fn o102_upvar_zero_dynamic_local_is_callee_own_frame_and_still_forwards() {
    // TN: `upvar 0 own $dst` aliases the *callee's own*
    // frame — no caller effect, so the caller's constant still forwards.
    // tclsh 9.0.4: prints `3` (the caller's x is untouched).
    let src = "proc q {dst} {\n    upvar 0 own $dst\n    set $dst 7\n}\nproc c2 {} {\n    set x 3\n    q alias\n    puts $x\n}\nc2\n";
    let out = optimised(src, TCL);
    assert!(
        out.contains("puts 3") || out.contains("puts $x"),
        "an own-frame alias must not block the caller's forward: {out}"
    );
    assert!(!out.contains("puts 7"), "never the callee's value: {out}");
}

#[test]
fn o102_pure_intervening_call_is_still_forwarded() {
    // TN / precision guard: the fix must not be so conservative that a
    // provably pure intervening call (no side effects, registry `pure:
    // true`) blocks the forward. `string length` is pure — it cannot
    // write `x` — so it must not gate the same-block scan.
    let src = "set x 5\nstring length hi\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o102_side_effecting_intervening_call_is_still_forwarded() {
    // TN / precision guard: `puts` genuinely carries a side effect
    // (`SideEffectTarget::FileIo`), but it has no way to *write* `x` —
    // Tcl's frame-based scoping means an intervening call can only touch
    // `x` via an alias (`global`/`variable`/`upvar`), a trace, or a
    // frame-crossing `uplevel`/`interp eval` body, all of which
    // `run_load_forwarding` checks directly (see its doc comment) rather
    // than gating on every impure intervening statement. Neither applies
    // here, so O102 fires the same as the provably-pure case above.
    let src = "set x 5\nputs \"start\"\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o102_unregistered_proc_call_between_def_and_use_still_forwards() {
    // TN / precision guard: a call to a proc the registry knows nothing
    // about cannot reach a caller's private, unaliased local (`x` is
    // never `global`/`variable`/`upvar` declared, and `helper` contains
    // no barrier), so the forward is sound — mirrors
    // `o102_still_forwards_top_level_global_no_proc_touches` in
    // propagation.rs's own unit tests, which locks in the same precision
    // at the whole-module-scan level.
    let src = "proc helper {} {\n    # opaque to the registry\n}\nset x 5\nhelper\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o102_uplevel_hash_zero_in_called_proc_kills_the_stale_global() {
    // TP: `uplevel #0 {set x 99}` inside a *called* proc rewrites the
    // caller-visible global, so the earlier literal must not forward
    // across the call. tclsh 9.0.3/9.0.4: prints `99`; emitting `puts 5`
    // would be wrong. The callee's global-frame write is part of its
    // `GlobalWriteInfo` summary and widens the call site's defs.
    let src = "proc setter {} {\n    uplevel #0 {\n        set x 99\n    }\n}\nset x 5\nsetter\nputs $x\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 5"),
        "the stale global 5 must not forward across `setter`: {out}"
    );
    assert!(out.contains("puts $x"), "the read must survive: {out}");
}

#[test]
fn o102_uplevel_hash_zero_list_built_script_kills_the_stale_global() {
    // TP — the list-built spelling: `uplevel #0 [list set k
    // 77]` names its command and target statically. tclsh 9.0.4: prints
    // `77`.
    let src = "proc s3 {} {\n    uplevel #0 [list set k 77]\n}\nset k 5\ns3\nputs $k\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 5"),
        "the stale global 5 must not forward across `s3`: {out}"
    );
}

#[test]
fn o102_alias_to_uplevel_uses_its_prepended_caller_level() {
    // Tcl 9.0.4: `write_caller` executes `uplevel 1 [list ::set x 99]`,
    // so `setter` writes x in its caller's frame. The source head is not a
    // registry command; its alias target and prepended level are.
    let src = "interp alias {} write_caller {} uplevel 1\n\
               proc setter {} { write_caller [list ::set x 99] }\n\
               set x 5\n\
               setter\n\
               puts $x\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 5"),
        "the stale caller-frame value must not forward across `setter`: {out}"
    );
}

#[test]
fn o102_uplevel_hash_zero_through_a_call_chain_kills_the_stale_global() {
    // TP — nested call chain: `outer` calls `setter`; the
    // global-frame write is transitive through the direct-call closure.
    // tclsh 9.0.4: prints `99`.
    let src = "proc setter {} {\n    uplevel #0 {\n        set g2 99\n    }\n}\nproc outer {} {\n    setter\n}\nset g2 5\nouter\nputs $g2\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 5"),
        "the stale global 5 must not forward across `outer`: {out}"
    );
}

#[test]
fn o102_uplevel_hash_zero_dynamic_script_widens_every_global() {
    // TP — `uplevel #0 $body` is an unreadable script at the
    // global frame: no name is enumerable, so the call site must widen to
    // an opaque barrier. tclsh 9.0.4: prints `42`.
    let src = "proc s2 {body} {\n    uplevel #0 $body\n}\nset h 5\ns2 {set h 42}\nputs $h\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 5"),
        "the stale global 5 must not forward across `s2`: {out}"
    );
}

#[test]
fn o102_uplevel_hash_zero_embedded_substitution_widens_too() {
    // TP — the same callee reached through a command
    // substitution (`set y [s2 …]`) instead of a bare statement.
    let src =
        "proc s2 {body} {\n    uplevel #0 $body\n}\nset h 5\nset y [s2 {set h 42}]\nputs $h\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 5"),
        "the stale global 5 must not forward across `[s2 …]`: {out}"
    );
}

#[test]
fn o102_uplevel_hash_zero_unrelated_global_still_forwards() {
    // FP guard — a *literal* global-frame write is per-name:
    // `setter` writes only `x`, so the untouched `y` still forwards.
    // tclsh 9.0.4: prints `7` then `99`.
    let src = "proc setter {} {\n    uplevel #0 {\n        set x 99\n    }\n}\nset x 5\nset y 7\nsetter\nputs $y\nputs $x\n";
    let out = optimised(src, TCL);
    assert!(
        out.contains("puts 7") || out.contains("puts $y"),
        "y is untouched by setter and may fold: {out}"
    );
    assert!(!out.contains("puts 5"), "x must not fold: {out}");
}

#[test]
fn o102_uplevel_hash_zero_non_writing_script_still_forwards() {
    // TN — a global-frame script that writes no variable
    // (`uplevel #0 [list puts hi]`) has no global effect at all, so the
    // caller's constant still forwards. tclsh 9.0.4: prints `hi` then `5`.
    let src = "proc shout {} {\n    uplevel #0 [list puts hi]\n}\nset x 5\nshout\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o102_alias_baked_set_target_kills_the_stale_global() {
    // Tcl 9.0.4: the alias contributes `x`, so `put 99` executes `set x 99`
    // in the target interpreter and `p` prints 99.  The compiler must project
    // that prepended variable name through the registry-owned `set` layout.
    let src = "interp alias {} put {} set x\n\
               proc p {} { global x; put 99 }\n\
               set x 5\n\
               p\n\
               puts $x\n";
    let out = optimised(src, TCL);
    assert!(
        !out.contains("puts 5"),
        "the stale global 5 must not forward across the aliased set: {out}"
    );
    assert!(
        out.contains("puts $x"),
        "the runtime read must survive: {out}"
    );
}
