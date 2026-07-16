<!-- Companion to name-resolution-fix-plan.md. Ground-truth C-source study. -->

# Source-link legend (all file:line refs below are pinned to these commits)

Every `file:line` in this document resolves to a stable GitHub permalink by
prefixing the path with the matching base URL below and appending `#L<line>`.

**This repo (`bitwisecook/tcl-lsp`)** — pinned to the **v2.1.9** release commit
(the code these findings were validated against; this branch adds only docs on
top, so every `rust/…`/`runtime/…` line number is valid there):
`https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/<path>#L<line>`

**C Tcl (`tcltk/tcl`)** — pinned to the exact release tags studied:

| Tree prefix | Tag | Commit | Base URL |
|---|---|---|---|
| `tcl8.4.20/` | `core-8-4-20` | `9ccfe9d1…` | `https://github.com/tcltk/tcl/blob/9ccfe9d1b35741ff7323837f6485ffe48b06fad9/<path>#L<line>` |
| `tcl8.5.19/` | `core-8-5-19` | `160d612a…` | `https://github.com/tcltk/tcl/blob/160d612a6b2b1c2c0db27236d648b7bc1364570c/<path>#L<line>` |
| `tcl8.6.16/` | `core-8-6-16` | `874e4fe4…` | `https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/<path>#L<line>` |
| `tcl9.0.4/`  | `core-9-0-4`  | `c655b477…` | `https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/<path>#L<line>` |
| `tcl9.1b0/`  | `core-9-1-b0` | `fbe83207…` | `https://github.com/tcltk/tcl/blob/fbe83207a70634a5031c70bdce3d59071920f6da/<path>#L<line>` |

Strip the tree prefix (`tcl9.0.4/`) to get the in-repo path (`generic/tclVar.c`).
Example: `tcl9.0.4/generic/tclVar.c:4737` →
`https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclVar.c#L4737`.

The concrete, spot-verified anchors that drive the milestones (M7–M13 in the
fix plan) are given as full clickable permalinks in
[`name-resolution-fix-plan.md`](name-resolution-fix-plan.md); this document
keeps the exhaustive `file:line` form for density.

---

# Tcl Name Resolution: The C Algorithm, the 8.4→9.1 Version Matrix, and Rust Conformance

This document extracts the *real* name-resolution algorithm from the C Tcl source on disk (`tmp/tcl8.4.20` … `tmp/tcl9.1b0`, always the primary ground truth), states what actually changed across 8.4→8.5→8.6→9.0→9.1 for name resolution specifically, and judges whether the Rust implementation (canonical resolver / VM / runtime / analyser / registry) matches both the algorithm and the version differences. Every claim carries a `file:line` citation in **both** the C tree and the Rust tree. It is read-only analysis; nothing was edited.

A verifier re-checked a subset of findings against both trees. Those carry a **CONFIRMED** / **PARTIAL** tag. Everything else is investigation-only and tagged **PLAUSIBLE**.

---

> **Milestone numbers in §4–§5 predate the final renumbering.** Authoritative
> scheme: the 16-milestone index in
> [name-resolution-fix-plan.md](name-resolution-fix-plan.md). Remap of this
> doc's proposed items: "dialect-aware resolver" (D1/D9)→**M10**;
> "cross-version var semantics" (D4)→**M11**; "expr-function fidelity"
> (D7/D8)→**M12**; "TclOO version fidelity" (D10/N3)→**M13**; "trace-target
> references" (D11)→**M14**; "coverage" (N4/N1/N2)→**M15**; "VM parity"
> (N5–N8)→**M16**. Cross-refs to the earlier plan: M1(class)→**M4**,
> M4(variable)→**M2**.

## 1. The algorithm, from C

### 1.1 COMMAND resolution — `Tcl_FindCommand`

**Authoritative algorithm** (`tcl8.6.16/generic/tclNamesp.c:2528-2687`; qualified-name split `TclGetNamespaceForQualName` at `tclNamesp.c:2272`):

1. Choose the context namespace `cxtNsPtr`: global if `TCL_GLOBAL_ONLY` or the name starts with `::`, else the passed `contextNsPtr`, else the interp's current namespace.
2. Run any per-namespace / per-interp `cmdResProc` resolver hooks first; return immediately on `TCL_OK` or error.
3. **Path tier (8.5+ only):** if `cxtNsPtr->commandPathLength != 0` AND the name is not `::`-rooted AND `TCL_NAMESPACE_ONLY` is not set — search the current namespace first, then iterate `commandPathArray[i]` **in order** (`tclNamesp.c:2627-2642`), then global as a last resort (`tclNamesp.c:2649-2659`).
4. **Else** (no path / rooted / namespace-only): the classic two-slot search `nsPtr[0]` = context-derived, `nsPtr[1]` = global, first hit wins (`tclNamesp.c:2661-2682`).

`TclGetNamespaceForQualName` splits `a::b::c` into `(realNs, simpleName)` and is what makes a `::`-rooted name bypass the context search. 9.0 is identical apart from cosmetics (`tcl9.0.4/generic/tclNamesp.c:2640-2808`: `NS_DYING`→`NS_DEAD`, `int`→`Tcl_Size`).

**Version delta:** 8.4 has **no path tier** — `Tcl_FindCommand` goes straight to the two-slot search (`tcl8.4.20/generic/tclNamesp.c:1961-2050`); grep for `commandPathLength`/`commandPathArray`/`NamespacePathCmd` in the 8.4 file returns nothing. The path tier arrives 8.5 (`tcl8.5.19/generic/tclNamesp.c:2447`, `NamespacePathCmd` at `:197`) via TIP 229 (`tcl8.5.19/changes:6483`). The resolution **order** never changed 8.5→9.1; only the 8.4→8.5 boundary added the middle "path entries" tier.

**Rust conformance — FAITHFUL (shape).** The canonical resolver `rust/tcl-syntax/src/naming.rs:455-492` (`command_resolution_candidates`) pushes current ns at `:475`, path entries in order `:476-489`, global last `:490`, with a rooted short-circuit at `:460-462` — matching the C 8.5+ order exactly. `resolve_command_with` (`naming.rs:516-524`) is first-hit. The VM (`rust/tcl-vm/src/interp.rs:1658-1684`) and analyser (`rust/tcl-compiler/src/analyser/scope.rs:394`) both delegate to it.

