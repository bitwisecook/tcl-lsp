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

The shared causes behind items 1–17 — one frame lookup context, one per-invocation record for nested commands, one instance value, fine-grained transitions, one class fact with option tables on methods, value grammars — are set out with their evidence in [TclOO and Tk naming: root causes](oo-tk-root-causes.md); fix those before the individual symptoms.

Build items 1–10 on one object model specialised per system through registry profiles and SpecTcl, as set out in the [object systems plan](object-systems-plan.md): profile facts replace every `DefinerFamily` match, source and registry classes share one class fact and one method-call checker, objects are values on one path, and member bodies get the procedure pass. New profiles follow for itcl, snit delegation, stooop, XOTcl/NX, critcl and Tk megawidgets.

Non-OO evaluation, namespace and alias gaps (queue item 4 of the handover, fixtures in `oo-dispatch-probe/dynamic/`):

11. Remove false W210 for variables set by `catch` bodies, `regexp` match variables, a callee's `upvar`, `subst` command substitution and `eval` of a literal `list` command.
12. Exempt fixed-signature callbacks (variable traces, ensemble `-unknown` handlers) from W214, and drop the nonsense "did you mean '!'" suggestion on W123.
13. Model 8.x namespace variable "creative writing" (`set x` in `namespace eval` writes an existing global on 8.4–8.6, not on 9.0) per selected dialect.
14. Report: `namespace code` captured in the wrong namespace, reads of never-set globals, reads after `unset`, calls through a deleted alias target, arity of procedures defined inside procedures, writes to a 9.0 `const`, and calls to a built-in replaced by a procedure with a different signature.

Tk gaps (queue item 5 of the handover, fixtures in `oo-dispatch-probe/tk/`):

15. Check options and enumerated values wherever Tk takes them, not only at creation: `configure`, `itemconfigure`, `entryconfigure`, `tag configure`, geometry managers, `wm`, dialogs, `font`, `image create`, canvas item types and coordinate counts, text indices, and event and virtual-event names; through the registry's widget and option specs.
16. Follow widget paths as values: paths built from variables (`$w.ok`), widget commands held in variables, `image create` and `font create` results; report duplicate paths, upper-case names and use after `destroy`; drop the W307 noise on these.
17. Report `-textvariable`/`-variable`/`-listvariable` naming a procedure local (Tk links the global) and arity in `after` scripts; gate `tk print` to 9.0 and check its arguments.

Before the final PR, move useful contracts and real experiments/tests into durable locations. Correct documentation/comments across the entire tracked repository, including generated guidance and files outside the implementation diff. Remove obsolete guidance, completed TODOs, development narratives, temporary ledgers/prompts/archives and their references, including this directory and the work-specific AGENTS exception. Preserve reproducible evidence and legal attribution. Complete all gates before creating a ready-to-merge PR; do not merge. This final cleanup is still required and is not completed by source-audit closure.
