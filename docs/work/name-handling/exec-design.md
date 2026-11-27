# `exec`, command pipelines and process words

How `exec`, `open "|…"`/`open |[list …]`, `auto_execok` and `chan pipe` are executed by the two engines and described to analysis (registry, W103, taint). Ten fixtures in [`oo-dispatch-probe/exec-pipes/`](handover/oo-dispatch-probe/exec-pipes/) cover the argument grammar (`|`, `|&`, `<`, `<@`, `<<`, `>`, `>>`, `2>`, `>&`, `2>@1`, `>@`, `2>@`, `&`, `-ignorestderr`, `-keepnewline`, `--`), exit status and `errorCode`, background pipelines, pipe channels, quoting and redirection injection. Only `echo`, `cat`, `true` and `false` are run. Each fixture ran under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, Jim, the VM (`tclvm --tcl-version 8.6|9.0`) and the Rust runtime (`run_script --quiet --tcl-version 8.6|9.0`) on `afe2f2d8`; results are in `engines.tsv`. The analyser was asked through the MCP tools (a cached build that may lag `afe2f2d8`); its answers are in `analyser-mcp.out`.

**Verdict:** neither engine implements `exec`. The VM runs the first word as a program and passes every other word, including `|`, `<`, `>`, `2>@1`, `&` and its own switches, to it as arguments, ignores the exit status and standard error, and strips every trailing newline; it rejects `open "|…"` and has no `chan`, `pid`, `fileevent` or `fcopy` command. The runtime refuses `exec`, `chan pipe` and `auto_execok` and treats `open "|cmd"` as a file name. On the analysis side the pipeline grammar exists only as prose in the `exec` hover: nothing reads redirection words, no grammar error is reported, no taint code fires for `exec` in plain Tcl, and the substituted-redirection injection hazard that tclsh exhibits is not modelled. W103 handles `open` well but ranks the safe `|[list …]` form above the literal form.

## Measured

C Tcl agrees across 8.4–9.1 on every grammar row (differences: `-ignorestderr` and the three-argument `catch` need 8.5; `chan pipe` and `chan close … write` need 8.6; `-encoding` and the "bad option" wording are 9.0). `2>@1` works on 8.4.20 as well, although the registry hover says 8.5+ (`rust/tcl-registry/src/commands/tcl/exec_.rs:126`).

| Fixture | tclsh 8.6/9.0/9.1 | VM 8.6 and 9.0 | Runtime |
| --- | --- | --- | --- |
| `e01` `exec -keepnewline echo ab`; `exec false` | length 3; error, `CHILDSTATUS pid 1` | tries to run a program called `-keepnewline` ("no such file or directory") | `"exec" is not supported under the WASM runtime` |
| `e02` `exec echo a b c \| cat \| cat` | `a b c` | `a b c \| cat \| cat` (the bars are arguments) | refused |
| `e02` `exec echo a \|` | `illegal use of \| or \|& in command` | `a \|`, no error | refused |
| `e03` `exec cat << {from value}` | `from value` | empty | refused |
| `e04` `exec echo one > out.txt` then `cat out.txt` | `one`, `two` after `>>` | no file; `cat` of a missing file returns empty, no error | refused |
| `e04`/`e05` stderr, `2>@1`, `-ignorestderr` | error / folded / exit status still fails | never an error; `$::errorCode` unset ("can't read") | `errorCode` `NONE` |
| `e06` `exec echo bg > bg.txt &` | one integer pid | a four-word string `bg > bg.txt &` | refused |
| `e07` `open "\|echo piped" r`, `open \|[list cat] r+` | works; `close` raises `CHILDSTATUS` | `command pipelines are not supported` | `couldn't open "\|echo piped": no such file or directory` |
| `e08` `set user {<user.txt}; exec echo $user` | empty line: the substituted word became a redirection | `<user.txt` | refused |
| `e08` `set user {>made.txt}; exec echo harmless $user` | creates `made.txt` | prints the word | refused |
| `e08` `exec -- echo $user` | `--` does not protect later words (empty line) | runs a program called `--` | refused |
| `e09` `auto_execok echo` | path | path (each unknown command costs ~10 s in the debug VM) | `invalid command name "auto_execok"` |
| `e10` `chan pipe` with `>@`/`<@` | works (8.6+) | `invalid command name "chan"` | `chan pipe is not supported under the WASM runtime` |

`e03`'s `exec cat < in.txt` prints the right text under the VM only by accident: `cat` is given `<` and `in.txt` as files and the error for `<` is discarded. The runtime's refusals say "WASM runtime" although `run_script` is a native binary (`runtime/rust/src/cmd_misc.rs:44-56`). Jim rejects `-keepnewline`, `-ignorestderr` and `--` as program names and has no `auto_execok` or `chan`; otherwise it agrees with tclsh on the grammar rows.

**Analysis** (the programs are correct; `analyser-mcp.out`):