**DIVERGENCE (CONFIRMED) — the path tier is not dialect-gated.** The resolver is path-parameterized, but nothing gates the path by dialect. See §2 and §4 (`command-c` gap). Registry *records* the boundary (`namespace path` = `TCL85_PLUS`, `rust/tcl-registry/src/commands/tcl/namespace_.rs:262-268`) but resolution never consults it. An 8.4-dialect file containing `namespace path {::foo}` has the path honoured; genuine 8.4 errors on the subcommand.

**NOT MODELLED — command-name caching / epoch invalidation (PLAUSIBLE, perf-only).** C caches a `ResolvedCmdName` on the name object (`cmdName` `Tcl_ObjType`), reused only while `cmdPtr->cmdEpoch == resPtr->cmdEpoch` and ns not dying (`tcl9.0.4/generic/tclObj.c:4340-4360`, `SetCmdNameObj` `:4401-4420`); epoch bumps on hide/rename/delete (`tcl8.6.16/generic/tclBasic.c:1858, 2802, 3161, 3295`). The Rust VM re-resolves every dispatch (`rust/tcl-vm/src/interp.rs:1658-1684`); no epoch field, no cached rep. This is a perf/staleness concern only — it does not change resolution order or correctness for static analysis.

### 1.2 VARIABLE resolution — `TclLookupSimpleVar`

**Authoritative bareword rule** (`tcl8.6.16/generic/tclVar.c:907-1011`, rule at `:907-913`; 9.0 `tcl9.0.4/generic/tclVar.c:924-1019`; 8.4 `tcl8.4.20/generic/tclVar.c:753-844`):

A bareword is looked up as a **namespace/global** variable iff (a) `TCL_GLOBAL_ONLY`/`TCL_NAMESPACE_ONLY` is set, OR (b) there is no active proc frame — global scope or a `namespace eval`/`inscope` body (8.4: `varFramePtr==NULL || !varFramePtr->isProcCallFrame`; 8.6/9.0: `!HasLocalVars(varFramePtr)`), OR (c) the name contains `::`. Otherwise it is a **frame-local**: search the compiled-locals array by exact string, then the frame's runtime var hashtable. `lookGlobal` (absolute `::name` or global-ns context) forces a `::` home; otherwise the namespace home is the current frame's `nsPtr`.

This rule is **byte-identical in semantics 8.4→9.1**; only the spelling of the "in a proc?" test changed (`isProcCallFrame` → `HasLocalVars`). No TIP altered it. (The frequently-cited "TIP 278" is a misattribution — grep for `278` across the changes files and `tclVar.c`/`tclNamesp.c` comments finds nothing relevant.)

**One genuine cross-version semantic change (CONFIRMED) — 9.0 removed the global fallback.** For an *unqualified undefined* name at namespace/global scope that is not `lookGlobal`:
- **8.4/8.5/8.6:** the `else` branch leaves `TCL_NAMESPACE_ONLY` unset (only sets it under `TCL_AVOID_RESOLVERS`), so `ObjFindNamespaceVar` runs its two-slot search `nsPtr[0]`=current, `nsPtr[1]`=global (`tcl8.6.16/generic/tclVar.c:5641`, `for(search=0;search<2;…)`). Current namespace **then global fallback**. `TclGetNamespaceForQualName` sets the alt (global) slot unless namespace-only (`tcl8.6.16/generic/tclNamesp.c:2241-2243`).
- **9.0/9.1:** the `else` branch **unconditionally** forces `flags |= TCL_NAMESPACE_ONLY` and `*indexPtr = -2` (`tcl9.0.4/generic/tclVar.c:935-938`, identical `tcl9.1b0/generic/tclVar.c:976-977`), zeroing the alt slot — **no global fallback**, raises "no such variable".

Documented at `tcl9.0.4/changes.md:189` ("Unqualified varnames resolved in current namespace, not global"). This is the *only* resolution-semantics (not feature) change in the whole 8.4→9.1 range. (The 8.6→9.0 `indexPtr`/`TCL_AVOID_RESOLVERS` cosmetic delta at `tcl8.6.16/generic/tclVar.c:918-925` vs `tcl9.0.4/generic/tclVar.c:935-938` produces no observable user-code outcome change.)

**VAR_LINK unification** (`upvar`/`global`/`variable`/`namespace upvar`): each installs a `Var` flagged `VAR_LINK` whose `value.linkPtr` points at the target (`TclPtrObjMakeUpvarIdx`, `tcl9.0.4/generic/tclVar.c:4737` `varPtr->value.linkPtr = otherPtr;`). Every lookup transparently follows the chain (`tcl8.6.16/generic/tclVar.c:757-759` `while (TclIsVarLink(varPtr)) varPtr = varPtr->value.linkPtr;`). The alias and target names are **one storage cell**. `VAR_LINK`/`TclIsVarLink` defined `tcl8.6.16/generic/tclInt.h:706, :803`. Mechanism identical 8.4→9.1; the 4th linker (`namespace upvar`, TIP 250) is 8.5+ only.

**Rust conformance:**

| Layer | Verdict | Evidence |
|---|---|---|
| Core bareword rule | FAITHFUL | `runtime/rust/src/vars.rs:26-38` (`classify`) + `current_home()` `:77-82`; VM `rust/tcl-vm/src/interp.rs:2497-2529` (`locate_from`) + `in_ns_script()` `:2472` mirrors `HasLocalVars`. |
| 9.0 no-global-fallback | **DIVERGENT (CONFIRMED)** — models 9.0 for *all* dialects | `runtime/rust/src/vars.rs` `classify()`/`current_home()` return `VarHome::Namespace(current_ns)` with no fallback, no version branch (module doc `:20-38` pinned to 9.0.3); VM `ns_var_fallback()` `interp.rs:2886-2914` + `locate()` `:2518-2525` never try `::name`; `tcl_version` hardcoded `"9.0"` at `interp.rs:666`. In an 8.4/8.5/8.6 file, `set ::x 1; namespace eval ::foo { set g $x }` reads `::x` in real tclsh but errors in Rust. |
| VAR_LINK in runtime/VM | FAITHFUL | runtime `runtime/rust/src/vars.rs:124` (`follow_links`); VM `rust/tcl-vm/src/interp.rs:2434` (`add_link`) / `:2449` (`add_global_link`) / `:2492,:2501` (`locate_from` follows `Local::Link`). |
| VAR_LINK in **analyser** | **DIVERGENT (CONFIRMED)** — no link modelled | `handle_global` `handlers.rs:197`, `handle_variable` `:218`, `handle_upvar` `:1495`, `handle_namespace_upvar` `:1520` all call `define_var` (`scope.rs:750-759`) registering the alias as a fresh standalone `VarDef`; `VarDef` (`types.rs:285-298`) has **no** link/target field. Rename/find-references never unify alias and target. |
| VAR_LINK in place/SSA layer | PARTIAL FAITHFUL (CONFIRMED) — separate model | `rust/tcl-compiler/src/var_resolve.rs:47-63` `ResolveContext { upvar_aliases, globals, ns_vars }`; `bind_scalar` `:111-135` resolves aliases to `place::upvar_alias(base,target)`; `place.rs:309-319` `overlap()` unifies alias places by owner. Sound for dataflow, but a *different resolver* from the analyser's `VarDef` map (the M4 4-way split). |

