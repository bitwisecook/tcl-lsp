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

**Foundations first:** the [cross-cutting review](cross-cutting-review.md) finds the same faults in every domain study below — no semantic identity layer, no query layer, no common meaning for "unknown", release policy copied instead of owned, conformance data that drives only one side, kernels that cannot take partial knowledge, an IR that loses structure, incomplete coverage of reachable code, semantics hard-coded outside the registry, two drifting engines, silent failures, spelling-only gates, stale contracts and ad hoc measurement. Build those foundations first (probe harness and conformance matrix, answer and outcome types, identity layer, then release facts, registry descriptors and abstract-value kernels); the domain items then reduce to moving consumers onto owners and deleting copies (items 74–91 number these foundations). Its correctness backlog — wrong answers in both engines and in the analyser — is fixed first, independent of the refactor.

**Overall order:** [tidying the architecture](architecture-tidy-plan.md) sets out one pipeline with one set of facts, the shared structures, the crate layers, a single caching layer, and the phases that retire the duplicate walkers and resolvers; the plans below are parts of it, including [call frames](frame-design.md), [namespaces](namespace-design.md), [expressions](expression-design.md), [packages](package-design.md), [rename and alias tracking](rename-alias-design.md), [scripts, words and writers](words-scripts-design.md) and [lists, dicts and arrays](lists-dicts-arrays-design.md).

**First:** centralise command lookup — one lookup-frame value, registry-declared evaluation contexts, one lookup owner with one answer type that every diagnostic, editor feature and the optimiser ask, and consumer conformance vectors. Evidence and steps: [command lookup is not centralised](command-lookup-centralisation.md). Most items below are symptoms of it. Variable lookup has the same fault and should share the frame value: [variable lookup is not centralised](variable-lookup-centralisation.md); the false W210s of item 11 and the missing TclOO, `apply`, `uplevel` and `upvar` array navigation come from it.

The shared causes behind items 1–58 — one frame lookup context, one per-invocation record for nested commands, one instance value, fine-grained transitions, one class fact with option tables on methods, value grammars — are set out with their evidence in [TclOO and Tk naming: root causes](oo-tk-root-causes.md); fix those before the individual symptoms.

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

Lists, dicts and arrays (evidence and full steps: [lists, dicts and arrays](lists-dicts-arrays-design.md); fixtures in `oo-dispatch-probe/lists-dicts-arrays/`):

42. Fix the engines: `lremove` (Tcl 9.0) is missing from `tcl-cmd-core` and the VM though the registry declares it shipped; the runtime prints an empty line for `puts $d` after `dict with` changes `d` (stale string representation); the runtime lacks `array statistics`.
43. One evaluator: the registry's `const_fold.rs` and SCCP's `llength` fold go through `tcl-cmd-core` like value transfers already do; `canonical_dict_slots` for `dict with` keys; keep the optimiser's constant list and dict folds (11 of 12 fixtures fold correctly given a dialect; the earlier "none" came from a probe without one) on the shared evaluator.
44. One index grammar per release in `tcl-cmd-core` (today two inside it, a dead one in `tcl-bytecode` and an ad hoc one in type inference) and one option table per command shared by the registry and `tcl-cmd-core`.
45. One container model (list lengths and element shapes, dict key sets, array element sets; one `PlaceKind` with dict paths in production) for SCCP, intervals, type inference, taint, W230, editor dict-key hover/completion and `switch_to_dict`; one `dict with`/`update` binder model with declaration spans; fix the false W210 after `array unset` with a pattern; add an array/scalar misuse diagnostic; run the captured Tcl list/dict tests and engine vectors through the analysis folds.

Caching, indices and lookup layers (evidence and full steps: [caching](caching-design.md)):