| Case | Answer |
| --- | --- |
| `open "\|echo piped" r` | W103 hint (right) |
| `open \|[list cat] r+` (the recommended form) | W103 **warning**, "containing substitution risks injection" — stronger than the literal form |
| `open $mp/hello.txt r` with `mp` a `//zipfs:/` path | W103 warning (heuristic; the value cannot start with `\|`) |
| `exec echo a \|`, `exec \| echo a`, `exec -bogus …`, `exec echo a >` (outside `catch`) | nothing |
| `exec -encoding` under 8.6, `exec -ignorestderr` under 8.4, bare `exec` | W004, W004, E002 (right) |
| `set user [gets stdin]; exec sh -c $user` (also `exec cat $user`, `socket $user 80`) | no T100 or T104 from `analyze`, `review` or `validate`; `exec` carries `TAINT_SINK` |
| `exec echo $user` where `user` may start with `<`, `>`, `\|`, `2>` or `&` | nothing |
| optimiser across `exec` | no rewrites (safe) |

## Census

Production code that implements, models or special-cases these commands:

- **Engines.**
  - `tcl_cmd_core::platform::exec` (`rust/tcl-cmd-core/src/platform.rs:86-121`): switches come from the registry spec (`resolve_exec_switch`); the remaining words are passed through as one argv; the exit and stderr raise as `TclCleanupChildren` does. It is the VM's whole `exec` (`rust/tcl-vm/src/cmd_file.rs:42`, `:148`).
  - `tcl_platform::Process::run` (`rust/tcl-platform/src/lib.rs:455`) and `NativeProcess` (`rust/tcl-host-native/src/lib.rs:653`): run one argv to completion. No pipes, redirections, background processes or pid list exist at the host layer.
  - VM `open`: `name.starts_with('|')` → "command pipelines are not supported" (`rust/tcl-vm/src/cmd_chan.rs:168`). No `chan`, `pid`, `fileevent`, `fcopy`, `load`, `socket` commands are registered (measured with `info commands`).
  - Runtime: `exec`, `socket`, `load`, `fileevent`, `fcopy` registered as unsupported (`runtime/rust/src/cmd_misc.rs:47-56`); `pid $chan` returns `{}` (`:64`); `chan pipe` unsupported (`runtime/rust/src/cmd_chan.rs:247`); `open "|…"` falls through to the file path. `auto_execok` is missing because the runner loads no `auto.tcl`.
- **Registry.**
  - `exec` spec (`rust/tcl-registry/src/commands/tcl/exec_.rs`): options (`-encoding` 9.0+, `-ignorestderr` 8.5+, `-keepnewline`, `--`), `TAINT_SINK | TAINT_SOURCE | UNSAFE | SAFE_INTERP_HIDDEN`, a `Process` side effect, the `SHELL_ATOM` safe colour (`:133`), and the whole word grammar as hover prose (`:126`). There is no structured description of redirection or pipeline words.
  - `open` (`rust/tcl-registry/src/commands/tcl/open_.rs:139`): the literal `|` test that skips file-open transitions.
  - `auto_execok` (`auto_execok.rs`), `chan pipe` (`chan_.rs:662`), `pid`, `irules_policy.rs:98, 349, 385` (the iRules ban), `taint.rs:1426`.
- **Analysis.**
  - W103: `RegistrySourceDiagnosticKind::ChannelPath` and `pipeline_prefix` (`rust/tcl-compiler/src/analyser/diagnostics/security/source_paths.rs:80-125`): literal `|` → hint, `|` followed by any substitution → warning, other substitution → warning.
  - Taint sinks are registry-driven (`rust/tcl-compiler/src/taint.rs:85-100`, `taint/sink_warnings/classification.rs`), but none fired in the measured plain-Tcl cases.
  - W304 (`--` terminator) is generic option handling.
- **Tools.** `tcl-spec-hooks` keeps its own deny tables naming `exec` (`rust/tcl-spec-hooks/src/pack_eval.rs:122`, `sandbox.rs`).

Duplicates of one rule: the "this word starts a pipeline" test is written three times (`open_.rs:139`, `source_paths.rs:104`/`pipeline_prefix`, `cmd_chan.rs:168`), each as a first-byte check. The exec word grammar has **no** implementation anywhere in the repository, so there is nothing to disagree with; the hover prose is the only description, and it is wrong about `2>@1`. The unsafe-command roster exists as the `SAFE_INTERP_HIDDEN` trait and again as hand-written tables in `tcl-spec-hooks` and `irules_policy.rs`.

## Design