The analyser/place split is the core M4 defect: LSP editor features (analyser) and dataflow (place layer) disagree about whether alias and target are the same variable.

### 1.3 CLASS / METHOD resolution — TclOO (8.6+)

**MRO linearisation is DFS + late-placement, NOT C3.** `AddSimpleClassChainToCallContext` walks the class tree pre-order: (1) recurse class-level mixins first (`TRAVERSED_MIXIN`), (2) add the class's own method, (3) tail-recurse the superclass(es). Dedup in `AddMethodToCallChain` does **not** skip a re-encountered `Method*` — it **copies it down** to the latest position ("methods come as *late* in the call chain as possible"). Resolves diamonds/C3-inconsistent hierarchies deterministically where Python/Perl C3 would raise. (`tcl8.6.16/generic/tclOOCall.c:1468-1549`, late-placement `:838-858`; 9.0 identical `tcl9.0.4/generic/tclOOCall.c:1753-1836`, late-placement `:1016-1036`.)

**Two-pass BUILDING_MIXINS ordering:** `TclOOGetCallContext` adds the method twice at object level — pass 1 with `BUILDING_MIXINS` (mixin-reached classes only), pass 2 without (non-mixin only), gated by `MIXIN_CONSISTENT` (`tcl8.6.16/generic/tclOOCall.c:1145-1147`, gate `:810`). Guarantees mixins precede the main hierarchy [Bug 1998221].

**9.0 change:** `AddSimpleClassChainToCallContext` returns `privateDanger` and folds the private-method skip into the walk (`IS_PRIVATE`/`HAS_PRIVATE_METHODS`, TIP 500 true-private). Gates **visibility, not order**.

**Class-name resolution context:** superclass/mixin/self-mixin names resolve via `GetClassInOuterContext`, which walks `iPtr->varFramePtr` **up** past every `FRAME_IS_OO_DEFINE` frame to the frame that invoked `oo::define`/`oo::class create`, then `Tcl_GetObjectFromObj` (→ `Tcl_FindCommand`) in **that** namespace (`tcl8.6.16/generic/tclOODefineCmds.c:745-773`, call sites superclass `:2132`, mixin `:2002/:2472`; 9.0 `tcl9.0.4/generic/tclOODefineCmds.c:1069-1090`, also skips `FRAME_IS_PRIVATE_DEFINE`). Bare `superclass Base` resolves relative to the **define call-site** namespace, in exactly two scopes (current ns then global, plus `namespace path` in 8.5+) — **no** intermediate-ancestor walk, **no** unique-tail guess.

