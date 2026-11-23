# Jim Tcl: what the registry, the analyser and the engines model

How Jim Tcl's own semantics are covered: references and garbage collection (`ref`, `getref`, `setref`, `collect`, `finalize`), `local`, `upcall`, `$(expr)` and `$d(key)` sugar, `lambda`, `alias`, `curry`, procedure statics, `defer`, `stackdump`, `exists`, `lmap`, `info`, error and return differences, and Jim namespaces. The 23 fixtures in [`oo-dispatch-probe/jim/`](handover/oo-dispatch-probe/jim/) each isolate one feature. They were run under jimsh 0.84 (`tmp/jimtcl-oracle/jimsh`, built with namespaces), tclsh 8.6 and 9.0, and the VM and runtime in 9.0 mode (`engines.tsv`). The `mcp__tcl-lsp__*` tools offer no Jim dialect, so analysis was driven through `target/release/tcl-lsp-server` (built 2026-11-22) over JSON-RPC with the dialect set to `jimtcl`, and to `tcl9.0` for comparison: diagnostics (`analyser-mcp.out`), and hover, definition and references at Jim-specific sites (`navigation.out`).

**Verdict:** Jim is modelled as data and grammar, partly as analysis, and not as a runnable engine:

- The lexer handles Jim's grammar correctly from `tcl-dialect` data: `$(expr)`, dict sugar, and literal backslash-newline in braces.
- `jim-own-surface.tclspec` gives every Jim-only command a release window, arity and hover.
- The analyser accepts `jimtcl` and raises no false "unknown command" on Jim code.

But the commands with scoping or lifetime effects (`local`, `upcall`, `ref` finalizers, statics, `alias`) carry no effect descriptors. So navigation sends a call to the wrong procedure under `local`, and the analyser reports false W210, W214 and W001 on correct Jim programs, one of them with a Rust `Debug` dump as its message.

The VM and the runtime each contain about 12,000–14,000 lines of Jim-specific code, mostly parallel copies of each other. Neither exposes a Jim mode from its command line, and neither implements references, `collect`, `defer`, `curry` or `stackdump` (*read*). So no engine can be compared with jimsh today.

## Measured

**jimsh against Tcl** (all fixtures; the VM and the runtime in 9.0 mode match tclsh 9.0 except where noted):

| Fixture | jimsh 0.84 | tclsh 9.0 / VM / runtime |
| --- | --- | --- |
| j01 `ref`/`getref`/`setref` | `1 hello world` | invalid command `ref` |
| j02 finalizer and `collect` | `finalized val`, `collected 1` | invalid command `ref` |
| j03 `local proc` | `local-helper`, then the command is gone | invalid command `local` |
| j04 `upcall` | `new+orig`, then `orig` | invalid command `local` |
| j05 `$($x*2)`, `"$(1+2)"` | `6`, `sum=3` | can't read `(3*2)` |
| j06 `$d(a)` on a dict scalar, `set d(c) 3` | `1`, `a 1 b 2 c 3` | variable isn't array |
| j07 `lambda` with statics | `8`, `2` | invalid command `lambda` |
| j08 `alias`, `curry` | `1:0`, `ABC` | invalid command `alias` |
| j09 namespaces | `::ns`, `5` | same |
| j10 `-errorinfo` and `stackdump` | keys `-code -errorcode -errorinfo -level`; `stackdump` works | adds `-errorline -errorstack`; no `stackdump` |
| j11 `exists` | `1:0:1:1:1` | invalid command `exists` |
| j12 `lmap` with `continue` | `1 4 9`, `1 3` | same |
| j13 `info statics`, `info alias`, `info references`, `info source` | supported | `proc` with statics is a usage error |
| j14 `catch -error`, `catch -break` | option-filtered catch | `-error` is run as a script. **The runtime's options dictionary orders its keys differently from tclsh and gives `-errorstack {INNER -error}` where tclsh and the VM give `{INNER {invokeStk1 -error}}`** |
| j15 backslash-newline in braces | kept (length 6) | folded (length 3) |
| j16 an array read as a dict | arrays are dicts | variable is array |
| j17 `defer`; j18 `loop`, `range` | run | invalid command |
| j19 procedure statics | `2` | usage error |
| j20 `010`, `7/2`, `-7/2`, `2**-1` | `10 8 3:-4 0` | same; **the runtime returns `0o10` for `catch {expr {0o10}} r`** (also `0x10`: j23) |
| j21 `global`, `upvar` | same as Tcl | same |
| j22 `string byterange`, `lassign`, `lreverse` | supported | no `byterange` |

