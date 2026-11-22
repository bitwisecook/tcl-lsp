# Naming completion plan

The source/role audit is complete for **110,709 tracked baseline paths**, with explicit current-source annexes and zero unexamined paths. The reconciled catalogue contains **1,321 caller migrations**, **377 shared-owner repairs/extensions** and **101 metadata/target-schema changes**: **1,799 semantic code changes**. There are separately **305 validation-code repairs** and **54 authored-documentation corrections**, making **2,158 confirmed required changes**. The exact catalogue records integrated/partial/remaining status separately. Examined requirements and original/provider provenance questions remain separate.

Searchable report · Full JSON catalogue · [Current validation](current-state.md) · [Implementation obligations](implementation-ledger.md)

Active priority is every LSP consumer plus its necessary shared Compiler/DB/Registry owners. Follow exact catalogue units for complete source/configuration/provider/selection and typed readonly/transform purposes. Runtime/VM and BIG-IP migrations and dedicated suites are deferred, with existing changes/results preserved. Uncovered BIG-IP behaviour uses the user's Tcl8.4 working assumption, separate from measured evidence. Parallel implementation covers original declaration/source geometry, genuine method/provider context, exact cursor/source roles, source-action/proposal currency and generic editor/project adapters. Do not repeat a provisional audit or count helpers/unchanged dependencies as migrations.

Root owns Git, integration, shared tracking and build freezes. Give parallel implementers bounded source/API ownership; coordinate shared files and never share a Cargo target across simultaneous builds. Follow repository environment/worktree owners and use one Cargo job where required. Commit and push coherent reviewed development work about every 15 minutes, with failures/unexecuted checks explicit.

Keep written source, evaluated names, namespace identity, command slots, live command identity, variable places/cells and display text distinct. Shared owners retain original bytes, spans, complete configuration, source ownership, namespace/frame/interpreter context and lifetimes/generations. A display string cannot reconstruct identity. Source advice grants no Native execution, physical cell, entered frame, absence, edit/optimisation authority or normal completion. Repair missing owner transport instead of weakening guards or adding String inverses.

For oracles, use original C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0, matching ORIGINAL init.tcl/Tcltest/support, and latest upstream Jim identified at execution. Existing 5bac7c99 evidence stays bounded to its capture. Record question, exact input/controls, provider/surface/reached phase, source/executable/library/harness hashes, argv, complete channels, exit/completion and limitations. Derive conformance expectations from actual oracle results.

Current counts, original controls, build/source/artifact bindings, actual provider captures, red/unrun gates and next actions are maintained in [current-state.md](current-state.md). Report progress against the audit every30minutes. No source review, compile check or assumed provider behaviour supplies ordinary execution, issue acceptance or repository-gate completion.

Review methods against procedures in the per-function pass: method bodies currently get no SSA, so SSA-driven diagnostics are silent in them, and procedure code after an unproven call stays unlowered. Give both unit kinds the same pass; see the [signed scan and precedence handover](handover/signed-scan-precedence.md).

TclOO and class-system coverage, measured against tclsh 8.6–9.1 (matrix in the same handover, queue item 2):