- **The grammar is registry data with one reader.** `exec` and the pipeline form of `open` share one grammar (C's `TclCreatePipeline`): switches up to `--`, then words that are either arguments or one of about twenty redirection and pipe operators, some taking the next word and some glued to it (`<file`, `2>@1`), with `&` only as the last word. It belongs in `tcl-syntax` as a pure classifier over words (`PipelineWord::{Arg, Pipe, PipeStderr, Redirect { stream, target, append }, Background}`), described per release in the registry and checked against all five tclsh releases and Jim. The engines, the analyser and the hover all read it; the hover prose is generated from it.
- **Engines.** The host trait needs a pipeline operation, not a single argv: stages, redirections to files, channels or a literal, stderr policy, background launch with a pid list, and exit status. Both engines then implement `exec`, `open "|…"`, `pid`, `chan pipe` and the `CHILDSTATUS`/`CHILDKILLED`/`POSIX` error codes once, through `tcl-cmd-core`, instead of the current minimal passthrough. Until then `exec` should refuse anything the minimal form cannot honour (a redirection word, `&`, a switch, a non-zero exit) with a catchable error, not run a program named `-keepnewline`. The runtime's message should not say "WASM" in a native build.
- **Analysis.** Each `exec` invocation becomes an annotated command (design integration, decision 4) whose words carry their pipeline role, so W-codes can report grammar errors (dangling `|`, `&` not last, missing redirection target, unknown switch) as proofs, and the per-release switch and operator sets come from the registry. The injection model is a word property: a substituted word in argument position whose value is not proven to avoid the redirection prefixes is an injection site, independent of taint; with taint, `exec`'s sink applies per argument word and the `SHELL_ATOM` colour is the sanitiser. The `|[list …]` form of `open` is the sanitiser for the open case and should lower W103 to the hint, not raise it.
- **Answer and receipts.** A pipeline whose operator words are substituted is `Answer::Unknown` with the reason "operator position depends on a value"; consumers abstain rather than guess.
- **Conformance.** The ten fixtures become vector rows (word list → classified pipeline, and → output, status and `errorCode`) driven through tclsh 8.4–9.1, Jim, both engines, the classifier and the analyser.

## Plan

- **Stop the VM running the wrong program** (owner: `rust/tcl-cmd-core/src/platform.rs`): until pipelines exist, `exec` raises a catchable error for switches, operator words and `&`, honours `-keepnewline`/`--`, strips one newline, and raises `child process exited abnormally` with `CHILDSTATUS` on a non-zero exit and an error on unredirected stderr. Done when `e01`, `e05` and `e08` match tclsh or fail with that error under the VM.
- **One pipeline grammar** in `tcl-syntax` (new module beside `frame_level`), with per-release operator and switch tables in the registry `exec` spec, pinned by a vector file generated from tclsh 8.4–9.1 and Jim. Done when the `exec_.rs` hover's word description is generated from it (fixing the `2>@1` release claim) and `open_.rs:139`, `source_paths.rs` and `cmd_chan.rs:168` call it instead of a first-byte test.
- **Host pipelines** (owners: `tcl-platform`, `tcl-host-native`): a `Process` operation taking the classified pipeline, returning output, status, stderr and pids, with a background mode. Done when `e01`–`e08` and `e10` match tclsh under both engines.
- **`chan`, `pid`, `fileevent`, `fcopy` in the VM and `exec`, `chan pipe`, `auto_execok` in the runtime**, or an explicit non-goal recorded in the engine docs with a refusal message that names the engine, not "WASM". Done when `e07`, `e09` and `e10` pass or fail with the documented refusal.
- **Exec diagnostics from the grammar** (owner: `tcl-compiler` analyser diagnostics): new W-codes for a dangling or leading `|`, a redirection without a target, an unknown switch and `&` not last, all suppressed when operator words are substituted. Done when the analysis rows above report them outside `catch`.
- **Redirection injection** (owner: taint and security diagnostics): flag a substituted `exec` argument word that is not proven free of the operator prefixes, at hint level without taint and warning level when tainted; treat `{*}[list …]` of proven words and the `SHELL_ATOM` colour as safe. Done when `e08`'s three substitutions are reported and `exec echo [string map …]` with a proven value is not.
- **Taint coverage for plain Tcl:** find why no T100/T104/T105 is produced for `exec sh -c [gets stdin]`, `socket $user 80` and `interp eval $c $user` in the Tcl 9.0 dialect through `analyze`, `review` and `validate` (taint wiring or the MCP build), and add those three as MCP regression tests. Done when they report.
- **Re-rank W103** (`source_paths.rs`): the `|[list …]` form with literal list elements is a hint, the same as the literal string; a string `"|cmd $x"` stays a warning. Done when `e07` gives hints only.
- **Fold the unsafe-command rosters** in `tcl-spec-hooks/src/pack_eval.rs` and `irules_policy.rs` into registry traits, so `exec`'s classification has one owner.
- **Exec vectors in the conformance matrix:** the ten fixtures as rows for tclsh 8.4–9.1, Jim, both engines, the grammar classifier and the analyser.

## Status on the current source

Confirmed on the current source: no diagnostic for redirection injection (`e08`); the VM's `exec echo a | cat` prints `a | cat`. The VM's `exec` now resolves its switches from the registry spec per release, strips one newline, and raises `CHILDSTATUS pid status`, `CHILDKILLED pid SIGNAME msg`, or (without `-ignorestderr`) an error on stderr output, matching tclsh 8.4–9.1 (`rust/tcl-vm-cli/tests/exec_child_status.rs`); pipeline words and `&` are still passed through, and `-encoding` is not applied. Details: [audit summary](tcl-areas-audit.md) and [`tcl-areas-head/`](handover/oo-dispatch-probe/tcl-areas-head/). Plan items 153–199.
