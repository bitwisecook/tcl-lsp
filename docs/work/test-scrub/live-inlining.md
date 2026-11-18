# Live inlining: test scrub, 17 November 2026

Ledger: [`ledger/live-inlining.tsv`](ledger/live-inlining.tsv). 29 rows: the 23 `inline_module` tests routed by the coordinator's ruling, and the 6 failing `tcl-spectcl` it_5 `codegen_stamps` tests.

Measured: `tcl-codegen` inlining tests (273 run, 23 failed: the 22 routed rows, plus `inlining::frame::tests::actual_jim_module_purpose_cannot_open_a_logical_inline_frame`, which `execution.tsv` already holds) and the routed `tcl-vm` row. Causes were read from IR dumps of each fixture (statement kinds, `script_requires_admission`, the var-escape summary, the `inline_module` result). Every fixture was run on tclsh 8.4.20 to 9.1.0, or 8.6.18 to 9.1.0 for `try`. The six `codegen_stamps` rows were read, not run; their failures were measured earlier in the session (`docs/work/name-handling/current-state.md`, "Live inlining").

## Is `inline_module` brought under the live gate?

Not now. Only pack reference bodies are spliced. Their definition is the pack's text, proved by the pack's facts, and the VM refuses a forged or changed claim (`ReferenceBody`). A user procedure in a release unit has no such proof, and no production path adopts a release unit's procedures into a VM (guard c0ed6978). Once Option A has landed and `live_differential` matches tclsh 8.4 to 9.1, the same drop-and-reinvoke guard could cover a user procedure the live compile sees defined. That is a widening for the owner to ask for. Until then, the group (a) rows keep their cause and wait: `inliner-unshipped` and the escape and admission causes below are not live causes.

## Counts by verdict

| Verdict | Rows |
| --- | ---: |
| REWRITE | 13 |
| KEEP-CONTRACT | 8 (3 live-splice-off, 3 release-unit nested sites, 2 linked proof docs) |
| FIX-EXPECTATION | 4 |
| KEEP-ORACLE | 2 |
| FIX-FIXTURE | 1 |
| MERGE | 1 |

## Causes, ranked by tests blocked

1. `escape-pessimistic-nested-return` (5): var_escape marks a body with `return` in an `if`, or a `try` with handlers, as `DYNAMIC_BARRIER` (`walker.rs:59`, where `original_substitution_invocations` returns None because the original substitution metadata is absent). The procedure is then not `safe_to_inline`. Rows: early_return_in_if, implicit_trailing_return, try_on, try_trap, a_body_whose_every_path_binds.
2. `live-splice-off` (3): ruled, `LIVE_REFERENCE_INLINING = false` (97e7f8d5).
3. `release-unit-nested-site-unspliced` (3): the release unit splices nothing at static eval, lmap and the other nested sites. This is the dataflow area's.
4. `constant-if-body-requires-admission` (3): a procedure body holding `if {1} {…}` has `script_requires_admission` true (provider required). `rewrite_script` then returns it unchanged, so the call in the clause is never spliced. With `if {$n}` the same call is spliced.
5. `uplevel-passthrough-undetected` (2): `tcl_compiler::inline_uplevel`, which has production callers, no longer detects `proc reset {} {uplevel 1 {set counter 0}}`. The tcl-compiler unit twins fail too (analyser-binding list). These rows are misrouted: they are not `inline_module` tests.
6. `logical-module-inline-eligibility-refused` (2): `build_inlinable_map` refuses an eligible procedure in an explicit Logical module. Both tests are linked from proof documents that still state the positive case.
7. `fixture-no-source-entry` (2): the fixture is lowered with no source entry (`lower_script_module_for_bytecode(…, None, …)`, or `lower_to_ir_for_bytecode` with a profile-less registry). I inferred this and did not dump it.
8. `call-in-unreached-body` (2): `while {0}` and an `on error` handler that `noop` can never reach. Both splice once the body is reachable (verified).
9. One test each:
   - `no-call-selected-in-switch-arm`: no procedure call is selected in a switch arm, even one that matches.
   - `no-call-selected-in-for-next`: no procedure call is selected in a `for` next script, even a reachable one.
   - `expr-cmd-subst-operand-not-spliced`: `expr {[string length abc] + $x}` stays a call.
   - `call-by-name-cmd-subst-not-collected`: interprocedural, dataflow area.
   - `c8-unicode-varname-expectation`: tclsh contradicts the test's expectation.
   - `fixture-uplevel-1-at-global-level`: tclsh raises `bad level "1"`.
   - `inliner-unshipped`: the tcl-vm row.