**`forward` target:** stores only a prefix list; `InvokeForwardMethod` rewrites argv, sets `iPtr->lookupNsPtr = contextPtr->oPtr->namespacePtr` (the object's own namespace) and re-resolves the forwarded word as an ordinary command every call (`tcl8.6.16/generic/tclOOMethod.c:1395-1427`, lookup ns `:1424-1425`). Not statically resolvable.

**Per-object mixins/methods** (`oo::objdefine ... mixin`/`method`) processed **before** the object's `selfCls` chain (`tcl8.6.16/generic/tclOOCall.c:747-757`); object methods live in `oPtr->methodsPtr`. A purely class-level MRO cannot see them.

**Rust conformance:**

| Aspect | Verdict | Evidence |
|---|---|---|
| MRO shape (DFS + late-placement) | FAITHFUL | `rust/tcl-syntax/src/mro.rs:127-167` (`tcloo_dfs`), `:243-296` (two-pass); tests assert real tclsh 9.0 `info class call` (`mro.rs:490, :511`). |
| Two-pass BUILDING_MIXINS | FAITHFUL | `rust/tcl-syntax/src/mro.rs:257-288`, gate `:152`; test `:478`. |
| Class-name context | **DIVERGENT (M1)** — resolves in class's OWN ns ancestry, not caller | `rust/tcl-compiler/src/analyser/class_hierarchy.rs:258-308` starts from `owner_qname`'s ns (`:268`), not the define call-site. Coincides with C only for `namespace eval ::Ns { oo::class create Sub {…} }`; wrong for `oo::class create ::Ns::Sub` from another ns. VM `cmd_oo.rs:199-211` uses `vm.current_ns()` = the outer context — **more faithful** than the analyser. |
| 2-scope limit | **DIVERGENT (M1)** — over-walks ancestors + unique-tail fallback | `class_hierarchy.rs:270-289` loops **every** ancestor ns; `:303-307` adds a unique-tail fallback (comments `:290-302` admit it can manufacture a wrong edge). VM `cmd_oo.rs:199-211` is the reference: absolute short-circuit, else `current_ns::name`, else bare — no ancestor walk, no tail guess. |
| `forward` target | PARTIAL (correct-by-deferral) | VM stores prefix (`rust/tcl-vm/src/cmd_oo.rs:65-67`); analyser records kind `"forward"` with `forward_target` (`rust/tcl-compiler/src/analyser/oo.rs:42-43`) but cannot resolve the callee — matching C's call-time deferral. Limits go-to-definition. |
| Per-object mixins/methods | NOT MODELLED | `mro.rs` / `class_hierarchy.rs` operate only on class→super/class-mixin edges. Acceptable for static analysis, but methods added via `oo::objdefine` are invisible. |

### 1.4 EXPR-FUNCTION resolution

**Authoritative algorithm (8.5+):** the expr compiler emits, for a `FUNCTION` lexeme `f(x)`, a **relative** (no leading `::`) command literal `tcl::mathfunc::f`, then compiles a normal invoke (`tcl8.6.16/generic/tclCompExpr.c:2270-2283`, `TclDStringAppendLiteral(&cmdName, "tcl::mathfunc::")` at `:2276`; since 8.5 `tcl8.5.19/generic/tclCompExpr.c:2202`). Because the literal is relative, `INST_INVOKE` re-resolves it through `Tcl_GetCommandFromObj`/`TclNREvalObjv` against the **current namespace at run time** (`tcl8.6.16/generic/tclExecute.c:3092`, `return TclNREvalObjv(...)`). So a proc `tcl::mathfunc::f` in the calling ns shadows the global, and any `proc ::tcl::mathfunc::f` (TIP 232) dispatches exactly like a C builtin. The builtins are real exported commands `::tcl::mathfunc::NAME` (`tcl8.6.16/generic/tclBasic.c:920-928`).

**8.4 is completely different (CONFIRMED):** no `::tcl::mathfunc` namespace at all (grep `mathfunc` over `tmp/tcl8.4.20/generic` = 0). The compiler emits `INST_CALL_BUILTIN_FUNC1` with an index into `tclBuiltinFuncTable` (`tcl8.4.20/generic/tclExecute.c:427`), and execution calls the C function pointer directly (`:3934` `case INST_CALL_BUILTIN_FUNC1:` → `:3947` `mathFuncPtr = &(tclBuiltinFuncTable[opnd]);` → `:3949` `(*mathFuncPtr->proc)(...)`). No command name, no namespace, no shadowing, no user override. The 8.4 table **lacks** `min, max, isinf, isnan, isfinite, isnormal, issubnormal, isunordered, isqrt, entier, bool`. 8.5 introduces the namespace scheme (`grep -c mathfunc tcl8.5.19/generic/tclExecute.c` = 6) and adds `min`/`max` as init.tcl procs (TIP 255, `tcl8.5.19/library/init.tcl:79, :95`). 8.6 keeps `INST_CALL_BUILTIN_FUNC1` only as `TCL_SUPPORT_84_BYTECODE` legacy that rewrites the index to `::tcl::mathfunc::NAME` (`tcl8.6.16/generic/tclExecute.c:3095-3111`).

**Rust conformance:**

| Aspect | Verdict | Evidence |
|---|---|---|
| Relative-literal codegen | FAITHFUL | `rust/tcl-compiler/src/codegen/expressions.rs:324-328` pushes `"tcl::mathfunc::{function}"` relative then `INVOKE_STK` — matches C by name. Contract `docs/design/contracts/command-resolution.md:125-133`. |
| Dialect gating of the function set | **DIVERGENT (CONFIRMED)** — dialect-blind | `mathfunc` is not modelled in the registry at all (`grep mathfunc rust/tcl-registry/src` = 0, whereas `mathop` **is** gated). The shared evaluator `rust/tcl-syntax/src/expr/mathfunc.rs:69` `dispatch(name, args)` takes **no** dialect parameter; its arms (`:71-79`) accept `min, max, isinf, isnan, isfinite, isnormal, issubnormal, isunordered, isqrt, entier, bool` — none of which exist in 8.4. Const-fold `rust/tcl-compiler/src/tcl_expr_eval.rs:328` applies no dialect gate on the name. Under a pinned 8.4 dialect, `expr {min(1,2)}` / `expr {isinf(1.0)}` are silently accepted and folded; real 8.4 errors "unknown math function". |
| User-proc `::tcl::mathfunc::f` ↔ `f(...)` link | **PARTIAL / DIVERGENT (CONFIRMED)** — static side does not link | Runtime backends resolve it correctly (`codegen/expressions.rs:325`); static analysis does not. `mathfunc.rs` is a closed eval table with no interaction with the unit proc table; the analyser has zero `mathfunc` references; the expr collector `rust/tcl-compiler/src/analyser/commands.rs:2225` (`collect_expr_substitutions`) only harvests `TokenType::Cmd` (`[...]`) tokens — an `ExprNode::Call` (`f(...)`) is never pushed into `command_invocations` (`usage.rs:1758` walks it only for string-eq/ne). So no go-to-def/refs, no arity/shadow check, and the proc is flagged unused. Admitted at `docs/design/contracts/command-resolution.md:184-188`. |

---

## 2. The 8.4 → 9.1 version matrix

`ALL_TCL` includes `TCL84` (bit 0, `rust/tcl-registry/src/dialects.rs:31`); `TCL85_PLUS` excludes it (`dialects.rs:66`); `TCL86_PLUS` excludes 8.4/8.5; `TCL90_PLUS` is 9.0+.

| Feature / rule | Introduced / changed | C evidence (changes-file + source) | Rust version-aware? | Gap + severity |
|---|---|---|---|---|
| **`namespace path` command tier** in `Tcl_FindCommand` | 8.5 (TIP 229) | `changes:6483`; absent `tcl8.4.20/tclNamesp.c:1961-2050`, present `tcl8.5.19/tclNamesp.c:2447`, `NamespacePathCmd:3913` | Registry gates the subcommand `TCL85_PLUS` (`namespace_.rs:262-268`) but **resolver ignores it** | **YES / medium (CONFIRMED)** — path honoured in 8.4 files; **FALSE-RESOLVES** bare calls via a path that cannot exist in 8.4 |
| **`namespace unknown`** subcommand | 8.5 (TIP 181) | `changes:6686`; absent 8.4, present 8.5+ | **Mis-gated** to `NON_IRULES_OPERATORS` (includes TCL84 via `ALL_TCL`) at `namespace_.rs:290-300` | **YES / low (CONFIRMED)** — accepted in 8.4 files; should be `TCL85_PLUS` like its siblings |
| **`namespace upvar`** (4th VAR_LINK linker) | 8.5 (TIP 250) | `changes:6684`; `NamespaceUpvarCmd tcl8.5.19/tclNamesp.c:203,2809,4450-4588`; absent 8.4 | **YES** — `TCL85_PLUS` (`namespace_.rs:303-313`) | none (gate holds; but the analyser alias-link gap of §1.2 applies) |
| **`namespace ensemble`** | 8.5 (TIP 112) | `changes:6003`; impl moved `tclNamesp.c`(8.5)→`tclEnsemble.c`(8.6) | **YES** — `TCL85_PLUS` (`namespace_.rs:160`) | none |
| **`apply`** | 8.5 (TIP 194) | `changes:6688`; `ApplyObjCmd` absent `tcl8.4.20/tclProc.c`, present 8.5+ | **YES** — `apply.rs:41 = TCL85_PLUS` | none |
| **`{*}` expansion** | 8.5 (TIP 157/293) | `changes:6046, :6853`; `TCL_TOKEN_EXPAND_WORD` absent `tcl8.4.20/tcl.h` | **YES** — `rust/tcl-lexer/src/lexer.rs:213-238` `expand_syntax:false` for 8.4/f5-irules | none |
| **`::tcl::mathfunc`** (functions as commands) | 8.5 (TIP 232); 8.4 = fixed C table | `tcl8.5.19/tclBasic.c:707`; 8.4 `tclExecute.c:427` fixed table, no namespace | **NO** — mathfunc unmodelled in registry; `mathfunc.rs` dialect-blind | **YES / medium (CONFIRMED)** — 8.5+/9.x functions folded under 8.4; see §3 |
| **`::tcl::mathop`** (operator commands) | 8.5 (TIP 174) | `changes:6875`; `tcl8.6.16/tclBasic.c:934-956`; absent 8.4 | **YES** — `mathop.rs:22-35 = TCL85_PLUS` | none (expr operators compile inline and never consult mathop — matches C) |
| **`${...}` braced var nesting** | changed 9.0 | `naming.rs:163` cites `tcl9.0.1/tclParse.c` braceCount loop; 8.x = first-close | **YES** — `lexer.rs:205-222` `FirstClose` (8.x) vs `Tcl9Nesting` (9.x) | none |
| **Unqualified var global fallback** | **removed 9.0** | `changes.md:189`; `tcl8.6.16/tclVar.c:918-925` (fallback) vs `tcl9.0.4/tclVar.c:936-937` (none) | **NO** — VM/runtime hardcode 9.0 for all dialects | **YES / medium (CONFIRMED)** — 8.4/8.5/8.6 files mis-resolve; wrong for 3 of 5 advertised dialects |
| **TclOO** (`oo::class/define/objdefine`) | 8.6 (TIP 257) | `changes:7223`; `tclOO*.c` only 8.6+ | **YES** — `oo_class.rs:65 / oo_define.rs:130 = TCL86_PLUS` | none |
| **`coroutine`** | 8.6 (TIP 328) | `changes:7373`; `CoroutineObjCmd` absent `tclBasic.c` pre-8.6 | **YES** — `coroutine.rs:37 = TCL86_PLUS` | none |
| **`tailcall`** | 8.6 (TIP 327) | `changes:7371`; absent pre-8.6 | **YES** — `tailcall_.rs:37 = TCL86_PLUS` | none |
| **`try`, `zipfs`** | 8.6 | 8.6 changelog | (out of resolution scope) | n/a |
| **`::oo::Helpers` (`next`/`nextto`/`self`)** | 8.6 | `tcl8.6.16/tclOO.c:343-344, :506-512, :699` (`TclSetNsPath` object ns → helpersNs) | **YES** — `oo_next.rs:28-30 / oo_self.rs:28-30 = TCL86_PLUS` | none (modelled as bare-name in-method specs, not literal path members — cosmetic) |
| **TclOO properties** (`property`, `oo::configurable`, configure/cget) | 9.0 (TIP 558) | `tcl9.0.4/tclOOProp.c` present; **absent** `tcl8.6.16/generic/`; present `tcl9.1b0/tclOOProp.c` | **PARTIAL** — `oo::configurable = TCL90_PLUS` (`oo_configurable.rs:38`) but the `property` **body member** carries no version/metaclass gate (`definer.rs:321`) | **YES / low (PLAUSIBLE)** — `property` accepted in 8.6 bodies and non-configurable 9.0 classes; **FALSE-RESOLVES** an 8.6 `oo::class create C { property x }` |
| **9.0 `::tcl::` reorg** (removed `unsupported::inject`; added `tcl::process`, `tcl::idna::*`; `isunordered`/`tm::path` fixes) | 9.0 | `changes.md:85, :103, :209, :261, :264` | **NO** — `tcl::tm`/`tcl::prefix`/`tcl::process`/`tcl::idna` not surfaced as specs | **YES / low (PLAUSIBLE)** — 9.0 ::tcl:: deltas unmodelled |
| **legacy `trace variable/vdelete/vinfo`** | removed 9.0 | 8.4-8.6 only | **YES** — `TCL8X` gate (`trace.rs:197,209,219`) | none |
| **Command resolution ORDER** (current→path→global) | stable except 8.5 path insert | `changes:6483` | version-agnostic by design (order never changed) | none |
| **Core-builtin existence** (unresolved-command W123) | per above | enforced `registry.rs:398 get_for_dialect` → `unresolved.rs:174` | **YES** | none — 8.6-only constructs draw unknown-command in an 8.4 file |

**Places the LSP would FALSE-RESOLVE an 8.6+-only construct inside an 8.4/8.5 file** (the ones that actually change resolution outcome, not just existence):

1. **`namespace path` in an 8.4 file** — the path is honoured, so a bare `helper` resolves to `::foo::helper` (goto-def/references succeed) where genuine 8.4 raises "invalid command name". (CONFIRMED, medium.)
2. **8.5+/9.x expr math functions in an 8.4 file** — `min()`, `isinf()` etc. are accepted and const-folded where 8.4 raises "unknown math function". (CONFIRMED, medium.)
3. **Unqualified global-only var read at namespace scope in 8.4/8.5/8.6** — Rust reports "no such variable" (9.0 semantics) where real 8.6 reads `::name`. (CONFIRMED, medium — this is a false *non*-resolution.)
4. **`property` body member in an 8.6 class** — accepted where 8.6 has no such definer keyword. (PLAUSIBLE, low.)

Note the existence checks (W123 via `get_for_dialect`) are correctly dialect-aware; the leaks above are all in code paths that *bypass* the dialect gate the registry already encodes.

---

## 3. The `::tcl::` / `::mathfunc` / `::mathop` / `::oo::Helpers` special namespaces

**`::tcl::mathfunc`** — expr `f(x)` compiles to the **relative** command `tcl::mathfunc::f`, resolved per-namespace at run time; a proc `::tcl::mathfunc::f` (TIP 232) is a first-class overridable function. Builtins are real exported commands (`tcl8.6.16/generic/tclBasic.c:920-928`). **Dialect gating:** the whole namespace is 8.5+; 8.4 uses `tclBuiltinFuncTable` C pointers (`tcl8.4.20/generic/tclExecute.c:427, 3934-3949`). **Rust:** codegen faithful (`rust/tcl-compiler/src/codegen/expressions.rs:324-328`); but the function set is dialect-blind and unmodelled in the registry (`rust/tcl-syntax/src/expr/mathfunc.rs:69`, `grep mathfunc rust/tcl-registry/src` = 0). **Two gaps** (both CONFIRMED, medium): (i) 8.5+/9.x functions accepted/folded under 8.4; (ii) the **expr-function↔user-proc linking gap** — a user `proc ::tcl::mathfunc::sq {x} {...}` is not linked to `sq()` call sites: no go-to-def/refs, no arity check, flagged unused. The runtime/VM resolve it correctly; only the static/LSP layer is unfaithful. Admitted at `docs/design/contracts/command-resolution.md:184-188`.

**`::tcl::mathop`** — namespace + per-operator commands (`::tcl::mathop::+`, `::eq`, `::%`, …) created via `Tcl_CreateNamespace` + a `Tcl_CreateObjCommand` loop and exported `*` (`tcl8.6.16/generic/tclBasic.c:934-956`); each has a compileProc so a direct call `[+ 1 2]` compiles natively (compileProcs in `tclCompCmdsSZ.c`). But expr's own operators are lexed/compiled inline in `tclCompExpr.c` to `INST_*` opcodes and **never** look up mathop — the mathop commands exist purely for command-position use (typically via `namespace path ::tcl::mathop`). **Dialect:** 8.5+ (TIP 174), absent 8.4. **Rust — FAITHFUL:** `rust/tcl-registry/src/commands/tcl/mathop.rs:22-35` (`TCL85_PLUS`); `command_binding.rs:481-498` special-cases mathop members so a proc named `+`/`eq` shadows nothing (correct — ops live in `::tcl::mathop`, not global). No gap.

**`::oo::Helpers`** — created once per interp; registers `::oo::Helpers::next`, `::nextto`, `::self` (`tcl8.6.16/generic/tclOO.c:343-344, :506-512`). Each object's private namespace gets a command **path** to `helpersNs` (`TclSetNsPath`, `tcl8.6.16/generic/tclOO.c:699`), so inside a method body bare `next`/`self` resolve first against the object ns then along the path to `::oo::Helpers`. Ordinary path-reachable commands, TclOO-scoped (8.6+). **Rust — FAITHFUL:** `rust/tcl-registry/src/commands/tcl/oo_next.rs:28-30`, `oo_self.rs:28-30` (`TCL86_PLUS`); `rust/tcl-compiler/src/analyser/oo.rs:81-84` handles in-method implicits. Minor: registered as flat bare-name specs rather than literal `::oo::Helpers` path members — relies on "in-method" context rather than reproducing the namespace-path mechanism; adequate, not structural.

**Other `::tcl::` sub-namespaces** — `::tcl::tm` (8.5), `::tcl::prefix` (8.5), `::tcl::clock`, `::tcl::unsupported::*`; 9.0 removed `unsupported::inject`, added `tcl::process` and `tcl::idna::*`, fixed `mathfunc::isunordered`/`tm::path` (`tcl9.0.4/changes.md:85, :103, :209, :261, :264`). **Rust — NOT MODELLED:** only `mathop` surfaces under `rust/tcl-registry/src/commands/tcl/`; these helper namespaces are not dialect-gated resolution specs. Low-priority 8.4→9.1 fidelity gap.

---

## 4. Conformance gaps vs C, mapped to milestones

Legend: **CONFIRMED** = verifier checked against both trees; **PLAUSIBLE** = investigation-only.

### 4.A DANGEROUS — false-resolution / wrong-link gaps

These produce silently-wrong LSP output (wrong goto-def/references/rename, false or missing diagnostics).

| # | Gap | Tag | Sev | C evidence | Rust evidence | Milestone |
|---|---|---|---|---|---|---|
| D1 | **`namespace path` not dialect-gated** — 8.4 file resolves bare calls through a path 8.4 cannot have | CONFIRMED | medium | `tcl8.4.20/tclNamesp.c:1961` (no path tier) vs `tcl8.5.19/tclNamesp.c:2447, 2469`; `NamespacePathCmd:3913` | ungated: `commands.rs:511-513` (`DialectSet::empty()`), `registry.rs:1088-1090`, `spec.rs:801-803` (name-only), `handlers.rs:963-964`, `scope.rs:340,393-394`; VM `interp.rs:1675-1682`. Registry gate exists but unused: `namespace_.rs:262-268` | **NEW** (dialect-aware resolver) |
| D2 | **Analyser VAR_LINK not modelled** — upvar/global/variable/namespace-upvar alias not unified with target; rename misses target, find-refs misses alias sites | CONFIRMED | high | `tcl9.0.4/tclVar.c:4737`; follow `tcl8.6.16/tclVar.c:757-759`; `tclInt.h:706,:803` | `handlers.rs:197,218,1495,1520` all `define_var`; `scope.rs:750-759`; `VarDef` `types.rs:285-298` has no link field. (Runtime/VM faithful: `vars.rs:124`, `interp.rs:2434,2449,2492,2501`) | **M4** |
| D3 | **Analyser vs place-layer alias split** — two resolvers disagree on variable identity | CONFIRMED | medium | `tcl9.0.4/tclVar.c:4736-4737, 776-777` | analyser `types.rs:285-298` / `handlers.rs:1483-1523` (no link) vs `var_resolve.rs:47-63,111-136` + `place.rs:309-319` (unifies) | **M4** |
| D4 | **9.0 unqualified var global-fallback removal not version-gated** — 8.4/8.5/8.6 files mis-resolve global-only vars at namespace scope | CONFIRMED | medium | `tcl8.6.16/tclVar.c:918-925` (fallback) vs `tcl9.0.4/tclVar.c:936-937`, `tcl9.1b0/tclVar.c:976-977`; `ObjFindNamespaceVar tclVar.c:5734`; `tclNamesp.c:2241-2243`; `changes.md:189` | `runtime/rust/src/vars.rs` `classify`/`current_home` (no branch); VM `interp.rs:2886-2914, 2518-2525`, `tcl_version` hardcoded `:666` | **NEW** (cross-version var semantics; adjacent to M4) |
| D5 | **Analyser class-name context** — resolves in class's own ns ancestry, not the define call-site (wrong/no base cross-namespace) | PLAUSIBLE | medium | `tcl8.6.16/tclOODefineCmds.c:745-773`; 9.0 `tcl9.0.4/tclOODefineCmds.c:1069-1090` | `class_hierarchy.rs:258-308` (start `:268`). VM more faithful: `cmd_oo.rs:199-211` | **M1** |
| D6 | **Analyser over-walks ancestor namespaces + unique-tail fallback** — can link to an unrelated class C leaves unresolved | PLAUSIBLE | low | `tcl9.0.4/tclNamesp.c` `Tcl_FindCommand` (current+global+path only); `tclOODefineCmds.c:761` | `class_hierarchy.rs:270-289, 303-307` (comments admit `:290-302`). Reference: VM `cmd_oo.rs:199-211` | **M1 (extend)** |
| D7 | **expr math-function set dialect-blind** — 8.5+/9.x functions accepted/folded under 8.4 | CONFIRMED | medium | `tcl8.4.20/tclExecute.c:427, 3934-3949` (no min/max/is*); 8.5 `init.tcl:79,95`; 8.6 legacy `tclExecute.c:3095-3111` | `mathfunc.rs:69-82` (no dialect param); `tcl_expr_eval.rs:299-332`; VM `expr.rs:686-698`; unmodelled in registry (mathop is gated) | **NEW** (expr-function dialect gating) |
| D8 | **expr-function ↔ user-proc link missing** — `proc ::tcl::mathfunc::f` invisible to `f(...)`; no def/refs/arity, flagged unused | CONFIRMED | medium | `tcl8.6.16/tclCompExpr.c:2276`; `tclExecute.c:3092` | codegen faithful `expressions.rs:325`; static: `mathfunc.rs` closed table; `commands.rs:2225` only `TokenType::Cmd`; `usage.rs:1758`. Admitted `command-resolution.md:184-188` | **NEW** |
| D9 | **`namespace unknown` mis-gated to include 8.4** — accepted in 8.4 files | CONFIRMED | low | absent `tcl8.4.20/tclNamesp.c`; `changes:6686` (TIP 181) | `namespace_.rs:290-300` (`NON_IRULES_OPERATORS` ⊇ TCL84 via `ALL_TCL`, `dialects.rs:107-111,31`); should be `TCL85_PLUS` like `:262-268` | **NEW** |
| D10 | **`property` body member not version/metaclass-gated** — accepted in 8.6 bodies & non-configurable 9.0 classes | PLAUSIBLE | low | `tcl9.0.4/tclOO.c:727-744` (registered only in configurable config-ns, 9.0+); absent 8.6 | `definer.rs:321` (`flag_keyed("property")`, no guard). Commands correctly gated: `oo_configurable.rs:38 = TCL90_PLUS` | **NEW** |
| D11 | **Trace command/execution TARGET name not a reference** — `trace add command foo …` / `trace add execution foo …`; rename/find-refs on `foo` misses the trace site (variable-trace targets ARE handled) | CONFIRMED | medium | `tcl8.6.16/tclTrace.c:487-488` → `Tcl_FindCommand:1119`; `:507`; 9.0 `tclTrace.c:388,454,625,661` | `trace.rs:62-71` (`VarWrite` only for `variable`; empty for command/execution); callback modelled as `CommandPrefix` `:165`; `traits.rs:212-216` slot-only | **NEW** |

### 4.B Merely-missed — not a wrong link (feature-completeness / behavioural / cosmetic)

| # | Gap | Tag | Sev | Evidence | Milestone |
|---|---|---|---|---|---|
| N1 | **`forward` callee not statically resolvable** (correct-by-deferral; limits goto-def) | PLAUSIBLE | low | C `tcl8.6.16/tclOOMethod.c:1395-1427`; Rust `cmd_oo.rs:65-67`, `oo.rs:42-43` | **NEW** |
| N2 | **Per-object mixins/methods not modelled** (`oo::objdefine`) | PLAUSIBLE | low | C `tcl8.6.16/tclOOCall.c:747-757`; Rust `mro.rs`/`class_hierarchy.rs` class-edges only | **NEW** |
| N3 | **Property surface (configure/cget, `<ReadProp-*>`) omitted from `known_methods`** — real but **benign** (W308 firing independently allow-lists configure/cget) | PARTIAL | low | C `tcl9.0.4/tclOOProp.c:487-497,514-524,103,135`; Rust `class_hierarchy.rs:174-175`, `var_command.rs:261-264` (allow-list gates FP), `:348,:355` | **NEW** (code-hygiene) |
| N4 | **9.0 `::tcl::` reorg deltas unmodelled** (`unsupported::inject`, `tcl::process`, `tcl::idna`, `isunordered`/`tm::path`) | PLAUSIBLE | low | C `tcl9.0.4/changes.md:85,103,209,261,264`; Rust: only mathop surfaces | **NEW** |
| N5 | **Command-name caching / epoch not modelled** (VM re-resolves each dispatch; perf only) | PLAUSIBLE | low | C `tcl9.0.4/tclObj.c:4340-4360`; `tclBasic.c:1858,2802,3161,3295`; Rust `interp.rs:1658-1684` | **NEW (perf)** |
| N6 | **Alias loop prevention (`TclPreventAliasLoop`) not in VM** — self-alias recurses instead of clean error | PLAUSIBLE | low | C `tcl8.6.16/tclInterp.c:1387-1464,1531`; Rust `command.rs:737-745` | **NEW** |
| N7 | **Cross-interp aliases (child→parent) unsupported in VM** (analyser correctly refuses the link) | PLAUSIBLE | low | C `tcl8.6.16/tclInterp.c:1841,1884-1911`; Rust VM `command.rs:738-741`, analyser `signature_scan/handlers.rs:321,878-891` | **NEW** |
| N8 | **VM does not fire command/execution traces** (accepted no-op; resolution unaffected) | PLAUSIBLE | low | C `tcl8.6.16/tclTrace.c:1105-1151,1282`; Rust `cmd_trace.rs:27,94-95` | **NEW** |

### 4.C Confirmed-faithful (no gap) — reference points

- Command resolution **order** (current→path→global): FAITHFUL — `naming.rs:455-492,516-524`.
- Core bareword local-vs-namespace rule: FAITHFUL — `runtime/rust/src/vars.rs:26-38`; VM `interp.rs:2497-2529,2472`.
- MRO shape (DFS + late-placement) and two-pass mixin ordering: FAITHFUL — `mro.rs:127-167,243-296,257-288`.
- `::tcl::mathop` gating + operator inline-compile: FAITHFUL — `mathop.rs:22-35`, `command_binding.rs:481-498`.
- `{*}`, `${...}` nesting, core-builtin existence, `namespace upvar/ensemble/apply/coroutine/tailcall` version gates, `::oo::Helpers` gating, legacy-trace `TCL8X` gating: all FAITHFUL (see §2 table).
- **Reference implementations for the analyser M1 fix:** VM `cmd_oo.rs:199-211` (2-scope class resolve), and for command resolution the shared `naming.rs` resolver.
- **Alias resolution (late-bound, global-ns anchored):** FAITHFUL — C `tcl8.6.16/tclInterp.c:1832,1892,1436-1439`; Rust `exec.rs:2701-2735`. Hidden-command table + parent/child interp isolation: FAITHFUL — `interp.rs:379-381,369-372`, `signature_scan/walker.rs:35-38,128-146`.

---

## 5. Consolidated NEW-work table

Existing milestones **M1** (class-name resolution) and **M4** (variable 4-way split) already cover D2, D3, D5, D6. Everything below is **not** covered by M0–M6 and needs a slot.

| ID | Proposed slot | Title | Sev | Primary fix locations (Rust) | Anchoring C truth |
|---|---|---|---|---|---|
| **M7** | *Dialect-aware resolver* | Gate the `namespace path` command tier by dialect (D1) | medium | gate at `handlers.rs:955-964` (check `self.dialect` ⊇ `TCL85_PLUS` before inserting `namespace_paths`); thread real dialect into `commands.rs:511`; honour `sub.dialects` in `spec.rs:801-803` + `registry.rs:1088-1128`; mirror VM `interp.rs:1675-1682`. Registry gate `namespace_.rs:262-268` already correct | `tcl8.4.20/tclNamesp.c:1961` vs `tcl8.5.19/tclNamesp.c:2447` |
| **M7** | *Dialect-aware resolver* | Re-gate `namespace unknown` to `TCL85_PLUS` (D9) | low | `namespace_.rs:290-300` | `changes:6686` |
| **M8** | *Cross-version var semantics* | Dialect-gate the 9.0 unqualified-var global-fallback removal (D4) | medium | `runtime/rust/src/vars.rs` `classify`/`current_home`; VM `interp.rs` `ns_var_fallback`/`locate` + `:666`; analyser var path (with M4) | `tcl8.6.16/tclVar.c:918-925` vs `tcl9.0.4/tclVar.c:936-937`; `changes.md:189` |
| **M9** | *Expr-function fidelity* | Dialect-aware mathfunc allowlist (8.4 = `tclBuiltinFuncTable` set only) (D7) | medium | `mathfunc.rs:69-82`; `tcl_expr_eval.rs:299-332`; VM `expr.rs:686-698`; add gated registry model | `tcl8.4.20/tclExecute.c:427` |
| **M9** | *Expr-function fidelity* | Route `f(...)` through command resolution to link `::tcl::mathfunc::f` procs (D8) | medium | emit a `::tcl::mathfunc::<f>` collected head in `commands.rs` (`collect_expr_substitutions`); `usage.rs:1758` | `tcl8.6.16/tclCompExpr.c:2276`, `tclExecute.c:3092` |
| **M10** | *TclOO version fidelity* | Version/metaclass-gate the `property` body member (D10) | low | `definer.rs:321` (add `TCL90_PLUS` + configurable-family guard) | `tcl9.0.4/tclOO.c:727-744` |
| **M10** | *TclOO version fidelity* | Fold property accessors / configure-cget into `known_methods` (N3, hygiene) | low | `class_hierarchy.rs:169-180` | `tcl9.0.4/tclOOProp.c:487-524` |
| **M11** | *Trace-target references* | Assign a command-reference role to `trace add command/execution NAME` (D11) | medium | `trace.rs:62-71,76-85` (emit command-ref role); consume in `commands.rs` (~`:345,1131`) | `tcl8.6.16/tclTrace.c:487-488,1119` |
| **M12** | *Coverage / lower-priority* | Model 9.0 `::tcl::` reorg as dialect-gated specs (N4) | low | new specs under `rust/tcl-registry/src/commands/tcl/` | `tcl9.0.4/changes.md:209,261,264` |
| **M12** | *Coverage* | Forward callee / per-object mixins static handling (N1, N2) | low | `analyser/oo.rs`, `mro.rs` | `tclOOMethod.c:1395-1427`, `tclOOCall.c:747-757` |
| **M13** | *VM behavioural parity (out of resolution scope)* | Alias loop prevention (N6), cross-interp aliases (N7), fire command/execution traces (N8), command-name epoch cache (N5) | low | `command.rs:737-745`, `command.rs:738-741`, `cmd_trace.rs:94-95`, `interp.rs:1658-1684` | `tclInterp.c:1387-1464`, `tclTrace.c:1105-1151`, `tclObj.c:4340-4360` |

**Bottom line.** The Rust resolver reproduces the *shape* of the C algorithm faithfully for all four kinds (command, variable, class/MRO, expr-function) — the order, the two-slot/path search, DFS-late-placement MRO, and the relative-`tcl::mathfunc::` scheme are all correct. The LSP's "8.4–9.1" claim holds for **existence** gating (W123 via `get_for_dialect`) but breaks in the handful of places where resolution *outcome* is version-dependent and the code bypasses the dialect gate the registry already encodes: the `namespace path` tier (D1), the 9.0 variable-fallback removal (D4), and the expr math-function set (D7). The most damaging correctness gap is analyser-only and version-independent: `VAR_LINK` alias↔target is not unified (D2, high), so rename/find-references are silently incomplete for every `upvar`/`global`/`variable`/`namespace upvar`.