# Name resolution — the C algorithm and the 8.4 → 9.1 matrix

**Status:** current reference. This document extracts the *real*
name-resolution algorithm from the C Tcl sources for all four name kinds —
command, variable, class/method, and expr function — and states what actually
changed for resolution across 8.4 → 8.5 → 8.6 → 9.0 → 9.1. It is the
ground-truth companion to [name-resolution.md](name-resolution.md) (what we
build on top) and [contracts/command-resolution.md](contracts/command-resolution.md)
(the rule and its conformance gates).

Every claim carries a `file:line` citation into the C tree. Read this
document when you need to know *what C does*; read the other two when you
need to know what we do about it.

## Source-link legend

Every `file:line` below resolves to a stable GitHub permalink by prefixing
the path with the matching base URL and appending `#L<line>`. The C trees are
pinned to the exact release tags studied:

| Tree prefix | Tag | Base URL |
|---|---|---|
| `tcl8.4.20/` | `core-8-4-20` | `https://github.com/tcltk/tcl/blob/9ccfe9d1b35741ff7323837f6485ffe48b06fad9/<path>#L<line>` |
| `tcl8.5.19/` | `core-8-5-19` | `https://github.com/tcltk/tcl/blob/160d612a6b2b1c2c0db27236d648b7bc1364570c/<path>#L<line>` |
| `tcl8.6.16/` | `core-8-6-16` | `https://github.com/tcltk/tcl/blob/874e4fe4264a40c00c4db5115afba9600f9f368d/<path>#L<line>` |
| `tcl9.0.4/`  | `core-9-0-4`  | `https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/<path>#L<line>` |
| `tcl9.1b0/`  | `core-9-1-b0` | `https://github.com/tcltk/tcl/blob/fbe83207a70634a5031c70bdce3d59071920f6da/<path>#L<line>` |

Strip the tree prefix to get the in-repo path: `tcl9.0.4/generic/tclVar.c:4737`
→ `https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclVar.c#L4737`.
The `make fetch-tcl-source` trees under `tmp/` are always the primary ground
truth; the permalinks exist so a reader without them can follow along.

---

## 1. Command resolution — `Tcl_FindCommand`

Authoritative algorithm (`tcl8.6.16/generic/tclNamesp.c:2528-2687`; the
qualified-name split `TclGetNamespaceForQualName` at `:2272`):

1. Choose the context namespace `cxtNsPtr`: global if `TCL_GLOBAL_ONLY` or
   the name starts with `::`, else the passed `contextNsPtr`, else the
   interp's current namespace.
2. Run any per-namespace / per-interp `cmdResProc` resolver hooks first;
   return immediately on `TCL_OK` or error.
3. **Path tier (8.5+ only):** if `cxtNsPtr->commandPathLength != 0` AND the
   name is not `::`-rooted AND `TCL_NAMESPACE_ONLY` is not set — search the
   current namespace first, then iterate `commandPathArray[i]` **in order**
   (`tclNamesp.c:2627-2642`), then global as a last resort (`:2649-2659`).
4. **Otherwise** (no path, rooted, or namespace-only): the classic two-slot
   search — `nsPtr[0]` = context-derived, `nsPtr[1]` = global, first hit wins
   (`tclNamesp.c:2661-2682`).

`TclGetNamespaceForQualName` splits `a::b::c` into `(realNs, simpleName)` and
is what makes a `::`-rooted name bypass the context search. 9.0 is identical
apart from cosmetics (`tcl9.0.4/generic/tclNamesp.c:2640-2808`: `NS_DYING` →
`NS_DEAD`, `int` → `Tcl_Size`).

**Version delta.** 8.4 has **no path tier** — `Tcl_FindCommand` goes straight
to the two-slot search (`tcl8.4.20/generic/tclNamesp.c:1961-2050`); grepping
that file for `commandPathLength` / `commandPathArray` / `NamespacePathCmd`
returns nothing. The tier arrives in 8.5 via TIP 229
(`tcl8.5.19/generic/tclNamesp.c:2447`, `NamespacePathCmd` at `:197`;
`changes:6483`). The resolution **order** never changed 8.5 → 9.1; only the
8.4 → 8.5 boundary inserted the middle tier.