Only one row ends up with the ruling's default cause `inliner-unshipped`. Every other row fails for a reason I could name. Most of them are lowering or analysis gates upstream of an inliner that nothing has exercised since it lost its production caller.

## Distinct failing behaviours (worth fixing)

- No wrong answers. Every failure is a missed splice or a refusal, which is always semantics-safe, except the oracle-contradicted expectation.
- var_escape is pessimistic for a body with a nested `return` or a `try` handler. Missing substitution metadata becomes `DYNAMIC_BARRIER`.
- A procedure body that holds a constant `if {1}` requires native compilation admission, which blocks any rewrite of the body.
- No procedure call is selected in a switch arm or in a `for` next script.
- `inline_uplevel` static passthrough detection is lost. This is the only behaviour here on a production path (CompilationUnit build).
- A Logical-module inline eligibility positive is refused (proof-document contract).
- The release unit splices nothing at nested sites (static eval, lmap, and others).
- Live splicing is off (ruled).
- Fixture faults:
  - `uplevel 1` at global level.
  - Calls in unreachable bodies.
  - A comment citing `f 0` while the fixture calls `f 5`.
  - A `try` comment citing an `on ok` variant the fixture does not run.
  - Modules with no source entry.
- An expectation contradicted by the oracle: tclsh 8.5 to 9.1 read `$café` as `$caf` followed by `é`.

## Splice matrix: can it be smaller?

The matrix is not 132 cells. It runs 21 shapes × 23 sites = 483 cells, each for n in {-5, 0, 7}, which is 1,449 reference-and-inlined VM pairs. That is why it takes about 20 minutes. The 132 is the splice-count bound `SPLICED_SHAPES.len() * 6`, measured when there were 22 shapes. With the 21 in the tree the bound is 126.

### Axes

Shapes, grouped by the splice path they take:

- **R1, trailing return:** vdouble, vconst (nullary), vlocal (local def), vinc (incr), vpair (two formals)
- **R2, last command's value:** vtail, vnoop (empty), vlit (braced `$x`)
- **R3, early and trailing return:** vabs, vretbs (braced backslash)
- **R4, early return, then the last command's value:** vsign, vsetbs, vsetbr (braced brackets)
- **R5, value from a branch with no return:** vif
- **R6, loop in the body:** vloop
- **R7, return of a literal or substituted word:** vbrace, vsubst
- **D, declines** (a command substitution names a variable, so it stays a call): vsetcmd, vincrcmd, vfmtcmd, vexprcmd

Sites, grouped by how `rewrite_stmt` treats the call:

- **Returned tail:** terminal, after a set
- **Dropped:** dropped before a return, dropped before a last set
- **If arm with tail:** then arm, else arm
- **Switch arm:** switch arm
- **Block, tail and dropped:** static eval; static eval, then more
- **Held value:** catch result, catch code, try ok, try terminal
- **Handler:** try handler
- **Loop body, dropped:** loop body, while body
- **Substitution:** in a substitution
- **Collector:** lmap body, lmap body after a set, dict map body
- **Frame:** uplevel 0
- **Counter:** twice
- **Names:** caller with the bodies' names

### Redundant cells (same shape class, same site class)

- terminal and after a set. Keep both only if `Site.first` matters. It is only a flag for the first statement, so one non-first cell is enough.
- dropped before a return and dropped before a last set.
- then arm and else arm.
- catch result and catch code. None of the shapes raises, so the code is always 0.
- try ok and try terminal: both are the `try` body with its value held.
- loop body and while body: both go through `rewrite_single_body_stmt`.
- lmap body and lmap body after a set. dict map is the same collector class through another command, so keep it only as a one-shape check.
- The four D shapes stay calls at every site. 4 × 23 × 3 runs compare a call with itself. One site each is enough, and `inlining::tests::a_body_whose_substituted_command_names_a_variable_stays_a_call` and `spliced_bodies::a_variable_a_substituted_command_names_is_the_definitions_own` already hold them.
- Within R1, R2 and R7 the variants test binding and rename forms: formals, locals, `incr`, literals. They do not depend on the site, so one tail site, one dropped site and the names site cover them.

### Proposed minimal matrix

