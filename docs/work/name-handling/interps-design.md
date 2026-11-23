# Multiple interpreters

How child interpreters — `interp create` (and `-safe`), `interp eval`, aliases across interpreters, hidden commands, `share`/`transfer`, limits, `marktrusted`, `bgerror`, `cancel`, the `safe::` package and the child's own command (`$child eval …`) — are executed by the two engines and modelled by analysis, and in particular how a body that runs in another interpreter is treated: the parent's commands and variables are not visible there. Sixteen fixtures in [`oo-dispatch-probe/interps/`](handover/oo-dispatch-probe/interps/) were run under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, Jim, the VM (`tclvm --tcl-version 8.6|9.0`) and the Rust runtime (`run_script --quiet --tcl-version 8.6|9.0`) on `afe2f2d8` (`engines.tsv`), and through the analyser, go to definition, references, rename, hover and the optimiser over MCP, a cached build that may lag that commit (`analyser-mcp.out`).

**Verdict:** the VM is right on almost everything that matters (worlds, aliases both ways, hide/expose, limits, `safe::`), with three faults: `share`/`transfer` are silent no-ops, `interp cancel` aborts the caller, and wrong-argument messages through aliases use the 8.4 wording. The runtime cannot alias from a parent into a child, refuses `share`, `transfer` and `cancel` by stopping the whole script, does not enforce command limits, loses `errorCode` across `interp eval`, and has no `safe::` package. Analysis has a real model of child worlds — a synthetic `@interp@<path>` domain per child, safe-interpreter visibility, release gates — but it is a second model beside command binding's interpreter transitions, it reads `interp create` with a third parser, and the editor features do not respect the world boundary: rename edits words in the wrong interpreter and misses words in the right one, references cross from a procedure's locals into a child's globals, and `interp bgerror`'s handler is resolved in the parent.

## Measured

C Tcl agrees across 8.4–9.1 apart from release additions (`limit`, `bgerror`, `debug` 8.5; `children`, `cancel` 8.6) and the 8.5 change to name the alias in wrong-argument errors. Jim's `interp` is a different command (`interp` with no arguments returns a handle) and fails every fixture at `interp create`.

**Execution:**