46. Measure and bound: sizes of every static and memo behind a debug-memory command; the per-analysis timing and memory probe plus a pack-churn loop in the analysis-performance suite (today 17 ms for two lines, 1.27 s for a 687-line file, about 230 MB retained after ten distinct analyses); cap the generation-keyed ingress maps and overlay-0 assembly entries and stop leaking per generation (#2543).
47. Registries, pack sets, rosters and profiles as salsa inputs, built once per revision and projected per profile as queries (today `project_for_profile` rebuilds the default registry and two static defaults exist).
48. Memos inside analysis values and process-global compiler memos become tracked queries or per-query scratch; retire `RetainedModules`' private LRU inside a salsa input.
49. One analysis pipeline (fold the fresh-analysis, workspace-scan, recovery, class and provider analyses into salsa tiers), secondary indices as tracked queries, per-body re-analysis on edits, interned identities in the database, and gates against new memos and leaks outside it with determinism, growth and latency budgets.

Analysis data structures and a typed IR (evidence and full steps: [analysis data structures](analysis-data-structures.md)):

50. Annotated commands in the IR: the resolved `CommandRef`, the selected form, and per word its role, place, value type or domain and body unit, computed once from the registry and packs (today about 990 production lines re-ask the registry for roles by command name and argument strings); `word_exprs` mandatory, flat `args`/`argv_texts` retired.
51. Typed keys in every pass: GVN keys over command references and operand values instead of `Vec<String>` spellings; SSA uses and defs as places and symbols; SCCP constant sets with a typed value; blocks by `BlockId` everywhere (dead stores today parse the `_N` suffix of block names); symbol- or place-keyed interval, escape and parameter sets; enums for finding reasons, labels and the top-level function.
52. Shared structural queries: cached predecessors and dominance (today `predecessors()` rebuilds a map on each of about 18 calls), offset-insensitive `ExprNode` equality instead of comparing rendered text, dense vectors and bitsets.
53. Type-level gates: no new `String`-keyed maps of commands, variables, blocks or namespaces in analysis passes, no comparisons of rendered text, no parsing of display names.

User procedures, objects and libraries (design: [user and library commands](user-and-library-commands.md)):

54. One command-contract type (identity, forms, per-parameter roles with frames and evaluation contexts, result type, effects, provenance) shared by the registry, packs, stubs and inference, carried on annotated commands.
55. Inferred contracts as salsa queries per body unit and call-graph component: roles from parameter traits (caller-frame variables, caller-frame scripts, command prefixes, expressions), types and domains from uses, results from returns, effects from the frame-effect and interprocedural summaries; consumed by W210, W123, E002, navigation and the optimiser.
56. Library contracts: installed source packages found by package discovery analysed per version and dialect and cached on disk by source hash; declared contracts for binary and critcl extensions; class facts from object-system profiles giving method contracts of the same shape.
57. Declared and inferred in one format: export inferred contracts as SpecTcl (`spec-author`, Spec Studio `import_package`); declared contracts win for consumers and inference reports contradictions.
58. Integrate contracts with the naming owners: selection through positioned binding answers, inferred contracts on publication and declaration receipts, parameter roles as naming purposes and effects as existing transitions, Logical authority only, existing partial receipts folded into contract views, owner-manifest and consumer-matrix rows ([integration](user-and-library-commands.md#integration-with-the-naming-architecture)).

Constants, constant folding and procedure folding (evidence and full steps: [constants and folding](constant-folding-design.md); fixtures in `oo-dispatch-probe/constants/`):

59. One constant value: `ExactValue` (bytes, numeric reading, representation) in SCCP, the expression folder, static loops, return summaries and `ConstOps`, replacing about ten constant types and three literal parsers; float spelling decided once by the release policy.
60. One evaluator per command: registry folds for `string` case conversion, `reverse`, `map`, `format`, `scan` and `concat` call `tcl-cmd-core`; codegen's `fold_const_branch` (release-blind number parsing) and `concat_cmd` (`str::trim`) use the shared folders; one `format` path for every entry point; the open `string compare --`/`equal --`/`string is` wrong folds from the test-scrub handover fixed.
61. One SCCP: the diagnostic "original value facts" solver and the main solver share their fold functions, then merge.
62. Specialised procedure summaries: fold calls with constant arguments, recursion under a fuel budget, constant procedures inside `expr`, and cross-file and library calls through contract queries (today O103 folds zero-argument, single-file, non-recursive calls only); bounded constant loops through `static_loops`; one boolean spelling for `ConstantReturn`.
63. Whole-program absence facts (no `tcl::mathfunc` override, no `tcl_precision` write, no intervening `unknown`) to unlock math-function folds and 8.5–8.6 float folds where proven.
64. No silent no-op entry point: `optimise()` and `optimise_with_dialect(…, None)` fold nothing; remove them or make "release unknown" fold what every release agrees on; no folder defaults to 9.0 when the release is absent.
65. Execution-checked fold conformance as a committed test: every fold fixture run before and after rewriting under every tclsh and jimsh per release (today the registry differential is mostly 9.0-only and the optimiser coverage values were checked by hand).

Name resolution end to end (validation of the plans above against two code inventories: [name-resolution validation](name-resolution-validation.md); fixtures in `oo-dispatch-probe/name-validation/`):

66. Split the frame value: `Frame` per activation (identity, kind, namespace identity, level relation, receiver) and `NameWorld` at each program point (path, imports, aliases, defined commands, variable links, instance variables, traces) from the one binding walk; both lookup owners take a frame and a point and return one `Answer<T>` with provenance and authority, extending `SourceInvocationBinding` (which evolves into the annotated command) and `SsaSourceView::read_reference`.
67. A reverse index per identity derived only from forward answers at every site (proven and possible kept apart), replacing `fallback_proc_by_simple_name`, `invocation_references_named`, the minifier's `find_proc_call_sites` and the per-feature variable models behind references, rename and highlights.
68. The cross-file name world: files and packages export contracts and transitions; binding applies them at `source`, `package require` and workspace assumptions; the workspace index stores exports and the reverse index and stops resolving (`command_settlement.rs`, `namespace_imports.rs`).
69. One candidate generator: the byte `NativeNameProjection` with per-interpreter recipes; relative `namespace path` entries made absolute when bound; the string candidate family becomes display-only, then goes; every resolver that passes no path (`alias.rs`, `type_infer.rs`, class resolution, the LSP's class lookup) moves to the owner.
70. Variable name text: the name-end rule in `tcl_core_types::naming`, one variable-reference grammar per release and dialect in `tcl-lexer` (`word_parts::scan_var_ref`) built on it, both re-exported through `tcl-syntax::naming`, one element splitter in the lexer taking the selected protocol, one `static::` rule; retire the three scanners, four splitters, about ten `contains('(')` tests and the optimiser's text counters; add a drift gate.
71. Builtins only through a resolved `CommandRef` (no registry lookup by written word in analysis, codegen or the LSP); one subcommand-prefix rule; one import-pattern matcher (`NativeGlobProtocol`).
72. Engine resolution once: one lookup loop per engine over a `tcl-runtime-api` namespace-walk trait (today two in the VM and one in the runtime), one execution name policy with live-object adapters (today 970 and 772 lines), `tcloo_linearise` in both engines (the runtime has its own C3), one level grammar, one compiled-locals decision read by codegen, the bytecode, the VM and the runtime; engine caches invalidated by the shared identity tokens on the same events.
73. Name caches in three salsa tiers (per-body binding facts and forward answers, per-file and per-package exports, the reverse index); delete the analyser and signature-scan maps, workspace link maps, `VariableSymbolMap`, the per-URI class analysis and SCCP's `frame_clobbers` memo; in architecture phase 3 the analyser walk and signature scan stop resolving and only produce declarations; add the minifier, codegen, MCP, graphs, explorer, diagram, SpecTcl static drive and test generators to consumer conformance.

Foundations, architecture and object systems (the structural work the domain items depend on; evidence and detail in the [cross-cutting review](cross-cutting-review.md), the [architecture plan](architecture-tidy-plan.md), [root causes](oo-tk-root-causes.md), the [object systems plan](object-systems-plan.md) and the [overrides review](overrides-package-review.md)):

74. Decide the licence (Tcl or `MIT OR Apache-2.0` instead of AGPL-3.0-or-later) before phase 1, then change `LICENSE`, every `Cargo.toml`, the source headers and their checker, the xtask banner, AGENTS.md, `deny.toml` and the marketplace manifests; set the dependency licence policy.
75. Probe harness and conformance framework (faults 5 and 14): vector tables as rows, implementations as columns (tclsh 8.4–9.1, Jim, the VM, the runtime, the folder, analysis queries, each LSP feature, the optimiser, writers and tools), one reported test per row (`insta`, `libtest-mimic`); the `oo-dispatch-probe/` fixtures become rows and the temporary probes in its README become permanent runners.
76. Semantic gates (fault 12): the walker and resolver manifest with an only-shrinks `walker-ownership` ratchet, the crate-dependency rule (`cargo-deny` wrappers), the memo rule, the release-table lint and host-primitive lints (`split_whitespace`, `trim`, `.chars().count()`, `contains('(')`); contract claims linked to vector rows or tests (fault 13).
77. Identity layer (fault 1): interned, typed keys with incarnation or generation and provenance for command, namespace, variable place, frame, body unit, instance and package; display text derived and never parsed; the engines' tokens exposed in the same vocabulary through `tcl-runtime-api`.
78. Answer and outcome types (fault 3): one `Answer<T>` (proven, bounded set, absent, unknown with a reason) with provenance and naming authority, one editor outcome (done, or refused with a reason the user sees); transitions mark only what they touch as unknown (persistent maps such as `imbl`); abstentions measured separately from wrong answers.
79. Release facts owned once (fault 4), generated from or checked against the C Tcl and Jim sources and oracles: operator tables, math functions, bundled packages, numeral and escape syntax, float spelling, character model, level grammar, compiled-locals rule.
80. Registry descriptors (fault 9): variable, frame, command, namespace and instance effects; an evaluation context for every argument that holds a script, command prefix, lambda or name (caller frame, named namespace, global at call time, level-selected, object frame, scoped command overrides); value grammars; package and object-system identities; `namespace unknown` and `unknown` handlers in the name world.
81. Kernels with abstract-value interfaces (fault 6): `tcl-cmd-core` commands, index arithmetic and comparisons evaluated over value domains with unknowns and sets, used by SCCP, value transfer, intervals and the optimiser.
82. Structured words (architecture phase 2, fault 7): every command word lowered as structured parts with source maps; the 143 users of text scanners migrated; about ten walkers deleted.
83. Every body is a unit (phase 4, fault 8): methods, one-member definitions, `initialise`, `itcl::body`, deferred callbacks, caller-frame scripts (including parameters passed to `uplevel 1`/`eval [list …]`), `namespace eval` bodies and lambdas held in data, each with entry facts; lowering continues past unproven calls; library procedures are not specialised to the one in-file caller (the `overrides` I230).
84. One binding walk (phase 5): fold the analyser walk, the signature scan and `visit_analysis_executable_commands` into binding; one invocation record for every reached command, however nested (root cause 4); instance values with creation and destruction effects for objects, widgets, images and fonts (root cause 2); the typed class model.
85. The semantic model (phase 6, fault 2): one query API joining facts to source positions; diagnostics become rules over it; LSP features, MCP, the diagram and the explorer read it; the agreement harness covers every feature.
86. Object-system profiles (object systems plan, Order 1–7): profile facts replacing every `DefinerFamily` match; one class fact and one method-call checker; reshaping, options, delegation and components; SpecTcl `object_system` support; TclOO, itcl, snit, Clay and Jim rewritten as shipped profiles; new profiles (stooop, XOTcl/NX, critcl, Tk megawidgets); inheritance and lookup order as code (`lookup::c3`, `lookup::dfs-keep-last`, `lookup::dfs-refuse-repeats`, custom), used by both engines in place of their own linearisations; receiver-style helpers usable by non-object definers (`overrides`' `nextOverride`).
87. Value grammars and code in data (root cause 6): grammars for option values and structured arguments with a generic option check; dictionaries of lambdas or command names walked as bodies and command references (the `overrides` dictionary).
88. Engine convergence (fault 10): complete `Frames`, `Namespaces` and command-identity traits; shared implementations behind them; one differential suite over both engines; the decision on converging their internal models recorded.
89. No silent failures (fault 11): every failure path carries a reason to the user or the log, and tests assert on reasons; the `overrides` false W214 and lost W301 around `"${name}::$x"` (`external-overrides/w214-reproducer.tcl`) fixed.
90. Crates and caching (phase 7): split `tcl-compiler` along the layers where that removes a cycle or a duplicate (`tcl-source`, `tcl-binding`, `tcl-ir`, `tcl-analysis`, `tcl-semantic`, `tcl-diagnostics`, `tcl-optimiser`); data and loaders below features, domain crates contributing data and hooks through interfaces, tools on the semantic model; all memoisation in salsa (items 46–49, 73); build time and binary size measured per split.
91. Third-party adoption by payoff: `imbl` and `slotmap`, `cargo-deny` wrappers, `insta` with `libtest-mimic`, a time-boxed `ascent` trial, `rowan`/`cstree` only if structured words show the need; replace hand-rolled generic code (AES/OpenPGP first, then line index, URIs, ANSI, JSON escaping, base32, punycode, CRC/Adler, similarity, DOT), each behind its stated condition.

**Coverage and order.** Every study in this branch's plan set maps to numbered items:

| Study | Items |
| --- | --- |
| TclOO, other class systems, critcl, methods versus procedures | 1–10, 86 |
| Non-OO dispatch, eval, global/namespace/alias trickery | 11–14 |
| Tk | 15–17, 84, 87 |
| Root causes (TclOO and Tk) | 66, 80, 84, 86, 87 |
| Command and variable lookup centralisation, validation | 66–73 |
| Overrides package | 80, 83, 86, 87, 89 |
| Call frames, namespaces | 18–25 |
| Expressions | 26–29 |
| Packages | 30–33 |
| Rename and alias | 34–37 |
| Words, scripts, strings, writers | 38–41, 82 |
| Lists, dicts, arrays | 42–45 |
| Caching | 46–49, 73, 90 |
| Analysis data structures and the typed IR | 50–53 |
| User procedures, objects and libraries | 54–58 |
| Constants and folding | 59–65 |
| Cross-cutting faults 1–16 | 75–85, 88, 89 (16 → 50–53) |
| Parsers and walkers, crates and APIs | 76, 82, 84, 85, 90 |
| Third-party crates, licence | 74, 91 |
| Editor actions and tools | 100–106 |

**Order:** the phase order, the structural decisions that make the designs one design (binding and value fixpoint, `Frame` versus name world, one per-site record, identity types, summaries versus contracts, the answer type and receipts, the potential-execution owner, native linearisation recipes) and the dependencies behind them are in [design integration](design-integration.md); it supersedes the order previously given here.

Validation spikes, phase 0, before the phases that rely on them ([design integration](design-integration.md)):

92. Name-world cost: memory and time of per-point binding states (`ModuleCommandBindings`) on the 687-line file and the analysis-performance corpus.
93. Annotated commands for nested sites: build the per-site record for every reached command over `oo-dispatch-probe/frames/` from today's binding and compare with existing consumers.
94. Binding and value fixpoint: run the bounded loop on the `oo-dispatch-probe/dynamic/` dynamic-head, `upvar $n` and `uplevel $lvl` fixtures; count iterations and re-bound sites.
95. Per-body salsa query: move W210 for one body unit onto a per-body query and measure re-analysis after a one-line edit.

Architecture review of the earlier decisions together with the new design ([architecture review](architecture-review.md)):

96. Owner decisions first: freeze the 2,158-unit catalogue and re-map each unit to keep, delete with its phase, or re-target to the phase owner (no caller migrated onto an owner scheduled for deletion); and fix the release scope (recommended: release the LSP from current scope, engine convergence, crate split, salsa rework and licence after release).
97. One positioned record: `SourceInvocationBinding` evolves into the annotated command; `invocation_at_source` drops its unused `head` parameter (403 call sites); the roughly 30 positioned command, namespace and variable queries become facets of that record keeping their authorities.
98. Authorities and receipts capped: one fixed authority enum (Display, Logical, Native, normal completion, Edit), one receipt type per authority with a shared currency key replacing per-module custody checks; a Display authority for written-word fallbacks used only by display features; a gate requiring a manifest reason for any new `Original*`/`Logical*`/`Native*`/`Source*` type (2,079 today).
99. Contracts by question, with evidence: reorganise `name-handling.md` (4,611 lines), the consumer matrix and the owner manifest by question rather than consumer; delete per-consumer sections as consumers migrate; tie each remaining claim to a vector row or test; retire the string candidate family's canonical status and oracle-less proof records as acceptance evidence; assign one owner per phase for cross-cutting changes.

Editor actions and tool equivalents ([editor actions](editor-actions-design.md); benchmark and fixtures in `oo-dispatch-probe/editor-actions/`):

100. Fix rename's precision now (release scope): `source` with a literal or evaluable path (including `[file join [file dirname [info script]] …]`) imports the sourced file's exports instead of suppressing navigation in the whole file; rename coverage and collision checks scoped to the files and bodies that can see the identity, so an unrelated workspace file or unknown effect no longer refuses every rename; `prepareRename` returns the refusal reason instead of `null`.
101. Separate binding and full-pipeline tiers per body unit: editor point and collection queries read only binding facts (annotated commands, name world, places, exports) and never wait for SSA/SCCP/diagnostics; a one-line edit re-binds one body unit (today the whole file: 12.5 s for 5,451 lines).
102. Positioned lookups through an interval index over each body's annotated commands, so point queries cost no more on a large file (today 1.6–3.4 s per request on `http.tcl` after analysis).
103. A persistent workspace tier: per-file declarations, exports and reverse-index contributions from the binding tier, cached on disk by content hash, dialect and pack key; the cold scan builds only that tier (today 58 s for 10,513 lines); requests never re-analyse other files (today 20 s for requests on a 15-line file in that workspace); incremental inventory refresh instead of a rescan on rename.
104. One action engine over a session (workspace root plus overlays) used by the LSP server, MCP and the CLI; the server's rename tiers move into it; MCP keeps sessions instead of a fresh single-file analysis per call; the agreement harness requires identical edits or refusals across surfaces.
105. Cancellation, `workDoneProgress` and post-edit verification: superseded requests cancelled through salsa; progress for workspace renames and scans; every rename re-binds the edited body units and checks that each edited site now names the renamed identity and nothing else changed target.
106. Action budgets as gates: the editor-actions workspaces in the analysis-performance suite with per-class targets (point query ≤ 50 ms at p95 warm, collection query ≤ 300 ms on 10,000 lines, local transform ≤ 100 ms, point query ≤ 100 ms after a one-line edit) on release builds.

Before the final PR, move useful contracts and real experiments/tests into durable locations. Correct documentation/comments across the entire tracked repository, including generated guidance and files outside the implementation diff. Remove obsolete guidance, completed TODOs, development narratives, temporary ledgers/prompts/archives and their references, including this directory and the work-specific AGENTS exception. Preserve reproducible evidence and legal attribution. Complete all gates before creating a ready-to-merge PR; do not merge. This final cleanup is still required and is not completed by source-audit closure.