**Analysis under `jimtcl`** (programs that run cleanly in jimsh, so every diagnostic is false):

| Fixture | Report |
| --- | --- |
| j19 `proc counter {} {{count 0}} {incr count}` | **W210** whose message is `Variable 'Callable { allocation: CallableStorageIdentity { site: CommandAllocationSite { …` |
| j22 `string byterange` | **W001 unknown subcommand** (`jim.tclspec` inherits Tcl's `string` roster) |
| j02 finalizer `proc fin {r v}` | **W214** `r` unused (a fixed-signature callback) |
| j11 `exists x` | W211 `x` set but never used (`exists` observes `x`) |
| j01, j03–j08, j17, j18 | none: the Jim-only commands resolve, and `$(…)`/`$d(k)` raise nothing |

**Navigation under `jimtcl`:**

| Site | Answer | Right answer |
| --- | --- | --- |
| `[helper]` after `local proc helper` (j03) | no hover, no definition | the `local proc` on line 3 |
| `[greet]` inside `wrap` after `local proc greet` (j04) | definition = the global `proc greet` (line 2) | the local `proc greet` (line 4) |
| `upcall greet` (j04) | nothing | the global `proc greet` |
| `[e v]` after `alias e info exists` (j08) | nothing | the alias, and through it `info exists` |
| `ref`, `getref`, `lambda`, `exists`, `defer`, `stackdump` | hover from `jim-own-surface.tclspec` | correct |
| `$d(a)` dict sugar (j06) | variable `d`, definitions on lines 2 and 4 | correct |

Under `tcl9.0` the same Jim code gets W002/W123 on every Jim command. That is correct, but `$d(a)` on a scalar (j06), `dict get` on an array (j16) and the brace backslash-newline difference (j15) get nothing, though each changes the result.

## Census

- **Dialect and grammar** (`tcl-dialect`). `Family::Jim` (`model/family.rs:143`) and the `jim`/`jimsh`/`jimtcl` environment (`model/environment.rs:1277`). `jim` is deliberately not a catalogue profile (`grammar.rs:1369`). The grammar data: expression sugar (`grammar.rs:1060`, `has_expr_sugar` :1085) and literal brace backslash-newline (`grammar.rs:983`, asserted for Jim at :1379).
- **Lexer.** The `ExprSugar` token (`tcl-lexer/src/tokens.rs:77`), `lexer/substitutions.rs:61`, `lexer/jim_rosters.rs`, dict-sugar expansion in `executable_parts.rs:432`. One owner, correct in the fixtures.
- **Registry: two declaration routes for Jim commands.**
  - The SpecTcl pack `rust/tcl-spectcl/core-surfaces/jim-own-surface.tclspec` (977 lines; `proc` :221, `alias` :272, `local` :287, `upcall` :298, `curry` :309, `lambda` :321, `defer` :354, `ref`…`finalize` :393–:448, `stackdump` :488, `exists` :511), with `jim.tclspec` for what Jim inherits.
  - Rust specs with `Family::Jim` surfaces in 18 files, including `tcl-registry/src/commands/tcl/alias_.rs:25` and `exists_.rs`.
  - So `alias` and `exists` are declared in both routes (*read*; which wins is not measured).
  - `local`, `upcall`, `ref`, `finalize`, `defer` and `curry` have arity and hover only. They have no `command_table_effect`, no scope-lifetime effect and no callback role. `proc` declares a `StaticVarList` role (:240), but the statics are not seen as variable definitions (j19).
- **Native Jim policy shared by the engines.** `tcl-syntax`: `native_jim_lookup.rs` (488 lines), `native_jim_local.rs` (126), `native_jim_index.rs` (151), `native_jim_substitution.rs` (220), `native_jim_switch.rs` (65), `jim_script_layout.rs`, `jim_script_objects.rs`. `tcl-registry`: `native_jim_lookup.rs` (571), `native_jim_enum.rs` (223), `native_jim_local.rs` (57), `native_jim_switch.rs` (29). `tcl-cmd-core/src/native_jim_switch.rs`.
- **Engines.**
  - The VM has 26 Jim-named source files (about 11,900 lines) and the runtime 40 (about 13,700). The same module names appear in both: `jim_local`, `native_jim_lookup`, `native_jim_import`, `native_jim_index`, `native_jim_script`, `native_jim_subcommands`, `jim_teardown`, `jim_alias_publication`, `native_jim_links`, `native_jim_increment`. That is the engine duplication of fault 10.
  - A Jim interpreter is built only in tests: `Interp::with_native_core(…, profile_for_dialect("jimtcl"), …)` (`runtime/rust/src/environment.rs:62`, used in `cmd_info/native_jim/variable_inventory_tests.rs:73`) and `tcl-vm/src/native_fixture.rs:28`. `run_script` rejects `--tcl-version jim` (`runtime/rust/examples/run_script.rs:159`), and `tclvm --tcl-version` lists only 8.4–9.1.
  - No production source in either engine names `ref`, `getref`, `setref`, `collect`, `finalize`, `curry`, `stackdump` or `defer` (string-literal search: zero files). `local` and `upcall` exist in both.
- **Tools.** The MCP dialect enum has no Jim entry (tool schema). The server accepts `jimtcl` through `workspace/didChangeConfiguration` (`tcl-lsp-server/src/configuration/requests.rs:44`).

## Design

Jim is a family with its own release axis (0.76–0.84). It should fit the target design as data plus effects, not as special cases:

- **Effects of Jim commands** in the registry's effect vocabulary (decision 6; fault 9):
  - `local cmd …` runs `cmd` and makes the command it defines live until the caller's frame ends: a command-table effect with frame-scoped lifetime.
  - `upcall name` selects the incarnation a `local` hid. This is `CommandRef` with incarnations (decision 5), with lookup returning the previous incarnation.
  - `alias`/`curry` create a command whose target is a command prefix, as `interp alias` does.
  - `ref … finalizer` and `finalize` take a callback of fixed signature `{ref value}`. That is a callback role, as for traces.
  - `proc` statics define variables with persistent per-procedure storage: entry facts for the body unit.
  - `exists` observes a variable or command without reading it.
  - `defer` registers a deferred body run at frame exit, so it needs a deferred point (decision 3).
- **The per-point name world** then answers `local`/`upcall` correctly. The call inside `wrap` sees the local incarnation, and after `wrap` returns the name world reverts. Navigation, W123 and rename read that, so the j04 answer follows without a Jim-specific branch.
- **Registry declarations: one route.** Jim commands are declared once, in the SpecTcl packs, compiled into the registry (L2). The Rust `Family::Jim` specs that duplicate a pack entry (`alias`, `exists`) are removed or reduced to native hooks referenced from the pack. `jim.tclspec`'s inherited `string` gains Jim's own subcommands (`byterange`, `bytelength`).
- **Engines (decided 23 November 2026: analysis only; plan item 196).** Jim execution is a non-goal: the test-only engine Jim modes are deleted and references and GC are not implemented. The original options were: a Jim mode exists inside both engines but is reachable only from tests. Either expose it (`--dialect jim` on `tclvm` and `run_script`), so the conformance matrix can run every Jim fixture against jimsh, or state that Jim execution is out of scope and delete the duplicated engine code that serves only native-fixture tests. Whichever is chosen, the copied modules move behind the engine traits (fault 10) with one implementation over `tcl-syntax`'s native Jim policy. References and GC are the large missing piece: one value-kernel feature (a reference table with tags, finalizers and a collector), not something each engine writes.
- **Tools.** The MCP dialect enum should list `jimtcl`, since the server already accepts it.

## Plan

- **Fix the false diagnostics under `jimtcl`.** Model procedure statics as body-unit entry definitions, so j19 has no W210, and make W210's message print the variable name, not a `Debug` value (`tcl-compiler` read-before-set). Add Jim's `string byterange` (and any other Jim-only subcommands) to the inherited roster in `jim.tclspec` (j22). Give `ref`'s and `finalize`'s finalizer argument a fixed-signature callback role, so W214 does not fire (j02). Declare `exists` as an observing reference (j11). Done when the four fixtures are clean under `jimtcl` and still flagged under `tcl9.0`.
- **Effect descriptors for `local`, `upcall`, `alias`, `curry`, `defer`.** Add them to `jim-own-surface.tclspec` with the registry's effect vocabulary (frame-scoped command lifetime, previous-incarnation lookup, command-prefix alias, deferred body at frame exit), and consume them in command binding. Done when definition on `[greet]` in j04's `wrap` goes to line 4, `upcall greet` goes to line 2, `[helper]` in j03 and `[e v]` in j08 resolve, and none of them raise W123.
- **One declaration route for Jim commands.** Remove or reduce the Rust `Family::Jim` duplicates (`tcl-registry/src/commands/tcl/alias_.rs`, `exists_.rs` and any other that a pack entry also declares) to native hooks referenced from the pack. Add a registry gate that fails when a command name has two declarations for the same family and release window.
- **Decide on Jim execution.** Either add `--dialect jim` to `tclvm` and `run_script`, wired to the existing `profile_for_dialect("jimtcl")` construction, and run `oo-dispatch-probe/jim/` against jimsh; or record Jim execution as a non-goal and remove engine code reachable only from native-fixture tests. Owners: `tcl-vm`, `runtime/rust`. Done when the decision is in the [plan](plan.md) and the matrix has a Jim engine column or an explicit "not run".
- **Share the engine Jim modules.** Move the modules duplicated between `tcl-vm/src` and `runtime/rust/src` (`jim_local`, `native_jim_lookup`, `native_jim_import`, `native_jim_index`, `native_jim_script`, `native_jim_subcommands`, `jim_teardown`, `jim_alias_publication`) behind `tcl-runtime-api` traits with one implementation, as fault 10 proposes for frames and namespaces. Done when each name exists once.
- **References and GC as one kernel feature** (only if Jim execution is kept). Implement `ref`/`getref`/`setref`/`collect`/`finalize` once in the value kernel (`tcl-cmd-core` or the kernel of item 81) with Jim's tag format and finalizer order, and conformance rows from j01 and j02.
- **Fix the runtime's measured differences** seen on these fixtures: the literal spelling returned by `catch {expr {0o10}}` (j20, j23) and the return-options key order and `-errorstack` content (j14), both against tclsh and the VM.
- **Tool parity.** Add `jimtcl` to the MCP dialect enum (`tcl-mcp` tool schema) so the MCP tools can be asked what the server already answers.
- **Conformance rows.** Move `oo-dispatch-probe/jim/` into the conformance matrix with columns jimsh 0.84 (and the 0.76–0.83 builds the pack's windows cite), tclsh, the analyser codes under `jimtcl` and `tcl9.0`, and the navigation answers in `navigation.out`.

## Status on the current source

Not re-run on the current source: there is no `jim` analysis profile reachable from `DialectProfile::find`; the page used the release server's `jimtcl` dialect. Details: [audit summary](tcl-areas-audit.md) and [`tcl-areas-head/`](handover/oo-dispatch-probe/tcl-areas-head/). Plan items 153–199.
