# Errors, completion codes and return options

How `catch`, `try … on … trap … finally`, `return -code/-level/-options`, `error`, `throw`, custom integer codes, `break`/`continue` leaving procedures and scripts, `errorInfo`, `errorCode`, `-errorline` and `-errorstack` behave in the two engines, and how analysis (CFG reachability, W210, W220, W242, W302, the optimiser's O107/O103) models them. Method: 32 fixtures in [`oo-dispatch-probe/errors-returns/`](handover/oo-dispatch-probe/errors-returns/), each printing deterministic output, run under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, Jim, the VM (`tclvm --tcl-version 8.6|9.0`) and the runtime (`run_script --quiet --tcl-version 8.6|9.0`) on `afe2f2d8` (`engines.tsv`); then the analyser and optimiser through the MCP server (`analyser-mcp.out`), and a read-only census of `rust/` and `runtime/rust`.

**Verdict:** the completion-code core is **right in both engines** — every catch, try, trap, finally, custom-code, `return -level`, `-options` rethrow, loop-propagation and `errorInfo`/`-errorstack`/`-errorline` fixture matches tclsh on 8.6 and 9.0 — and it is the best-shared area audited so far: `tcl-cmd-core::return_options` and `native_return_merge` own option parsing and merging, and the registry owns the `try` clause grammar. The **wrong answers are all in `errorCode`**: neither engine sets `POSIX <errno> <msg>` for filesystem failures, so `trap {POSIX ENOENT}` handlers are skipped and the error escapes; and the runtime reports a missing variable as `TCL READ VARNAME` (or `NONE` for `$x`) where tclsh says `TCL LOOKUP VARNAME x`. On the analysis side the cached MCP build gives false W210 for a `for` loop directly inside `catch`, false W220/W210 around `finally` and `return -level 0`, deletes `finally` bodies (O107) and misses every "break outside a loop" error; HEAD's source shows fixes for the `finally` and `return -level 0` cases that this audit could not run. The sprawl is in the **completion-code word grammar (five readers), `trap` matching (two engine copies), five completion-code enums, and about 55 files that pattern-match `catch`/`try` statements** instead of reading one exit model.

## Measured

Execution (all fixtures; 8.4/8.5 rows fail where `try`, `throw`, `lmap`, `-errorstack` or two-variable `catch` do not exist yet, as tclsh does):

| Case | tclsh 8.6–9.1 | VM | runtime |
| --- | --- | --- | --- |
| catch codes 0–4, custom 5/99/-3, `return -code` names (`e01`, `e11`) | as Tcl | same | same |
| options dict keys per code, `-level 2`, extra `-foo` keys (`e02`, `e21`) | as Tcl | same | same |
| `return -code break/continue` from a procedure ends the caller's loop (`e03`); bare `break` leaving a procedure is `invoked "break" outside of a loop` (`e04`) | as Tcl | same | same |
| `try` handler order, `trap` prefix match, `on 5`, finally ordering, `-during` (`e05`, `e06`, `e18`) | as Tcl | same | same |
| `throw`, `error msg info code`, `errorInfo` growth through `eval`/`uplevel`/procs, `-errorstack`, `-errorline` (`e07`–`e10`, `e16`) | as Tcl | same | same |
| `return -options $o` rethrow, `-level 0`, `-level 2` and `-level 2 -code break` (`e13`, `e28`, `e30`) | as Tcl | same | same |
| break/continue through `eval`, `uplevel 1` from a proc, `catch`, `switch`, `uplevel #0` (`e12`, `e17`) | as Tcl | same | same |
| **filesystem errorCode** (`e19`, `e32`): `open`, `cd`, `file stat` on a missing path | `POSIX ENOENT {no such file or directory}`; `trap {POSIX ENOENT}` runs | `NONE`; trap skipped, error escapes | same as VM |
| **missing-variable errorCode** (`e08`, `e31`), top level | `TCL LOOKUP VARNAME x` | same as tclsh | `TCL READ VARNAME`; `$x` gives `NONE` |
| uncaught error/`break`/custom code in file mode (`e22`–`e24`) | traceback, exit 1 | same | `error: msg`, no traceback (the example runner, not an engine fault) |

Inside a procedure tclsh itself reports `TCL READ VARNAME` (compiled read); the runtime is wrong only for uncompiled reads. Jim differs throughout (`try` traps not supported, no `-errorline`/`-errorstack`, `error` takes no code), as expected.

Analysis (MCP server binary of 19 Nov; any diagnostic on a fixture that runs cleanly is false):

| Case | Report | HEAD status |
| --- | --- | --- |
| `catch {for {set i 0} {$i<3} {incr i} {…}}` as a statement (`e25`) | **W210 `i` read before set**, spanning the whole `catch` | no test found; `catch` now lowers to real blocks with an exception edge per statement (`cfg_builder/cfg_lower/catch.rs:41`), the likely source (*inference*) |
| same inside `[catch …]` in an `if`, or `foreach`/`while` in `catch` | none | — |
| `for … {break} …` step (`e17`, `e26`) | W242 "termination cannot be proven" (the step is `break`) | not checked |
| `try {set a 1; return 1} finally {puts $a}` (`e29`) | **W220 assignment never read** | probably fixed with the `finally` edges below (*inference*) |
| `return -level 0 x; set y 1; return $y` (`e30`) | **W210 `y`** | fixed: `lowering/hooks/control.rs:118` lowers `-level 0` ok/error/break/continue as an ordinary command |
| `array get missing` (`e31`) | W210 (the command reads nothing that must exist) | not checked |
| bare `break` leaving a proc (`e04`), `uplevel 1 {break}` (`e12`), top-level `break` (`e22`) | nothing (always an error in Tcl) | no diagnostic exists |
| code after `return`, `error`, `throw`, `break` (`e14`) | nothing in diagnostics; the optimiser finds it (O107) | there is no unreachable-code diagnostic, only O107 and I230/I231 |
| 8.4 `catch {…} r o` | nothing (`wrong # args` in tclsh 8.4) | registry arity is not per release |
| 8.5 `try`, `throw`, `lmap` | W002 **and** W123 (8.4 gives W002 only) | — |
| W210 after `catch` bodies and result/options variables (backlog) | **not reproduced** (`e15`, `e02`) | — |

Optimiser (MCP `optimize`, profiles full and aggressive, 8.6 and 9.0; readability, the LSP default, does nothing):

| Rewrite | Effect | HEAD status |
| --- | --- | --- |
| O107 empties `finally {puts fin}` after `try {return 1}`; O103 then folds `[t]` to `1` (`e27`) | prints `1` instead of `fin 1` | fixed in source as #2142 (`cfg_lower/try_construct.rs:401-420`); this binary still deletes #2142's own example |
| O107 deletes `puts reached-v` after `return -level 0 lvl0` (`e28`) | prints `lvl0` instead of `reached-v` | fixed (`control.rs:118`) |
| O107 after `return`, `error`, `throw`, `return -code error`, `continue`; procedures returning `-code break/continue` | correct; not inlined | — |

## Census

**Engines — shared and right.** `tcl-cmd-core/src/return_options.rs` (811 lines: `parse_completion_code` :203, option pairs, `-level`, `-options`) and `native_return_merge.rs` (226) are the owners; the VM (`tcl-vm/src/return_options.rs` 743, `native_return_merge.rs` 666) and the runtime (`runtime/rust/src/return_options.rs` 451, `native_return_merge.rs` 385) implement only object adapters, except that `known_word`/`manufacture` are copied in both (VM :98–155, runtime :109–170). The `try` clause grammar and its compilation are registry data (`tcl-registry/src/native_try.rs`, `native_try_compilation.rs`, `clause_grammar.rs`); `catch` variable effects are `tcl-registry/src/catch_invocation.rs` (725). Error-stack types are shared in `tcl-runtime-api/src/error_stack.rs`, but 34 VM and 37 runtime files touch `errorstack`.

**Engines — copied.**

- `trap` errorcode prefix matching: VM `cmd_try.rs:164` and runtime `cmd_error.rs:790`, not in `tcl-cmd-core`.
- `try`: three interpreters of the same clauses — VM `cmd_try.rs` (1488), runtime `cmd_error.rs` (1756, interpreted path) and `interp/native_body_artifact/native_try.rs` (904), plus Jim's `jim_try.rs` (399).
- Completion-code words: `tcl-cmd-core` `parse_completion_code` (engines' `return`), runtime `code_word_to_int` with its own `bad completion code` text (`cmd_error.rs:777`, :1150), registry `completion_code_selector` (`completion.rs:54`) and `resolve_completion_code_selector` (:128) in the same file, and `tcl-diagram/src/data.rs:633`. **Five readers.**
- POSIX errorCode: `tcl-cmd-core/src/encoding/original_names.rs:177` maps `HostError` to `POSIX ENOENT …`, but the channel and filesystem commands of both engines (`tcl-vm/src/cmd_chan.rs:197` `io_reason`, `runtime/rust/src/cmd_chan.rs:408`) build only the message. The owner exists; two consumers ignore it.
- Completion-code types: `tcl_core_types::Code` (lib.rs:89, also the registry's `CompletionCode`), the runtime's own `interp::Code` (interp.rs:213, a copy), `tcl_engine_api::CompletionCode` (lib.rs:555, no `Error`).

**Analysis.** Exits are modelled by:

1. `cfg_builder/flow_facts.rs:47` `Completion` — three states (`Normal`, `LoopJump`, `ProcExit`), folding `return`, `error`, `throw`, `exit` and `tailcall` together, with no custom codes; used by four CFG files.
2. `tcl-compiler/src/completion.rs:45` `CompletionCodeLattice` — exact code sets, used only by `semantic_analysis.rs`.
3. The registry's value-transfer completions (`CompletionOutcome` in 29 files, `ExitRule`, `CompletionProtocol`) and `InvocationCompletion` (90 files).
4. `executable_ir`'s `CompletionPayload` and exception regions; `value_transfer.rs:147` `PathCompletion`; interprocedural `return_summaries.rs`.
5. The CFG lowering of `catch` (`cfg_lower/catch.rs`, 245) and `try` (`try_construct.rs`, 1142, most of it repairs for `finally` reachability found one review at a time).

About **55 production files** match `Statement::Catch`/`Statement::Try` directly (42 in `tcl-compiler`: SSA defs and uses, escape analysis, GVN rebase, code sinking, six optimiser passes, upvar and global-write summaries, interprocedural body scans, iRules flow; 10 in `tcl-codegen` inlining; diagram, explorer and `f5-xc`), each deciding for itself what a trapped body does. Spelling checks remain: `inline_uplevel.rs:398` and `shimmer/thunking.rs:133` find `break` by name, `cfg_builder/execution_regions.rs:1078` finds `return` by its first word, and the refactors in `tcl-lsp-core/src/refactor/{switch_to_dict,datagroup}.rs` and `tcl-mcp/src/datagroup.rs:264` match a two-word `return`. The registry's `break` documentation (`commands/tcl/break_.rs:57`) says the procedure's top level handles a `break`, which tclsh contradicts (`e04`).

**Owners and disagreements.** The engines agree with each other and with tclsh except `errorCode`. Analysis has two exit vocabularies that disagree by construction: `flow_facts::Completion` cannot say "custom code" or "error, but a `finally` still runs", which is why `try_construct.rs` grows special cases, while `CompletionCodeLattice` can but is almost unused. Release gating disagrees between 8.4 (W002) and 8.5 (W002 plus W123), and the arity of `catch` is not per release.

## Design

- **Registry effects own completion.** Each command's descriptor states its possible completions as a set over `tcl_core_types::Code` with payload obligations — `error`/`throw` complete with `Error` and an errorCode shape; `return` decodes its options (`ReturnDecoding`, already registry-owned); `break`/`continue` complete with their codes; `catch`, `try`, loops, `switch`, `if`, `eval`, `uplevel` and procedure calls declare which codes they *intercept* and which they *convert* (a procedure turns `Break`/`Continue` into an `Error` with the "outside of a loop" message, and `Return` at level 1 into its result). This is the existing transitions vocabulary (decision 6 of [design integration](design-integration.md)), extended, not a parallel one.
- **One exit model in analysis.** The CFG reads those descriptors and carries `CompletionCodeLattice` per edge, replacing `flow_facts::Completion`. `finally` is then not special: it is a region every completion passes through, and O107, W210 and W220 query the same edges. "Break outside a loop", "custom code reaching a procedure boundary" and "unreachable after an unconditional exit" become diagnostics computed from that model, not handler cases. Consumers that match `Statement::Catch`/`Try` today ask the annotated command (decision 4) for its intercepted codes and body regions instead.
- **Frames and receipts.** `uplevel`/`eval` bodies take the caller's loop context only when the level is proven (`e12`: `uplevel 1 {break}` from a procedure is an error because the procedure boundary converts it). Answers about reachability carry a receipt; an unproven callee leaves the region possibly reached (decision 7), never unreachable.
- **Engines.** `trap` matching, the completion-code word reader and the POSIX errorCode builder belong in `tcl-cmd-core` behind the engine traits; the runtime's `interp::Code` becomes `tcl_core_types::Code`. Missing-variable errorCode comes from one constructor per form: `TCL LOOKUP VARNAME name` for `TclLookupVar` failures, and `TCL READ VARNAME` for a compiled read, which `CmdError::variable_read_missing` (`tcl-cmd-core/src/error.rs:454`, no production callers) already spells.
- **Conformance vectors.** The fixtures become rows of the conformance matrix (fault 5 of the [cross-cutting review](cross-cutting-review.md)): engines against every tclsh, and the analyser's W210/W220/W242/O107 columns, with "optimise then execute" as a check that compares output.

## Plan

- **Fix POSIX errorCode in both engines:** route `open`, `cd`, `file` subcommands, `glob`, `exec` and channel failures through the `HostError` → `POSIX errno msg` mapping in `tcl-cmd-core/src/encoding/original_names.rs:177` (moved to a channel-neutral helper); done when `e19` and `e32` match tclsh 8.6/9.0 in the VM and the runtime, and a vector row covers `trap {POSIX ENOENT}`.
- **Fix the runtime's missing-variable errorCode:** uncompiled reads and `$x` substitution report `TCL LOOKUP VARNAME name`, as `TclLookupVar` does (the exact runtime path was not located; the shared `TCL READ VARNAME` builders in `tcl-runtime-api/src/array_access.rs:173` and `:193` are the likely source, *inference*), and `$x` sets an errorCode at all; done when `e08` and `e31` match tclsh on 8.6 and 9.0.
- **Move `trap` matching and completion-code words into `tcl-cmd-core`:** one prefix matcher replaces VM `cmd_try.rs:164` and runtime `cmd_error.rs:790`; `parse_completion_code` serves runtime `code_word_to_int` (`cmd_error.rs:777`), the registry selectors (`completion.rs:54`, :128) and `tcl-diagram/src/data.rs:633`; delete the runtime's `bad_completion_code`. Done when only one function spells `"continue" => 4`.
- **One completion-code type:** replace the runtime's `interp::Code` with `tcl_core_types::Code`; document `tcl_engine_api::CompletionCode` as the host projection. Done when the runtime has no own enum.
- **Re-run this audit's analyser rows at HEAD:** add `e25`–`e30` to the analyser's conformance tests (W210, W220, W242, O107 with execute-before-and-after), so the cached-build findings are either confirmed fixed or fixed; owner: `tcl-compiler/src/cfg_builder`. Done when each row is a test that runs the fixture and checks the diagnostics against tclsh output.
- **Fix the `catch`-wrapped `for` false W210:** find why `cfg_lower/catch.rs` splits a `for` statement so its test reads `i` before the init block's definition; done when `e25` gives no W210 and a test pins it.
- **Replace `flow_facts::Completion` with registry-declared completions and `CompletionCodeLattice`:** add intercept/convert sets to the effect descriptors of `catch`, `try`, loops, `switch`, `eval`, `uplevel` and procedure calls; make `try_construct.rs`'s `finally` edges a consequence of the model. Done when `try_construct.rs` loses its special cases and the #2142 family of fixtures still passes.
- **Add the missing diagnostics from that model:** break/continue reaching a procedure or script boundary, custom code reaching the top level, and unreachable commands (as a hint with `Unnecessary` tag, sharing O107's facts). Done when `e04`, `e12`, `e14` and `e22` produce them and nothing in the other fixtures does.
- **Per-release `catch` arity and one W002/W123 policy:** registry `catch` takes one variable on 8.4; a command disabled in the profile reports W002 only on every release. Done when 8.4 and 8.5 rows of `analyser-mcp.out` agree.
- **Retire statement pattern-matching:** move the ~55 `Statement::Catch`/`Try` matchers and the spelling checks (`inline_uplevel.rs:398`, `shimmer/thunking.rs:133`, `execution_regions.rs:1078`, refactor `return` matchers) onto the annotated command's intercepted codes; track the count in the walker manifest with an only-shrinks ratchet.
- **Correct the `break` documentation** in `tcl-registry/src/commands/tcl/break_.rs:57` (a procedure's top level converts `break` into an error).
- **Engines' copied factories:** move `known_word`/`manufacture` from both `native_return_merge.rs` files into `tcl-cmd-core::native_return_merge`.
