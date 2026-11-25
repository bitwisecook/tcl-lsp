# Script parameters: procedures that run an argument as a script

A procedure that runs one of its arguments as a script (`proc t {label script} {uplevel 1 $script}`) hides that script from analysis today. A typo, a bad option, an unknown command or a variable read inside the argument is never reported, because the call site sees a plain string argument to a user procedure. This page designs the inference that lets the call site analyse the argument as a body. It is the caller-frame-script slice of item 55 (inferred contracts), raised by item 210 and the [overrides review](overrides-package-review.md).

**Question and method:** where does Tcl run a script passed to a wrapper procedure, what does the analyser already infer about such parameters, and what is missing between the inference and the call site? The frame rule was measured under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. The inference was read from `rust/tcl-compiler/src/analyser/param_traits.rs` and its consumers.

**Verdict:** the analyser already infers that a parameter is run as a script (`analyser::types::ProcArgTrait::Eval` and `Body`, from `param_traits.rs`), but nothing outside tests reads either trait. No call site analyses the argument, and the trait carries no frame, so a consumer could not place the script even if it tried. The work is a frame and timing fact on the trait, one call-site consumer that analyses a literal argument as a body unit in that frame, and a merge of the two `ProcArgTrait` enums.

## Measured

One fixture, run unchanged under every release, all of which agree:

```tcl
proc t {label script} {if {[catch {uplevel 1 $script} r]} {return "ERR $r"} else {return $r}}
proc p {} {t a {set x 1}; return $x}          ;# 1: the script ran in p's frame
proc e {s} {eval $s}
proc q {} {e {set y 2}; info exists y}         ;# 0: eval runs in e's own frame
proc g {s} {uplevel #0 $s}
proc r {} {g {set z 3}; list [info exists z] [info exists ::z]}  ;# 0 1: global frame
```

tclsh 8.4.20 through 9.1.0 print `1 0 0 1`. So:

- `uplevel 1 $p` (and `uplevel $p`) runs the argument in the **caller's** frame.
- `eval $p`, `catch $p` and `if 1 $p` run it in the **procedure's own** frame, where the caller's locals do not exist.
- `uplevel #0 $p` and `namespace eval :: $p` run it in the **global** frame.
- `uplevel 2 $p` runs it two frames up, which only the call graph can name.
- `after 0 $p` and `chan event $c readable $p` run it **later** in the global frame (item 169's future bodies).

`chan/c05`'s `t bad-dir {… close $f sideways …}` is this shape. tclsh rejects `sideways`, but the analyser reports nothing, because the script is an argument to `t`.

## Census

- `rust/tcl-compiler/src/analyser/param_traits.rs` infers `ProcArgTrait::Eval` (the parameter reaches `eval`, `uplevel` or `subst`) and `ProcArgTrait::Body` (it reaches a loop or control body). The inference is shallow and deep, and forwards through other procedures. It records **no level**, so `eval $s`, `uplevel 1 $s` and `uplevel #0 $s` are indistinguishable.
- `ProcDef::caller_frame_params` records the missing level fact, but only for `upvar 1`. `tcl-lsp-core/src/caller_frame.rs` uses it for call-by-name variables, which is the precedent this design follows.
- Nothing outside tests reads `Eval` or `Body`. `project_procedure_roles.rs`, semantic tokens and `caller_frame.rs` read only the variable and command traits.
- A second enum, `interprocedural::ProcArgTrait` (passthrough, condition, forwarded, variable read and write, unused), duplicates part of the analyser's. Item 55 makes both projections of one contract.
- Registry commands already express the same fact for built-ins: an `ArgRole::Body` position with an `EvaluationContext { frame, timing }`. `catch`, `uplevel`, `after`, `thread::send` and the rest are analysed this way.

## Design

1. **The fact.** A procedure formal gets a script contract `ScriptParameter { frame, timing }`. `frame` is `Caller(level)`, `Own`, `Global` or `Unknown`. `timing` is `Now` or `Deferred`. It is derived from the body's own uses, by the registry descriptor of the command the parameter reaches:
   - `uplevel 1 $p` and `uplevel $p` give `Caller(1)`;
   - `eval $p`, `catch $p` and `if … $p` give `Own`;
   - `uplevel #0 $p` gives `Global`;
   - `after … $p` gives `Global` and `Deferred`.

   A parameter forwarded to another procedure's script parameter composes the two: `Caller(1)` through `Caller(1)` is `Caller(2)`. A parameter that reaches two different frames, or is modified before it is run, gets `Unknown` and abstains.
2. **One shape with the registry.** A call to a procedure whose formal `i` has a contract gets the same view a registry command gets: argument `i` is an `ArgRole::Body` with an `EvaluationContext` built from the contract. `Caller(1)` is the calling frame; `Own` is a fresh frame with no locals; `Global` is the global frame; `Deferred` is a future body as in item 169. The analyser's existing body-role paths then analyse the argument, so diagnostics, references, rename and frame effects follow without a new walker.
3. **Selecting the callee.** The binding owner's selected target at the call site is the procedure whose contract applies. The call only qualifies when it has one proved target with no rename or redefinition between declaration and call. An alias, `{*}` expansion, a variadic `args` formal that absorbs the argument, or a defaulted formal whose position the call does not fix all abstain.
4. **Only literal arguments.** A braced or otherwise literal argument is analysed as source. A substituted argument (`t a $script`) is left alone, as the registry does for computed bodies.
5. **Frame effects.** Writes and reads inside a `Caller(1)` argument are writes and reads in the calling frame. They feed item 22's frame-effect model, so `t a {set x 1}; puts $x` stops being a candidate W210. `Own` and `Global` writes do not reach the caller's locals.
6. **One trait type.** `analyser::types::ProcArgTrait::{Eval, Body}` become `ScriptParameter` contracts, and `interprocedural::ProcArgTrait` is projected from the same per-formal contract (item 55).

## Plan

1. Add the frame and timing to the inferred trait, keeping the current shallow and deep inference and its tests. Unit tests: the five shapes measured above, plus a forwarded `Caller(1)` through `Caller(1)`.
2. Project a proc call with a contract into the invocation's role layout as `ArgRole::Body` with the contract's evaluation context, behind the binding owner's single-target proof.
3. Conformance rows: `chan/c05` (`close $f sideways` reported once item 210's `close` direction check exists), a new `dynamic/` fixture with the five measured shapes, and the `external-overrides/` typo from item 55.
4. Feed `Caller(1)` writes to item 22's frame effects; check `dynamic/e04` and the new fixture for W210.
5. Merge the two `ProcArgTrait` enums under item 55's contract.

Done when the new fixture's `Caller(1)` script reports an unknown command and a bad option inside the argument, its `Own` script does not see the caller's locals, its `Global` script writes `::z`, and `chan/c05` reports `sideways` under 8.6 and 9.0 once item 210's check exists.