**Caching.** C caches a `ResolvedCmdName` on the name object (the `cmdName`
`Tcl_ObjType`), reused only while `cmdPtr->cmdEpoch == resPtr->cmdEpoch` and
the namespace is not dying (`tcl9.0.4/generic/tclObj.c:4340-4360`,
`SetCmdNameObj` `:4401-4420`); the epoch bumps on hide, rename, and delete
(`tcl8.6.16/generic/tclBasic.c:1858, 2802, 3161, 3295`). This is a
performance mechanism, not a semantic one — it never changes resolution
order.

## 2. Variable resolution — `TclLookupSimpleVar`

Authoritative bareword rule (`tcl8.6.16/generic/tclVar.c:907-1011`, rule at
`:907-913`; 9.0 `tcl9.0.4/generic/tclVar.c:924-1019`; 8.4
`tcl8.4.20/generic/tclVar.c:753-844`):

A bareword is looked up as a **namespace/global** variable iff (a)
`TCL_GLOBAL_ONLY` or `TCL_NAMESPACE_ONLY` is set, OR (b) there is no active
proc frame — global scope, or a `namespace eval` / `inscope` body (8.4:
`varFramePtr==NULL || !varFramePtr->isProcCallFrame`; 8.6/9.0:
`!HasLocalVars(varFramePtr)`), OR (c) the name contains `::`. Otherwise it is
a **frame-local**: search the compiled-locals array by exact string, then the
frame's runtime variable hashtable. `lookGlobal` (an absolute `::name`, or a
global-namespace context) forces a `::` home; otherwise the namespace home is
the current frame's `nsPtr`.

This rule is **semantically byte-identical 8.4 → 9.1**; only the spelling of
the "in a proc?" test changed (`isProcCallFrame` → `HasLocalVars`). No TIP
altered it — the frequently-cited "TIP 278" is a misattribution, and grepping
`278` across the changes files and the `tclVar.c` / `tclNamesp.c` comments
finds nothing relevant.

**The one genuine cross-version semantic change: 9.0 removed the global
fallback.** For an *unqualified, undefined* name at namespace or global scope
that is not `lookGlobal`:

- **8.4/8.5/8.6** — the `else` branch leaves `TCL_NAMESPACE_ONLY` unset (it
  sets it only under `TCL_AVOID_RESOLVERS`), so `ObjFindNamespaceVar` runs
  its two-slot search `nsPtr[0]`=current, `nsPtr[1]`=global
  (`tcl8.6.16/generic/tclVar.c:5641`). Current namespace, **then global**.
  `TclGetNamespaceForQualName` sets the alternate slot unless namespace-only
  (`tcl8.6.16/generic/tclNamesp.c:2241-2243`).
- **9.0/9.1** — the `else` branch **unconditionally** forces `flags |=
  TCL_NAMESPACE_ONLY` and `*indexPtr = -2`
  (`tcl9.0.4/generic/tclVar.c:935-938`, identical at
  `tcl9.1b0/generic/tclVar.c:976-977`), zeroing the alternate slot. No global
  fallback; it raises "no such variable".