1. Walk the bodies that are currently never read: one-member `oo::define C method|constructor|destructor|self method`, `oo::objdefine o method` (objects and class objects), the 9.0 `initialise` script (run at definition time), and `itcl::body`.
2. Give method, constructor, destructor, `self`/`classmethod`/`private` method, `oo::objdefine`, `oo::singleton`/`oo::abstract`, itcl and snit bodies the procedure dataflow pass; the relative-head guard in `build_method_units` should read the receiver command-world proof instead of refusing every relative head.
3. Resolve `my`, `self`, `next`, and the 9.0 `link` and `callback` helpers through the object namespace and `::oo::Helpers`, removing the false W123 in entered bodies.
4. Keep classes typed after `superclass`, `filter`, `private variable` and later `oo::define` members, removing the spurious W307 on `[C new]`.
5. Report constructor arity at `C new`/`C create`, honouring TclOO's deletion of an empty-bodied constructor or destructor, and calls to methods removed by `deletemethod` or renamed by `renamemethod`.
6. Check method calls nested in scripts and substitutions (`catch {o m}`, `puts [o m]`) as at the top level; today named objects, widgets and `after` scripts are checked only at the top level, while procedures and `$o` handles are checked everywhere.
7. Follow object reshaping: `oo::copy`, `oo::objdefine o class`, `export`/`unexport`, superclass slot operations, unexported built-ins such as `eval`, and 9.0 `oo::configurable` properties.
8. Model namespace ensembles as dispatchers: `-map` (resolved in the ensemble's namespace), exports, `-subcommands`, `-parameters` and `-unknown` (whose handler formals are fixed, so no W214).
9. Carry arity through `tailcall`, literal `{*}` command prefixes and coroutine commands, and stop W241 on `while 1 { yield … }` generators.
10. itcl: accept `package require itcl` as well as `Itcl`, and check object methods, `chain` and `configure` options. snit: model `delegate`. critcl: add a model for `critcl::cproc` and `critcl::ccommand` (measurements in [`oo-dispatch-probe/`](handover/oo-dispatch-probe/README.md); queue item 3 of the handover).

**Overall order:** [tidying the architecture](architecture-tidy-plan.md) sets out one pipeline with one set of facts, the shared structures, the crate layers, a single caching layer, and the phases that retire the duplicate walkers and resolvers; the plans below are parts of it, including [call frames](frame-design.md), [namespaces](namespace-design.md), [expressions](expression-design.md), [packages](package-design.md), [rename and alias tracking](rename-alias-design.md) and [scripts, words and writers](words-scripts-design.md).

**First:** centralise command lookup — one lookup-frame value, registry-declared evaluation contexts, one lookup owner with one answer type that every diagnostic, editor feature and the optimiser ask, and consumer conformance vectors. Evidence and steps: [command lookup is not centralised](command-lookup-centralisation.md). Most items below are symptoms of it. Variable lookup has the same fault and should share the frame value: [variable lookup is not centralised](variable-lookup-centralisation.md); the false W210s of item 11 and the missing TclOO, `apply`, `uplevel` and `upvar` array navigation come from it.

The shared causes behind items 1–41 — one frame lookup context, one per-invocation record for nested commands, one instance value, fine-grained transitions, one class fact with option tables on methods, value grammars — are set out with their evidence in [TclOO and Tk naming: root causes](oo-tk-root-causes.md); fix those before the individual symptoms.

Build items 1–10 on one object model specialised per system through registry profiles and SpecTcl, as set out in the [object systems plan](object-systems-plan.md): profile facts replace every `DefinerFamily` match, source and registry classes share one class fact and one method-call checker, objects are values on one path, and member bodies get the procedure pass. New profiles follow for itcl, snit delegation, stooop, XOTcl/NX, critcl and Tk megawidgets.

A third-party package review ([overrides](overrides-package-review.md)) adds: caller-frame script parameters derived from `uplevel 1` in procedure bodies, value grammars for code inside data (override dictionaries of lambdas), body-scoped helpers outside object systems, scoped command overrides, a false W214/lost W301 around `"${name}::$x"` (reproducer kept), and whole-file specialisation of library procedures.

Non-OO evaluation, namespace and alias gaps (queue item 4 of the handover, fixtures in `oo-dispatch-probe/dynamic/`):

11. Remove false W210 for variables set by `catch` bodies, `regexp` match variables, a callee's `upvar`, `subst` command substitution and `eval` of a literal `list` command.
12. Exempt fixed-signature callbacks (variable traces, ensemble `-unknown` handlers) from W214, and drop the nonsense "did you mean '!'" suggestion on W123.
13. Model 8.x namespace variable "creative writing" (`set x` in `namespace eval` writes an existing global on 8.4–8.6, not on 9.0) per selected dialect.
14. Report: `namespace code` captured in the wrong namespace, reads of never-set globals, reads after `unset`, calls through a deleted alias target, arity of procedures defined inside procedures, writes to a 9.0 `const`, and calls to a built-in replaced by a procedure with a different signature.

Tk gaps (queue item 5 of the handover, fixtures in `oo-dispatch-probe/tk/`):

15. Check options and enumerated values wherever Tk takes them, not only at creation: `configure`, `itemconfigure`, `entryconfigure`, `tag configure`, geometry managers, `wm`, dialogs, `font`, `image create`, canvas item types and coordinate counts, text indices, and event and virtual-event names; through the registry's widget and option specs.
16. Follow widget paths as values: paths built from variables (`$w.ok`), widget commands held in variables, `image create` and `font create` results; report duplicate paths, upper-case names and use after `destroy`; drop the W307 noise on these.
17. Report `-textvariable`/`-variable`/`-listvariable` naming a procedure local (Tk links the global) and arity in `after` scripts; gate `tk print` to 9.0 and check its arguments.

Call frames and namespaces (evidence and full steps: [call frames](frame-design.md), [namespaces](namespace-design.md); fixtures in `oo-dispatch-probe/frames-semantics/` and `oo-dispatch-probe/namespaces/`):

18. Fix the VM: run `next`, `nextto` and chained constructors at the original level (as tclsh and the runtime do), accept `filter` in class definition scripts, and implement `info frame` or record it as a non-goal.
19. One level-word reader (`tcl_syntax::frame_level`) for the registry tables, the runtime's compiled-locals scan, the VM's error text and both analysis fallback policies; delete the uncalled `tcl_cmd_core::frame`; C-`int` range.
20. Frame selection and namespace operations in the shared engine traits (`Frames`, `Namespaces`), implemented once per engine; delete the copied selection wrappers, the duplicate `eval_uplevel` path, and the copied ensemble-object code and `namespace` subcommand tables; key the VM's command table by namespace token.
21. One analysis frame value carrying its namespace identity, chosen once in binding (today nine frame and six namespace choosers); keep `FrameLevel` through the IR; drop `BodyFrame`'s parameter-frame view of `namespace eval`.
22. One frame-effect model (the `cfg_builder` summaries) extended to callee `uplevel` writes, aliases that reach procedures and `catch` result and options variables, removing the measured false W210s; the parameter-trait and escape summaries become projections.
23. One analysis namespace identity (`SourceNamespaceKey` with incarnation) in the scope tree, `namespace_paths`, the item tree, the LSP index, rename and context helpers; model `namespace delete`; report calls into deleted namespaces; follow cross-namespace `rename` and ensemble subcommands for navigation; make hover agree with definition through imports.
24. One analysis model of namespace operations, in command binding on the registry's `NamespaceTransition` descriptors, replacing the analyser hooks, the signature-scan copy and the LSP's import and export logic.
25. Conformance: run the 172 namespace-operation vectors and the frame vectors and fixtures through both engines, command binding, the analyser (W210, W211) and the LSP (definition, hover, references); close the naming-layer leaks (string-prefix descendant tests, the ad hoc glob-literal check, copied helpers, codegen paths from display keys) under the drift gates.

Expressions (evidence and full steps: [expressions](expression-design.md); fixtures in `oo-dispatch-probe/expressions/`):

26. **Wrong answers first:** `in`/`ni` bind at the relational level in the shared parser (C Tcl: the equality level), so `$a == $a in $l` and `"b" eq "b" in {1}` give 0 instead of 1 in the VM and the runtime on every release, the analyser reports a false I230 "always false" for such a condition, and W154 explains the wrong grouping. Fix `binary_bp` and the dialect rows that copy it, and add the cases to the conformance vectors.
27. One precedence source (tcl-dialect's table, per release and Jim version) for the parser, `BinOp::precedence`, the renderer, the hover and W154/W155; a parse-render round-trip property test, since codegen's fallback and optimiser rewrites emit rendered text.
28. An expression-semantics vector table (the 61 probe cases plus a generated precedence matrix) through tclsh, Jim, both engines, the folder and the interval evaluator; fix both engines' missing `max`/`min` in 8.6 mode and W233 on float division by zero.
29. One floor division and modulus in `tcl-numeric`, one math-function list in tcl-expr feeding every other list, diagnostics and editor features reading the lowered expression instead of re-parsing, and production expression type inference.

Packages (evidence and full steps: [packages](package-design.md); fixtures in `oo-dispatch-probe/packages/`):

30. Fix the engine faults: the VM exits silently (status 1, no message, not catchable) while loading the 9.0 library's `msgcat`; the runtime's script runner has no standard library (no `auto_path`, `package unknown` or `::tcl::tm`) even with `TCL_LIBRARY`.
31. One package-version owner (`tcl-dialect`'s `tclPkg.c` port) for the version gate, the LSP document floor, SpecTcl, `tcl pkg discover`, MCP and SslicTcl, replacing the prerelease-blind registry comparator, semver where Tcl requirements are meant, and SslicTcl's comparator; package-manager requirements that keep Tcl's ranges and implicit major cap.
32. A package-name owner with registry-declared aliases (`itcl`/`Itcl`) and required versions, used by W120, the compiler's loaders and the server; per-package unknowns instead of dropping every W120 in a scope.
33. One discovery model with Tcl's search order (`TCLLIBPATH` first), per-dialect installations, `pkgIndex.tcl`, `tclIndex` and Tcl modules (`.tm`, absent outside the engines today); one `TCLLIBPATH` decoder; one bundled-package roster per release generated from each Tcl source tree's `pkgs/package.list.txt` and library; `tm.test`/`pkgMkIndex.test` slices and the version oracle across every remaining implementation.

Rename and alias tracking (evidence and full steps: [rename and alias](rename-alias-design.md); fixtures in `oo-dispatch-probe/rename-alias/`):

34. Keep aliases registered under their creation name after `rename` in both engines (tclsh refuses `interp alias {} new {}` for a renamed alias; the VM and runtime delete it); merge the VM's two alias call paths.
35. One command identity and one analysis model of rename and alias in command binding (today five identity schemes and about five rename models); dynamic alias prefixes become unknown instead of being dropped; report calls through an alias whose target has gone; navigate TclOO `forward` methods.
36. Editor rename that edits the operand words of `rename`, `interp alias`, `namespace export`/`import`, literal callback lists, qualified calls and `uplevel` bodies when the lookup owner proves them, checks collisions against every command kind, and refuses with a stated reason instead of an empty edit list (and `null` in MCP).
37. Execution-checked rename for every fixture as a required test, and a mutation column (`rename`, alias, import, forget, delete, recreate) in the command-resolution vectors run through both engines, command binding and the LSP; correct `command-binding-and-aliasing.md`, which says a source rename removes an import.

Scripts, words, lists, strings and writers (evidence and full steps: [scripts and words](words-scripts-design.md); fixtures in `oo-dispatch-probe/words-scripts/`):

38. Fix the VM: `{*}` applied to a literal braced word fails in some compile contexts (`native compiler preflight provider required`) and, in file mode, the VM exits with status 1 and no message; together these are the silent `msgcat` failure of item 30. Decide 8.6's system-encoding source reading.
39. One writer owner that takes a purpose, with `read(write(v)) == v` property tests per dialect; fix SslicTcl's `tcl_word` (a value ending in `\` becomes an unbalanced `{a\}`), then move Spec Studio, SpecTcl upgrade, f5-xc, the test generators, the optimiser, codegen inlining and the minifier onto it.
40. Release policy for every copied rule: escape widths in taint, the owner's `concat` in codegen (Rust's Unicode `trim()` folds away a non-breaking space Tcl keeps) and deferred writes, release-selected character counting in the registry, VM and `tcl-cmd-core`, lexer comment rules in the registry.
41. One segmentation including the F5 `else` merge (today only in the compiler segmenter); retire `structural_index`'s unused and display-only paths; replace `split_whitespace`/unchecked brace stripping of Tcl text in analysis with the list owner and structured words; add writers, brace stripping, whitespace splitting and character counting to the parser census and drift gates; fix W102 on literal `subst` templates.

Before the final PR, move useful contracts and real experiments/tests into durable locations. Correct documentation/comments across the entire tracked repository, including generated guidance and files outside the implementation diff. Remove obsolete guidance, completed TODOs, development narratives, temporary ledgers/prompts/archives and their references, including this directory and the work-specific AGENTS exception. Preserve reproducible evidence and legal attribution. Complete all gates before creating a ready-to-merge PR; do not merge. This final cleanup is still required and is not completed by source-audit closure.