- **Routing grid:** one representative per routing class, {vdouble, vtail, vabs, vsign, vif, vloop, vnoop}, at one site per site class: {terminal, dropped before a return, then arm, switch arm, static eval, static eval then more, catch result, try ok, try handler, loop body, in a substitution, lmap body, dict map body, uplevel 0, twice}. That is 7 × 15 = 105 cells.
- **Binding and literal forms:** {vconst, vlocal, vinc, vpair, vlit, vbrace, vsubst, vretbs, vsetbs, vsetbr} at {terminal, dropped before a return, caller with the bodies' names}. That is 30 cells. vretbs, vsetbs and vsetbr also go at catch result, because their literal passes through the wrap where a value is held: 3 more.
- **Declines:** {vsetcmd, vincrcmd, vfmtcmd, vexprcmd} at terminal. That is 4 cells.

Total: 142 cells, or 426 runs instead of 1,449. That is about a 3.4× cut, roughly 6 minutes, under terminate-after.

The splice-count bound then has to be stated per cell class instead of as `shapes * 6`. The cleanest form is an expected spliced or kept verdict on each site class. That also turns the count failure into a list of the nested sites the release unit does not splice: loop, while, switch, static eval, lmap, dict map, catch, try, substitution and uplevel. Those are what dataflow owns.

To split it further, the 15 site classes could each become one `#[test]` over the 7 routing shapes. That gives parallel runs and a named failure per site class.

## Switching live splicing on: the case for the owner

**Status:** blocked on the owner. The switch commit and the guard-test change below need the owner's approval; this session's permission check refused the guard rewrite as weakening an oversight check, which is right while the approval is the owner's to give.

### What the switch does

`tcl_codegen::compile_service::LIVE_REFERENCE_INLINING` turns on splicing of pack reference bodies (`runtime_backing tcl-body`) into the VM's own compile of a calling procedure (`compile_procedure_bytes_target_with` under a live native entry). Today it is `false` and nothing is spliced on any production path. With it on, a call whose binding the live entry proves to reach a pack's Tcl body is replaced by that body, under these conditions:

- The body is proved re-executable from its start (`command_binding::re_executable_body`) both as a fresh interpreter reads it and as the caller's live entry lowers it: every command reaches one registry implementation whose declared effects stay in the frame, no callback but an untraced variable access, and every `expr` runs only its arithmetic at its site (`ExpressionMathBindings::runs_only_its_arithmetic`).
- The call stands directly in the caller's body (operand stack empty), its words are evaluated once into anonymous slots, and the body's locals are anonymous slots no name reaches.
- The body is bracketed as a `SpliceRange` and followed by the call itself made from those slots. On a failed guard or any abnormal completion inside the range the VM resumes at that call, so errors, codes, `-errorstack`, traces and introspection are the uninlined call's.
- A function whose range cannot be formed exactly is compiled without inlining. 8.4 splices nothing.

### The gate: `tcl-spectcl` it_5 `codegen_stamps::live_differential`

Compared with tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 for every row: the result, `errorInfo`, `errorCode`, `-errorstack`, `info errorstack`, and how many definitions the live compile spliced. Rows:

- shapes: no parameters, one, two, a default, `args`, a parameter run as the command, an error raised inside the body (`format`, `expr`, wrong arity), `return -code error`, `break`, `continue`, `error` with custom info, `return -level 2`;
- `expr` at its site: plain operands spliced; a math function, a redefined `::tcl::mathfunc::abs`, a command substitution and a traced global kept as calls;
- frame: `info locals`, `info vars` and `uplevel 1 {info locals}` after the call; caller variables spelled as the splice's hidden names, directly and through a computed name;
- traces: execution `enter`, `leave`, `enterstep` and `leavestep` added between two calls (the splice falls back and the trace fires as tclsh's), `enter`, `leave` and `enterstep` set before the caller compiles (kept as a call), a read trace on the word's variable (kept);
- words with effects (`[incr n]`), kept as calls;
- structured control (`if`, an error in an arm, `while`, `for`, `foreach`, `if` renamed between two calls), kept as calls until the reading of a reference body's structured control lands.

With the switch on, every row is identical to tclsh on every release.

### Measurement, switch off against on (work at 2e8b3814)

| Package | Failing, off | Failing, on |
| --- | ---: | ---: |
| tcl-codegen (1,487) | 110 | 111 |
| tcl-spectcl (622) | 20 + 1 timeout | 17 + 1 timeout |
| tcl-vm (3,209) | 80 | 80 |
| tcl-engine-api (9) | 0 | 0 |
| tcl-engine-tclvm (84) | 7 | 7 |
| tcl-engine-wasm (54) | 3 | 3 |
| tcl-cshim (123) | 14 | 14 |
| tcl-spec-hooks (77) | 3 | 3 |

Test by test: the only new failure is `tcl-codegen` it_5 `inliner_containment::the_live_compile_splices_no_reference_body`, the guard that asserts the switch is off (below). Fixed: `reference_bodies::a_tcl_body_backed_command_is_inlined_and_admitted`, `a_pack_text_body_that_diverges_turns_the_site_plain`, `a_host_native_backing_never_defines_a_proc`. Every other failing set is identical. `runtime/rust` depends on neither `tcl-codegen` nor `tcl-vm`.

Repeated on 18 November at 1a99ba86 with every oracle required (`TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1`, Jim built by `scripts/dev/ensure-jim-oracle.sh`):

| Package | Failing, off | Failing, on |
| --- | ---: | ---: |
| tcl-codegen (1,488) | 110 | 111 |
| tcl-spectcl (625) | 21 + 1 timeout | 18 + 1 timeout |

The same test-by-test result: the guard is the only new failure, and the three `reference_bodies` tests are fixed.

**Jim.** `live_differential` does not compare against Jim: it runs tclsh 8.4 to 9.1 only. A Jim unit splices nothing, so there is nothing to compare. `build_inlinable_map` (`tcl-codegen/src/inlining/mod.rs`) returns no inlinable body unless the module's parameter grammar is Tcl's, and the live import's reference spec is built through it.

### Fuzz smoke

`tcl-fuzz run --iterations 2000 --seed 90000`, `tclvm` subject against tclsh 9.0.4, the same seeds with the switch off and on: 2000 of 2000 matched both ways, no findings either way. This shows the switch harmless where no pack is loaded, not that splicing is fuzz-clean: the `tclvm` CLI loads no spec pack, so no generated script reaches a reference body. A fuzz arm over a pack with reference bodies does not exist yet. (The switch-on binary was rebuilt while the reference-body structured-control option was being added to `tcl-compiler`; that option acts only when a reference body is lowered, which the CLI never does.)

### What stays declined

A call whose word writes a variable (until analyser-binding's trace-free-local proof); a body wrapped for an early return; structured control in the body (until its reading lands); a math function or a list command on a value the live entry cannot see; any command with a callback, a frame or state effect, or a foreign variable; a call not directly in the caller's body; a caller in another namespace than the definition; 8.4. `info cmdcount` is not implemented by the VM on any release (execution's); when it is, a splice counts as the call it replaces.

### Replacement text for the guard test

In `rust/tcl-codegen/tests/it_5/inliner_containment.rs`, replace the module doc, `REMOVAL` and `the_live_compile_splices_no_reference_body` with:

```rust
//! Inlined procedure bodies reach a shipped compile only through the live
//! splice, whose gate is the live differential.
//!
//! A spliced body runs in its caller's frame; it presents as the procedure
//! call does only because its locals are anonymous slots and any abnormal exit
//! from its range falls back to the call itself. The live differential
//! (`tcl-spectcl` `codegen_stamps::live_differential`) holds that against
//! tclsh on every release. The release unit's procedure inliner
//! (`inlining::inline_module`) has no such gate and reaches no shipped compile.

const REMOVAL: &str = "the release unit's procedure inliner reaches a shipped compile \
    without a differential that compares errorInfo, errorCode, the errorstack, frame \
    introspection and traces with tclsh on every release";

/// What the live differential compares with tclsh for every spliced shape:
/// what a script can observe of a call that a splice could change.
const OBSERVED: &[&str] = &[
    "$::errorInfo",
    "$::errorCode",
    "-errorstack",
    "info errorstack",
    "info locals",
    "info vars",
    "uplevel 1",
    "trace add execution",
    "trace add variable",
    "TclVersion::ALL",
];

#[test]
fn the_live_compile_is_gated_by_the_live_differential() {
    let gate = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tcl-spectcl/tests/it_5/codegen_stamps/live_differential.rs");
    let text = std::fs::read_to_string(&gate).expect("the live differential");
    let missing: Vec<_> = OBSERVED
        .iter()
        .filter(|observed| !text.contains(**observed))
        .collect();
    assert!(
        missing.is_empty(),
        "live splicing ships only behind a differential comparing every observable: \
         {missing:?} missing from {}",
        gate.display()
    );
}
```

`no_production_source_calls_the_procedure_inliner` stays as it is. The switch commit itself changes only `LIVE_REFERENCE_INLINING` to `true`, names `live_differential` as the gate and lists what stays declined.