| Fixture | tclsh | VM 8.6 and 9.0 | Runtime 8.6 and 9.0 |
| --- | --- | --- | --- |
| `i01`, `i11` world isolation (parent vars/procs invisible in the child, child's invisible in the parent, proc locals invisible to the body) | as described | same | same |
| `i02`, `i12` alias child→parent, parent→child, `{}`→`{}`, late binding, deletion | works | works | **parent→child refused**: `only single-interp aliases (empty interpreter paths) are supported` |
| `i15` wrong # args through an alias, 8.5+ | `should be "viaAlias"` (the alias name) | `should be "target"` (8.4 wording) | same as VM |
| `i03`, `i13` `$c eval`, `$c alias`, prefixes (`interp cr`, `$c ev`) | works | works | works |
| `i04` `interp create -safe` hidden set | includes `load`, `socket` | `load`, `socket` missing (the VM has neither command) | same as tclsh |
| `i05` hide/expose/invokehidden, `-global`, hiding in the parent | works | works | works |
| `i06` `interp share`, `interp transfer` | the channel is usable in the child | silent no-op returning `{}`, then `can not find channel named "file3"` | `run_script: execution refused: native interpreter worker native handler policy is not selected`, exit 2 |
| `i07` `interp limit $c commands -value 100` then a loop | `command count limit exceeded` | same | limit stored but not enforced: the loop never stops (timeout at 120 s) |
| `i10` `errorCode` from `interp eval`, `interp bgerror` | `MY CODE`, handler runs | same | `errorCode` `NONE`; handler runs |
| `i09` `safe::interpCreate`, `-accessPath` tokens, `source`/`exit` aliases | works without `package require` (auto-loaded) | works (26 s in the debug build) | `invalid command name "safe::interpCreate"` (no library) |
| `i14` `catch {interp cancel $c}` on an idle child | `0 {}` | the whole script stops, exit 1, no message | execution refused, exit 2 |
| `i16` `interp create -s b`, `interp create c -safe`, `-- -safe`, `-bogus` | safe, safe, unsafe, `bad option` | same | same |

**Analysis** (every fixture is a correct program, so each report is false unless marked):

| Case | Answer |
| --- | --- |
| formal of a proc defined inside `interp eval $c {…}` read in its body (`i05` `$a`, `i10` `$msg`) | W210 read before set |
| `interp create kid; kid eval {…}` (`i03`) | W123 "Unknown command 'kid'; did you mean 'pid'" |
| `safe::interpCreate` without `package require safe` (`i09`) | W120 ×3 (tclsh auto-loads it on every release) |
| `interp create -s b`, `interp create c -safe` (`i16`) | E003 too many arguments; the quick fix on `c -safe` deletes `-safe`, making the child unsafe; W129 fires inside `c` but not inside `b` |
| `interp eval $c parentProc`, `interp invokehidden $s pwd` (a literal word) | W312 "unbraced script argument may cause code injection"; for `invokehidden` the word is a command name, not a script |
| `inChild extra` where `inChild` aliases a zero-argument proc in the child (`i02`) | E003 (right) |
| `interp eval a {exec …}` in a safe child | W129 (right) |
| `interp limit`/`bgerror`/`debug` under 8.4, `children`/`cancel` under 8.5 | W002 (right) |
| `$c ev {…}` (a prefix of `eval`) | W307 non-literal command name (`$c eval` is accepted) |

**Navigation and editing:**

| Case | Answer |
| --- | --- |
| go to definition on the alias target `compute` in `interp alias {} viaChild $c compute`, with `compute` defined in both worlds (`i12`) | the **parent's** `compute` (wrong) |
| go to definition on a call of `viaChild` | the child's `compute` (right); references of the child's `compute` list the calls but not the target word |
| rename the parent's `compute` | also edits the alias target word, re-pointing the alias at a child command that does not exist (wrong edit) |
| rename the parent's `helper` with `$c alias helper helper` (`i11`) | edits only the definition, leaving the alias aimed at a deleted name (wrong edit) |
| rename a proc-local `x` used as `interp eval $c {set x 7}` and `interp eval $c {set x}` | edits the local and the second child body but not the first (wrong edit) |
| references of a proc-local `local` (`i11`) | include the child's `{set local}` words (wrong) |
| go to definition on `handler` in `interp bgerror $c handler` with a parent and a child `handler` | the parent's (wrong: the handler runs in the child) |
| rename `bump` with three `interp alias … {} bump` targets (`i02`) | all four words (right) |
| go to definition inside a child body for a parent-only command or variable | nothing (right) |
| optimiser around `interp eval` | no rewrites; parent constants are not folded past `interp eval` (safe, conservative) |

The debug VM spends about 10 s on each unknown command and 26 s in `safe::interpCreate` (auto-loading), so several VM rows needed a 120 s timeout; the answers are then right.

## Census

**Engines** (no shared implementation beyond the hidden-command roster):

- VM, about 4,000 lines: `rust/tcl-vm/src/command/child_interpreters.rs` (the `interp` ensemble; `share | transfer => ok(Value::empty())` at `:586`; any other unknown arm, including `cancel`, goes to `refuse_host_command` at `:587`), `interp/interpreters/{aliases,child_command,children,hidden_commands}.rs`, `interp/limits.rs`, `interp/native_interpreter_paths.rs` (path lists), `make_safe` at `interp/interpreters/children.rs:491`.
- Runtime, about 4,250 lines: `runtime/rust/src/cmd_alias.rs` (dispatch `:329-485`; `only_single_interp` at `:489`; unknown arms report `CommandProtocolUnavailable("native interpreter worker")`, which the runner turns into a refusal), `cmd_alias/{child_interpreters,hidden_commands}.rs`, `interp/{native_children,native_hidden_invocation,limits,child_interpreters}.rs`, `interp/commands/hidden_commands.rs`; `interp/limits.rs:26-28` documents that `commands` limits are "stored for query/set only"; `make_safe` at `interp/child_interpreters.rs:619`.
- Shared: `tcl_registry::safe_interp_hidden_commands` (`rust/tcl-registry/src/cache.rs:501`, the `SAFE_INTERP_HIDDEN` trait), called by both `make_safe`s, and the `tcl_runtime_api::Aliases` trait (alias-prefix introspection only, `rust/tcl-runtime-api/src/commands.rs:106`). There is no interpreter trait: creation, path resolution, eval, alias dispatch, hide/expose, limits and channel sharing are written twice.

**Registry:** `rust/tcl-registry/src/commands/tcl/interp.rs` — the `interp create` option scan as C reads it, with prefixes and options after the path (`interp_create_state_transitions`, `:234`), transition descriptors for create, delete, marktrusted, recursionlimit, bgerror, hide, expose (domains `CommandBindings`, `InterpreterTopology`, `InterpreterPolicy`), `eval` as a body in `BodyInterpreter::Argument(1)` with `EVALUATES_CODE | SCRIPT_CONCATENATES_ARGS` (`:717`), alias and bgerror command prefixes, `taint_interp_eval_subcommands` for T105 (`:1108`), and a separate `jim_spec`. The hover text is the only description of the world boundary.

**Analysis** (about 46 production files in `tcl-compiler` touch interpreter transitions or the child domain):

- The analyser walk's model: `rust/tcl-compiler/src/analyser/handlers/interpreters.rs` — `interp_path_key` (`:29`, a path normalised with `split_whitespace`, not a list reader), `parse_interp_create_words` (`:55-89`, exact `-safe` only, unknown options skipped), the `InterpState` map, W140 for a never-created child, and `handle_interp_eval_command_original` (`:188`), which opens a synthetic `@interp@<key>[#<epoch>]` domain (`:496`) for a single literal body and skips multi-word bodies.
- Safe-interpreter visibility: `analyser/interp_visibility/{child_visibility,command_transitions,lambda_bodies,source_loads,source_schemas}.rs` (about 1,800 lines) — hide/expose deltas, W129, `safe_interp_stack`.
- Aliases: `analyser/handlers/command_aliases.rs` (`handle_interp_alias`, `:40`) and `rust/tcl-compiler/src/alias.rs` (the lowerer's alias table).
- Command binding's own interpreter model: `command_binding/interpreter_manufacture.rs` (the parent command a child creates), `source_transition_advice/{interpreter_path,interpreter_handle}.rs`, `module_bindings/{invocation_transitions,evaluated_bodies}.rs`; the world-state SSA (`world_state_ssa/transition_intents.rs`, `state_ssa.rs`, `effect_ssa.rs`) and `dispatch_proof/state_transitions.rs` consume the registry transitions.
- Elsewhere: `analyser/per_item/isolated_body.rs`, `analyser/tk_checks.rs` (keys Tk state by the `@interp@` identity), `diagnostics/security.rs` (W312), the arity checker (E003, from the form `interp create ?-safe? ?--? ?name?`).
- LSP: `tcl-lsp-core/src/workspace_index/source_views.rs` and `tcl-lsp-server/src/readonly_reference_reports/source_views.rs` see the `@interp@` domains; references and rename otherwise use the general variable and command indexes.
- Tools: `tcl-spec-hooks/src/pack_eval.rs:128` and `sandbox.rs` deny `interp` with their own tables.

**Duplicated rules and where they disagree:**

- `interp create`'s words are read three times: the registry's C-faithful scan (`-s` is `-safe`, options may follow the path, `-bogus` fails), the analyser's `parse_interp_create_words` (exact `-safe`, skips unknown options), and the arity checker's form synopsis (at most one word after options). On `interp create c -safe` the first says safe, the second safe, the third "too many arguments"; on `-s b` the first says safe, the second says a child named `-s`, the third "too many arguments".
- The child's world is modelled twice: command binding's interpreter transitions (`CommandBindings` + `InterpreterTopology`) and the analyser's `@interp@` domains with `InterpState`. W123 on `kid` shows the analyser's command check does not see the parent command that `interpreter_manufacture.rs` installs (inference: the W123 path reads the analyser's tables, not binding's).
- Interpreter paths are read by a list reader in each engine and by `split_whitespace` in the analyser.
- The world boundary for editing is decided per feature: go to definition respects it for calls, not for alias target words or `bgerror` handlers; references and rename do not respect it for variables.
- The unsafe-command roster: the registry trait (owner), the `tcl-spec-hooks` deny tables, and the iRules policy list.

## Design

- **An interpreter is part of the name world's identity.** The [design integration](design-integration.md) decision 2 frame is keyed by (body unit, entry context); a child interpreter adds a level above that: every command, namespace, variable place and frame identity carries an `InterpreterId` (creation site plus incarnation, like `CommandRef`, with a display path derived from it). The `@interp@<path>` string domain becomes that identity; `interp_path_key` and the synthetic names go. A child world starts from the release's builtin roster (narrowed by `SAFE_INTERP_HIDDEN` for `-safe`), never from the parent's world.
- **The registry describes which world each word lives in.** Extend the evaluation-context descriptors (cross-cutting fault 9) with an interpreter selector: `interp eval path body` → body in `path`; `interp alias src name target cmd` → `name` is a command slot in `src`, `cmd` is resolved in `target` at call time; `interp bgerror path cmd` → `cmd` in `path`; `interp hide/expose/invokehidden path cmd` → hidden slots in `path`; `$child eval`, `$child alias`, … the same through the child's command. Binding, go to definition, references and rename then read the selector instead of deciding per feature. A word whose interpreter is not proven yields `Answer::Unknown` and editors refuse with a reason.
- **One reader per grammar.** `interp create`'s option scan lives in one place (the registry's resolver, which is already C-faithful) and the analyser, the arity checker and the quick fix read its result; the form synopsis must not drive E003 for this command. Interpreter paths are read with the dialect's list reader.
- **One interpreter model in analysis.** Command binding's transitions (`InterpreterTopology`, `InterpreterPolicy`, `CommandBindings`) are the owner; the analyser's `InterpState`, its visibility snapshots and the `@interp@` domains become views of it when the analyser walk is folded into binding (architecture phase 5). A body in another interpreter is a body unit with entry context "global frame of interpreter I" (phase 4); its procedures are ordinary procedures in that world, so their formals are bound.
- **Deferred and cross-world execution.** Alias targets, `bgerror` handlers and `limit -command` callbacks run in a named interpreter at a deferred point (decision 3), joined over that interpreter's world, not the caller's. The potential-execution owner (decision 8) treats `interp eval` with an unknown path as possibly reaching any child world and none of the parent's names.
- **Engines.** An `Interpreters` trait in `tcl-runtime-api` (create, delete, resolve path, eval in, alias across, hide/expose, limits, share/transfer channel, bgerror, cancel) with conformance rows for every subcommand; both engines implement it, and unimplemented operations raise a catchable Tcl error naming the operation instead of stopping the run.
- **Conformance.** The sixteen fixtures, plus the wrong-argument and path cases, become rows run through tclsh 8.4–9.1, both engines, the analyser and every editor feature.

## Plan

- **Fix the engine faults with their fixtures** (owners: `rust/tcl-vm/src/command/child_interpreters.rs`, `runtime/rust/src/cmd_alias.rs`, `runtime/rust/src/interp/limits.rs`): VM `share`/`transfer` implemented (or a catchable error), VM `interp cancel` cancels only the target and returns `{}` when idle, runtime parent→child aliases, runtime command-limit enforcement, runtime `errorCode` propagation across `interp eval`, runtime `share`/`transfer`/`cancel`. Done when `i02`, `i06`, `i07`, `i10`, `i12` and `i14` match tclsh under both engines.
- **Alias wrong-argument wording** in both engines: from 8.5 the error names the alias, as C does by calling `TclInitRewriteEnsemble` in `AliasNRCmd` and `AliasObjCmd` (`generic/tclInterp.c:1825`, `:1876` in 8.6.18). Done when `i15` matches tclsh 8.4 and 8.6+ under the respective pins.
- **Unimplemented subcommands fail as Tcl errors:** replace the VM's `refuse_host_command` arm and the runtime's `CommandProtocolUnavailable("native interpreter worker")` with a catchable error that names the subcommand; the message "native handler policy is not selected" goes. Done when `catch` sees the failure in `i06` and `i14`.
- **`safe::` in the runtime runner and a load-time budget for the VM:** load the standard library (or document the runner as library-free) so `i09` runs; measure the VM's 26 s `safe::interpCreate` and ~10 s per unknown command.
- **One `interp create` reader** (owner: `rust/tcl-registry/src/commands/tcl/interp.rs:234`): delete `parse_interp_create_words` and `interp_path_key` in `rust/tcl-compiler/src/analyser/handlers/interpreters.rs`, read the transition's path and safety instead, and stop the arity checker and its quick fix from applying the synopsis to `interp create`. Done when `i16` gives no E003, W129 fires in `b`, and no fix deletes `-safe`.
- **Interpreter selectors in the registry:** add the per-argument interpreter selector to the evaluation-context descriptors for `interp eval`, `alias`, `bgerror`, `hide`, `expose`, `invokehidden`, `limit -command`, `target`, and the child-command ensemble. Done when every `interp` subcommand that takes a command, script or variable word has a selector and a unit test.
- **Child bodies as body units in their own world** (with architecture phase 4): procedures defined in a child body bind their formals, and the body's variables live in the child's global frame. Done when the W210s in `i05` and `i10` disappear and references of the proc-local `local` in `i11` exclude the child body.
- **Editor features read the selector** (owners: `tcl-lsp-core` definition, references and rename): alias target words resolve in the target interpreter, `bgerror` handlers in the child, rename edits only words in the renamed identity's interpreter and refuses with a reason when the interpreter is unknown. Done when every row of the navigation table above is right or a stated refusal.
- **`kid` is a command after `interp create kid`:** W123 and hover read binding's `interpreter_manufacture` result. Done when `i03` has no W123.
- **W120 for auto-loaded library commands:** `safe::*` (and other `tclIndex` commands) are available without `package require` on every release; the package roster marks them auto-loadable. Done when `i09` has no W120.
- **W312 only where injection is possible:** a single literal word with no substitution is not an injection site, and `invokehidden`'s command word is not a script. Done when `i01`, `i02`, `i04`, `i05` lose those W312s and `interp eval $c $user` keeps it.
- **An `Interpreters` engine trait** in `tcl-runtime-api`, with both engines implementing it and the copied subcommand dispatch, path resolution and hide/expose code moved behind it. Done when the subcommand table exists once and the interps rows run through the trait in both engines.
- **Interps vectors in the conformance matrix:** the sixteen fixtures as rows for tclsh 8.4–9.1, both engines, the analyser and go to definition, references and rename.