Documented at `tcl9.0.4/changes.md:189` ("Unqualified varnames resolved in
current namespace, not global"). This is the **only** resolution-semantics
(as opposed to feature) change in the whole 8.4 → 9.1 range. The 8.6 → 9.0
`indexPtr` / `TCL_AVOID_RESOLVERS` delta at
`tcl8.6.16/generic/tclVar.c:918-925` is cosmetic and produces no observable
user-code change.

**`VAR_LINK` unification.** `upvar`, `global`, `variable`, and `namespace
upvar` each install a `Var` flagged `VAR_LINK` whose `value.linkPtr` points
at the target (`TclPtrObjMakeUpvarIdx`, `tcl9.0.4/generic/tclVar.c:4737`), and
every lookup transparently follows the chain
(`tcl8.6.16/generic/tclVar.c:757-759`). **The alias and the target are one
storage cell.** `VAR_LINK` / `TclIsVarLink` are defined at
`tcl8.6.16/generic/tclInt.h:706, :803`. The mechanism is identical 8.4 →
9.1; only the fourth linker, `namespace upvar` (TIP 250), is 8.5+.

## 3. Class and method resolution — TclOO (8.6+)

**MRO linearisation is DFS with late placement, not C3.**
`AddSimpleClassChainToCallContext` walks the class tree pre-order: recurse
class-level mixins first (`TRAVERSED_MIXIN`), add the class's own method,
then tail-recurse the superclasses. The dedup in `AddMethodToCallChain` does
**not** skip a re-encountered `Method*` — it *copies it down* to the latest
position, because methods must come as **late** in the call chain as
possible. This resolves diamonds and C3-inconsistent hierarchies
deterministically, where Python's or Perl's C3 would raise.
(`tcl8.6.16/generic/tclOOCall.c:1468-1549`, late placement `:838-858`; 9.0
identical at `tcl9.0.4/generic/tclOOCall.c:1753-1836`, `:1016-1036`.)

**Two-pass `BUILDING_MIXINS` ordering.** `TclOOGetCallContext` adds the
method twice at object level — pass 1 with `BUILDING_MIXINS` (mixin-reached
classes only), pass 2 without (non-mixin only), gated by `MIXIN_CONSISTENT`
(`tcl8.6.16/generic/tclOOCall.c:1145-1147`, gate `:810`). This guarantees
mixins precede the main hierarchy [Bug 1998221].

**9.0 change:** `AddSimpleClassChainToCallContext` returns `privateDanger`
and folds the private-method skip into the walk (`IS_PRIVATE` /
`HAS_PRIVATE_METHODS`, TIP 500 true-private). It gates **visibility, not
order**.

**Class-name resolution context.** Superclass, mixin, and self-mixin names
resolve via `GetClassInOuterContext`, which walks `iPtr->varFramePtr` **up**
past every `FRAME_IS_OO_DEFINE` frame to the frame that invoked `oo::define`
/ `oo::class create`, then calls `Tcl_GetObjectFromObj` (→
`Tcl_FindCommand`) in **that** namespace
(`tcl8.6.16/generic/tclOODefineCmds.c:745-773`; call sites: superclass
`:2132`, mixin `:2002/:2472`; 9.0
`tcl9.0.4/generic/tclOODefineCmds.c:1069-1090`, which also skips
`FRAME_IS_PRIVATE_DEFINE`). So a bare `superclass Base` resolves relative to
the **define call-site** namespace, in exactly two scopes — current then
global, plus `namespace path` in 8.5+. There is **no** intermediate-ancestor
walk and **no** unique-tail guess: a bare `superclass Base` written inside
`::a::b::Sub` where `Base` exists only at `::a::Base` errors at
class-definition time.

**Method export visibility.** The `PUBLIC_PATTERN` rule (`tclOODefineCmds.c`)
is purely lexical: a member is exported iff its name's first character is an
ASCII lowercase letter. Explicit `export` / `unexport` layer over it, and a
re-`method` resets to the default.

**`forward` targets.** A forward stores only a prefix list.
`InvokeForwardMethod` rewrites argv, sets `iPtr->lookupNsPtr =
contextPtr->oPtr->namespacePtr` (the *object's* own namespace), and
re-resolves the forwarded word as an ordinary command on **every call**
(`tcl8.6.16/generic/tclOOMethod.c:1395-1427`, lookup namespace
`:1424-1425`). It is not statically resolvable, by construction.

**Per-object mixins and methods.** `oo::objdefine`'s methods and mixins are
processed **before** the object's `selfCls` chain
(`tcl8.6.16/generic/tclOOCall.c:747-757`); object methods live in
`oPtr->methodsPtr`. A purely class-level MRO cannot see them.

**Method bodies run in the object's namespace.** A method body executes with
`::oo::ObjN` current, whose `namespace path` is `::oo::Helpers` — the home of
`next` / `self`, with `my` an object-namespace command
(`tcl8.6.16/generic/tclOO.c:343-344, :506-512, :699`). Bare *and*
relative-qualified names resolve object-ns → Helpers → **global**; the
class's defining namespace is never searched.

## 4. Expr-function resolution

**8.5+ algorithm.** The expr compiler emits, for a `FUNCTION` lexeme `f(x)`,
a **relative** (no leading `::`) command literal `tcl::mathfunc::f`, then
compiles a normal invoke (`tcl8.6.16/generic/tclCompExpr.c:2270-2283`, the
`TclDStringAppendLiteral(&cmdName, "tcl::mathfunc::")` at `:2276`; since 8.5
at `tcl8.5.19/generic/tclCompExpr.c:2202`). Because the literal is relative,
`INST_INVOKE` re-resolves it against the **current namespace at run time**
(`tcl8.6.16/generic/tclExecute.c:3092`). A proc `tcl::mathfunc::f` in the
calling namespace therefore shadows the global one, and any `proc
::tcl::mathfunc::f` (TIP 232) dispatches exactly like a C builtin. The
builtins themselves are real exported commands `::tcl::mathfunc::NAME`
(`tcl8.6.16/generic/tclBasic.c:920-928`).

**8.4 is a completely different mechanism.** There is no `::tcl::mathfunc`
namespace at all (grepping `mathfunc` over `tmp/tcl8.4.20/generic` returns
nothing). The compiler emits `INST_CALL_BUILTIN_FUNC1` with an index into
`tclBuiltinFuncTable` (`tcl8.4.20/generic/tclExecute.c:427`) and execution
calls the C function pointer directly (`:3934` → `:3947` → `:3949`). No
command name, no namespace, no shadowing, no user override. The 8.4 table
**lacks** `min`, `max`, `isinf`, `isnan`, `isfinite`, `isnormal`,
`issubnormal`, `isunordered`, `isqrt`, `entier`, and `bool`. 8.5 introduces
the namespace scheme (TIP 232) and adds `min` / `max` as `init.tcl` procs
(TIP 255, `tcl8.5.19/library/init.tcl:79, :95`). 8.6 keeps
`INST_CALL_BUILTIN_FUNC1` only as `TCL_SUPPORT_84_BYTECODE` legacy, which
rewrites the index to `::tcl::mathfunc::NAME`
(`tcl8.6.16/generic/tclExecute.c:3095-3111`).

---

## 5. The 8.4 → 9.1 matrix

`ALL_TCL` includes `TCL84`; `TCL85_PLUS` excludes it; `TCL86_PLUS` excludes
8.4 and 8.5; `TCL90_PLUS` is 9.0+ (`rust/tcl-registry/src/dialects.rs`).

| Feature / rule | Introduced or changed | C evidence | How it is modelled |
|---|---|---|---|
| **`namespace path` command tier** in `Tcl_FindCommand` | 8.5 (TIP 229) | `changes:6483`; absent `tcl8.4.20/tclNamesp.c:1961-2050`, present `tcl8.5.19/tclNamesp.c:2447` | Analyser gates at the *recording* site (a pre-8.5 dialect records no path entry, so every consumer skips the tier); the VM gates at *resolution* time, since its version knob is mutable |
| **Unqualified variable global fallback** | **removed 9.0** (TIP-less; `changes.md:189`) | `tcl8.6.16/tclVar.c:918-925` (fallback) vs `tcl9.0.4/tclVar.c:936-937`, `tcl9.1b0/tclVar.c:976-977` | One registry knob, `DialectSet::namespace_var_global_fallback`, derived from the dialect's runtime base version; honoured by the analyser, the VM (`RuntimeVersion`), and the WASM runtime. Vectored through the VM at both versions *and* under real `tclsh8.6` / `tclsh9.0` |
| **`::tcl::mathfunc`** (functions as commands) | 8.5 (TIP 232); 8.4 = fixed C table | `tcl8.5.19/tclBasic.c:707`; 8.4 `tclExecute.c:427` | `tcl_syntax::expr::mathfunc::added_in` is the single source of truth for the name set and its per-release ceiling; the const-folder and W002 both read `math_func_ceiling_for_dialect`; W123 additionally gates the *command-wrapper* form at 8.5+ |
| **`namespace unknown`** | 8.5 (TIP 181) | `changes:6686`; absent 8.4 | `TCL85_PLUS`, matching its sibling `namespace path` |
| **`namespace upvar`** (4th `VAR_LINK` linker) | 8.5 (TIP 250) | `changes:6684`; `NamespaceUpvarCmd tcl8.5.19/tclNamesp.c:203, 2809` | `TCL85_PLUS` |
| **`namespace ensemble`** | 8.5 (TIP 112) | `changes:6003`; moved `tclNamesp.c` (8.5) → `tclEnsemble.c` (8.6) | `TCL85_PLUS` |
| **`apply`** | 8.5 (TIP 194) | `changes:6688`; `ApplyObjCmd` absent in 8.4 | `TCL85_PLUS` |
| **`{*}` expansion** | 8.5 (TIP 157/293) | `changes:6046, :6853`; `TCL_TOKEN_EXPAND_WORD` absent `tcl8.4.20/tcl.h` | Lexer flag `expand_syntax:false` for 8.4 and `f5-irules` |
| **`::tcl::mathop`** (operator commands) | 8.5 (TIP 174) | `changes:6875`; `tcl8.6.16/tclBasic.c:934-956` | `TCL85_PLUS` per spec, **and** on the profile axis (`operators_as_commands`), which is a separate gate that must agree |
| **`::tcl::tm`** (Tcl Modules) | 8.5 (TIP 189) | — | `tcl::tm::path` / `tcl::tm::roots` = `TCL85_PLUS` |
| **TclOO** (`oo::class` / `define` / `objdefine`) | 8.6 (TIP 257) | `changes:7223`; `tclOO*.c` 8.6+ only | `TCL86_PLUS` |
| **`coroutine`**, **`tailcall`** | 8.6 (TIP 328 / 327) | `changes:7373, :7371` | `TCL86_PLUS`; `::tcl::unsupported::corotype` likewise `TCL86_PLUS` (empirically confirmed on a real `tclsh 8.6.14` — it ships with coroutines, not with 9.0) |
| **`::oo::Helpers`** (`next` / `nextto` / `self`) | 8.6 | `tcl8.6.16/tclOO.c:343-344, :506-512, :699` | `TCL86_PLUS`, modelled as in-method bare-name specs rather than literal path members |
| **`${...}` braced variable nesting** | changed 9.0 | `tcl9.0.1/tclParse.c` brace-count loop; 8.x = first-close | Lexer: `FirstClose` (8.x) vs `Tcl9Nesting` (9.x) |
| **TclOO properties** (`property`, `oo::configurable`, `configure`/`cget`) | 9.0 (TIP 558) | `tcl9.0.4/tclOOProp.c` present; **absent** `tcl8.6.16/generic/` | `oo::configurable` and the `property` **body member** both `TCL90_PLUS` (`MemberSpec::dialects`); when the definer itself is disabled the member gate is bypassed so the body still resolves structurally and draws one diagnostic, not a cascade. A configurable class answers `configure` / `cget` for its properties in `known_methods` |
| **`zipfs`** | 9.0 (TIP 430) | `zipfs.n` | `TCL90_PLUS`, both the public `zipfs` and `::tcl::zipfs` ensemble spellings, with the full 9.0 subcommand set |
| **9.0 `::tcl::` reorg** (`tcl::process`, `tcl::idna::*`, `tcl::build-info`; removed `unsupported::inject`) | 9.0 | `tcl9.0.4/changes.md:85, :103, :209, :261, :264` | Gated specs for `tcl::process` / `tcl::idna` / `tcl::zipfs` / `tcl::build-info`. `tcl::prefix` and `tcl::clock`'s scripted ensemble helpers stay unmodelled by choice — implementation detail, not documented public surface |
| **Legacy `trace variable`/`vdelete`/`vinfo`** | removed 9.0 | 8.4–8.6 only | `TCL8X` gate |
| **Command resolution order** (current → path → global) | stable except the 8.5 path insert | `changes:6483` | Version-agnostic by design |
| **Core-builtin existence** (W123) | per row above | — | `registry.rs::get_for_dialect`; an 8.6-only construct draws unknown-command in an 8.4 file |

Two lessons the matrix has cost us twice, both worth restating:

1. **A version gate belongs on the version axis.** Putting a version
   restriction in a dialect's ad-hoc ban list (`IRULES_DISABLED_COMMANDS`)
   hides the fact that the command is version-gated everywhere else, and the
   ban list's own contract test warns against it. Conversely, a name that is
   genuinely dialect-independent (the `opt` package's `tcl::Opt*` commands,
   which predate namespaces and ship across every real 8.x/9.x) correctly
   *stays* `dialects: None` and is excluded from iRules for the real reason —
   its TMM sandbox has no `package` / `source` / `auto_load` to load it with.
2. **The per-spec gate and the profile shape are two axes, and both must
   agree.** The `tcl8.4` profile once carried `operators_as_commands: true`,
   so `::tcl::mathop::+` resolved under 8.4 even though its per-spec
   `TCL85_PLUS` gate had always been correct.

There is also no special protection on `::tcl` itself. Empirically verified
on a real `tclsh 8.6.14`: `rename ::tcl::mathop::+ ::foo`, `namespace delete
::tcl::mathop`, and even `namespace delete ::tcl` all succeed silently —
unlike the *global* namespace `::`, which cannot be deleted. `::tcl` is
merely unusually **load-bearing** (it hosts `::tcl::UnknownPending`, the
`unknown` auto-load recursion guard, plus the Tcl-scripted implementations
backing several C ensembles), so deleting it cascades into breaking most of
the interpreter. That is a footgun Tcl allows, not a locked door to model.

---

## 6. How conformance is kept

Faithfulness is asserted by execution, not by review:

- **Shared vectors.** `tcl-syntax/tests/data/command_resolution_vectors.txt`
  is executed by the pure resolver, the analyser settlement, the VM, and the
  WASM runtime — and by `vectors_match_real_tclsh`, which runs every vector
  under an installed `tclsh`. Adding a resolution behaviour means adding a
  vector, so drift fails a test rather than surviving a review.
- **Version-paired vectors.** The 8.x/9.0 variable-fallback table runs
  through the VM at both `RuntimeVersion`s *and* under real `tclsh8.6` and
  `tclsh9.0` (`TCL_LSP_TCLSH86` / `TCL_LSP_TCLSH90` overrides).
- **MRO.** `tcl_syntax::mro` asserts its linearisations against real tclsh
  9.0 `info class call` output, including the diamond and two-pass mixin
  cases.
- **Tricky interactions.** `tcl-vm/tests/tricky_resolution_e2e.rs` pins the
  alias, `forward`, mathfunc, `unknown`, `rename`, and colon-name
  interactions against both 8.6 and 9.0.
- **Selection drift.** `cargo xtask resolution-drift` (in `make xtask-check`)
  flags a new namespace-blind simple-name scan appearing outside the
  sanctioned helpers.

Fetch the C trees with `make fetch-tcl-source` (or the `fetch-tcl-source`
skill) before re-checking anything here.

## Related

- [name-resolution.md](name-resolution.md) — the model built on top of this
  algorithm.
- [contracts/command-resolution.md](contracts/command-resolution.md) — the
  rule, its single Rust home, and its consumers.
- KCS: [Why does a namespace variable behave differently on Tcl 8 and 9?](../kcs/kcs-qa-why-does-a-namespace-variable-behave-differently-on-tcl-8-and-9.md)
