# Related issues and prior work

GitHub metadata refreshed on 2026-10-12 across all 1,050 returned issues.
The #2300 closure was read back on 2026-10-13 at 00:32:48 UTC.
Issue metadata is a search snapshot; actual current state and narrow executed
acceptance are recorded below and in current-state.md. An open issue supplies an
acceptance target; its state alone does not establish current source behaviour.

The durable authorities are the [name model](../../design/analysis/name-resolution.md),
[command owner contract](../../design/contracts/command-resolution.md),
[canonical proof index](../../design/analysis/name-resolution-proofs/README.md)
and [consumer integration contracts](../../design/analysis/name-resolution-proofs/consumer-integrations.md).
Native observations, source anchors, implementation contracts and executed
Rust receipts retain separate authority.

## Starting user reports

- [#2286](https://github.com/bitwisecook/tcl-lsp/issues/2286), OPEN: W210 false positives across tcltest setup/body/cleanup, including optimiser/object variables, pdata, and pars built in a loop. This requires invocation identity plus registry body sequencing/frame semantics and definition/read tracking. It is not merely canonical spelling.
- [#2287](https://github.com/bitwisecook/tcl-lsp/issues/2287), CLOSED: tcltest import inside namespace eval loses highlighting. Closely paired consumer acceptance witness for the same namespace-aware invocation resolution.
- The refreshed tree includes all four reported #2286 source fixtures in
  `rust/tcl-compiler/tests/data/diagnostics/tcltest-shared-frame/`, five-profile
  W210 regressions, direct/captured-prefix alias controls, genuine-undefined-read
  negatives and namespace-scoped import controls. The earlier claim about
  `resolve_symbol_definer` is superseded; that helper is absent from this tree.
- [The retained native lifecycle question](../../design/analysis/name-resolution-proofs/tcltest-original-alias-caller-frame-lifecycle.md)
  measures all five C packages and Jim's distinct global-test protocol. It does
  not execute the full tclopt constructors or assert Rust diagnostics.
- Final current Tcltest diagnostic checks pass eight controls and fail the
  original-program missing-read control0versus2 at C8.4. #2286 remains
  open. Real matching init/Tcltest observations and exact compiler receipts are
  linked from [current-state.md](current-state.md).
- Both complete packaged highlighting regressions now pass, including the
  import moved inside `::nativeTest`, option pairs and nested commands.
  [Exact C-source receipts](../../../experiments/name-handling/results/c-source-namespace-advice/outcomes.json)
  bind unchanged source owners. Published `c7b94e8fb` resolves #2287;
  [verified closure](https://github.com/bitwisecook/tcl-lsp/issues/2287#issuecomment-6129655008)
  leaves #2286 and broader centralisation open.

## Open centralisation acceptance targets

| Issue | GitHub state | Relationship and acceptance |
|---|---|---|
| [#1778](https://github.com/bitwisecook/tcl-lsp/issues/1778) | OPEN | Stable structured namespace/command identity. Distinct a:/p and a/:p render the same FQN even in C Tcl; a rendered FQN cannot identify tokens. Thread identity through compiler, runtimes, sidecars, aliases/imports, traces, LSP indexes. [Codegen publication audit](../../../experiments/name-handling/results/codegen-root-colon-publication/audit.md) finds rendered qualified names sent back as original publication operands; direct C5/Jim controls distinguish :audit from :::transport. Emitted-WASM execution remains untested. |
| [#1755](https://github.com/bitwisecook/tcl-lsp/issues/1755) | OPEN | Compiled command-reference lifetime during delayed namespace deletion/recreation; cached absolute descendant must retain OLDQ token, yielding {OLDQ OLDQ} NEWQ. |
| [#1796](https://github.com/bitwisecook/tcl-lsp/issues/1796) | OPEN | One composed registry frame-effect owner with exact eval/uplevel/upvar reach; required variable-cell resolution dependency. |
| [#2259](https://github.com/bitwisecook/tcl-lsp/issues/2259) | OPEN | upvar/namespace upvar otherVar cell naming lacks a role; all registry-driven consumers must distinguish naming a cell from reading its value. |
| [#2260](https://github.com/bitwisecook/tcl-lsp/issues/2260) | OPEN | Empty variable name is a legal identity; never conflate empty literal name and unresolved name in SSA and O126. |
| [#2305](https://github.com/bitwisecook/tcl-lsp/issues/2305), [#2370](https://github.com/bitwisecook/tcl-lsp/issues/2370) | OPEN | Global-frame bare and absolute-qualified spellings address one place; SSA/value lattice, read/write tracking and every write kind must agree. |
| [#2306](https://github.com/bitwisecook/tcl-lsp/issues/2306) | OPEN | Same-frame upvar 0 writes must reach target identity and invalidate values/loop verdicts. |
| [#2410](https://github.com/bitwisecook/tcl-lsp/issues/2410) | OPEN | Qualified or alias-passed upvar name arguments must become actual global writes at callers. The prior baseline had a transfer-summary improvement with a remaining top-level witness; the refreshed source must be tested independently before retaining that conclusion. |
| [#2414](https://github.com/bitwisecook/tcl-lsp/issues/2414) | OPEN | Namespace variable writes must not overwrite an unrelated same-spelled caller local in W220 tracking. |
| [#2412](https://github.com/bitwisecook/tcl-lsp/issues/2412) | OPEN | variable ::cfg2 links local tail cfg2 to global cell; escaping/SSA/O126/W211 must retain that identity. |
| [#2297](https://github.com/bitwisecook/tcl-lsp/issues/2297), [#2415](https://github.com/bitwisecook/tcl-lsp/issues/2415) | OPEN | Caller reads through upvar and nested condition invocations must be tracked, preserving stores and avoiding W220. |
| [#2367](https://github.com/bitwisecook/tcl-lsp/issues/2367) | OPEN | Global write inside namespace eval :: must enter interprocedural summaries, also deferred callbacks. |
| [#2385](https://github.com/bitwisecook/tcl-lsp/issues/2385), [#2404](https://github.com/bitwisecook/tcl-lsp/issues/2404) | OPEN | Qualified foreach/lmap binders and body writes require shared place identity and conservative effects when lowering declines; purity must observe global writes. |
| [#2423](https://github.com/bitwisecook/tcl-lsp/issues/2423) | OPEN | Replacing ensemble implementation command must invalidate constant-fold trust for both direct and ensemble invocation. |
| [#2424](https://github.com/bitwisecook/tcl-lsp/issues/2424) | OPEN | Procedure definition in missing namespace raises; namespace existence/order must gate definition facts and folds. |

## Runtime/API dependencies and nearby conformance

| Issue | GitHub state | Relationship and acceptance |
|---|---|---|
| [#2193](https://github.com/bitwisecook/tcl-lsp/issues/2193), [#2269](https://github.com/bitwisecook/tcl-lsp/issues/2269) | OPEN | Shared qualified alias destination registration: create missing parents, namespace which, invoke, describe. Discussion adds token retention across rename, duplicate alias-token prefixes, global bare-alias creation and context-sensitive qualified names. Do not prematurely mark duplicates. |
| [#2275](https://github.com/bitwisecook/tcl-lsp/issues/2275) | OPEN | Child interpreter alias/aliases entry points must reuse same interpreter-arena alias/token/lifecycle owners. |
| [#2292](https://github.com/bitwisecook/tcl-lsp/issues/2292) | OPEN | Namespace/UnknownHandling guard invalidation ABI and engine event parity; do not base future lookup shortcuts on stale guards. |
| [#2300](https://github.com/bitwisecook/tcl-lsp/issues/2300) | CLOSED | Published `cac588499` uses shared pattern parts, direct live namespace tokens and authentic import-generation slots. Three genuine NativeC86 witness/adverse/chain controls pass; [accepted closure](https://github.com/bitwisecook/tcl-lsp/issues/2300#issuecomment-6135298423) verified 2026-10-13 00:32:48 UTC. Other-release Rust, native F5 and #1778 replacement/teardown remain unproved. |
| [#2311](https://github.com/bitwisecook/tcl-lsp/issues/2311) | OPEN | Runtime upvar 1 must resolve target through caller namespace-eval frame, not nonexistent frame slot. |
| [#2210](https://github.com/bitwisecook/tcl-lsp/issues/2210) | OPEN | EXIST_STK must consume resolved name exactly once; audit sibling LOAD/STORE/ARRAY stack forms. The refreshed Original ingress and downstream lookup owners must be tested with the exact resolved-name witness before stating fixed. |
| [#2342](https://github.com/bitwisecook/tcl-lsp/issues/2342), [#2343](https://github.com/bitwisecook/tcl-lsp/issues/2343) | OPEN | Adjacent lookup-result conformance: VM variable and runtime command errorCode fields. Name owner/resolution result must allow exact failures; not proof whole runtime error taxonomy is in scope. |
| [#2450](https://github.com/bitwisecook/tcl-lsp/issues/2450) | CLOSED | Direct documentation cleanup requirement: W210 note's obsolete Before this was registry-driven paragraph. That paragraph and #2117 mention are absent at the refreshed HEAD. Published removal commit `47c00bbad` is in PR #2457. GitHub readback confirms closure on 2026-10-12 with the published removal linked; repository-wide documentation cleanup remains separate. |

## Closed issues / merged prior work to preserve

| Item | GitHub state | Contract/evidence to retain |
|---|---|---|
| [#246](https://github.com/bitwisecook/tcl-lsp/issues/246) | CLOSED | Earlier cross-pass canonicalisation and registry hardcoded-knowledge audit; alias/namespace/mutation awareness, shared consumer resolution. Its string-canonicalisation sketch is superseded where display names are non-injective (#1778). |
| [#1473](https://github.com/bitwisecook/tcl-lsp/issues/1473) | CLOSED | Cross-surface owner/dialect audit. Final comments explicitly completed its tracked rows and retired audit artifacts in PR #1939; later follow-ons remain distinct. Do not revive archived Python-era paths as current consumer inventory. |
| [#1078](https://github.com/bitwisecook/tcl-lsp/issues/1078) | CLOSED | Brace-literal names such as {$n} must remain literal $n, distinct from n. |
| [#1448](https://github.com/bitwisecook/tcl-lsp/issues/1448), [#1454](https://github.com/bitwisecook/tcl-lsp/issues/1454) | CLOSED | Shared colon-run parsing for variable lookup and workspace import patterns; proper glob owner. |
| [#1493](https://github.com/bitwisecook/tcl-lsp/issues/1493) | CLOSED | Namespace qualifiers distinguish no qualifier from global namespace. Global namespace import must work. |
| [#1588](https://github.com/bitwisecook/tcl-lsp/issues/1588), [#1602](https://github.com/bitwisecook/tcl-lsp/issues/1602) | CLOSED | Parent namespace lookup errors and leading-brace/resolved-name exactness; #2210 shows need to probe every stack form despite closure. |
| [#1745](https://github.com/bitwisecook/tcl-lsp/issues/1745) | CLOSED | upvar relative-qualified target uses designated target frame's namespace. |
| [#1751](https://github.com/bitwisecook/tcl-lsp/issues/1751), [#1752](https://github.com/bitwisecook/tcl-lsp/issues/1752), [#1753](https://github.com/bitwisecook/tcl-lsp/issues/1753) | CLOSED | Delayed namespace command tables, observable teardown hash order, command/namespace sidecar token ownership across same-name recreation. Preserve while fixing #1755/#1778. |
| [#1763](https://github.com/bitwisecook/tcl-lsp/issues/1763), [#1764](https://github.com/bitwisecook/tcl-lsp/issues/1764) | CLOSED | Runtime command traces and TclOO class/object state keyed by incarnation tokens. |
| [#1648](https://github.com/bitwisecook/tcl-lsp/issues/1648) | CLOSED | Mutation visibility across compilation units and folded AssignExpr; compiled dispatch must share live binding trust. |
| [#1772](https://github.com/bitwisecook/tcl-lsp/issues/1772), [PR #1879](https://github.com/bitwisecook/tcl-lsp/pull/1879) | CLOSED / MERGED | One compiler cell-word owner in native_lowering::cells; compatibility text cannot be reparsed as a single variable reference. PR explicitly excludes full restructuring of direct native lowering. |
| [#2065](https://github.com/bitwisecook/tcl-lsp/issues/2065), [#2159](https://github.com/bitwisecook/tcl-lsp/issues/2159) | CLOSED | Namespace-local command realm shadows and fully-qualified expr shadow trust. The branch retains the namespace-local shadow fix; exact current consumer results remain independent of issue closure. |
| [#2134](https://github.com/bitwisecook/tcl-lsp/issues/2134) | CLOSED | Open-world procedure name parameters must not be globally specialised to one caller's alias target. |
| [#2166](https://github.com/bitwisecook/tcl-lsp/issues/2166) | CLOSED | Jim dialect/editor/CLI surface supports proc statics and two-word method names; comments reference jimsh 0.76–0.84. This does not fulfil latest-Jim oracle rerun requirement. |

## Evidence policy for continuation

Keep issue metadata, current branch witness results and oracle outcomes as separate fields. Treat closed issue reports as regression inputs, open issue bodies as hypotheses/witness candidates, and report implementation complete only after exact real-oracle-backed branch checks. Probe full C Tcl 8.4–9.1 version matrix and current latest Jim rather than assuming every reported C9 answer generalises. F5 must remain pending until the BIG-IP chat supplies versioned raw answers in branch documents. A lack of a test or an unresolved static name means decline conservatively, never invent an identity.


## Wider issue-search inventory

Search covered all 1,050 repository issues returned by `gh issue list --state
all --limit 2000`, title families for lookup/namespaces/aliases/colon runs,
W210/scopes/tcltest/shadowing/centralisation, and bodies/discussions of the
primary relationships above. This is a reproducible discovery scope, not a
claim that keyword matching finds every related defect. Refresh with body
searches and follow links before implementation. PR #1879 was read separately.

The following additional matches are adjacent regressions or historical
context. Triage their actual body and branch reproduction into the relevant
consumer row; do not silently widen name centralisation into unrelated product
work merely because a title contains a keyword.

| Issue | GitHub state | Title |
| --- | --- | --- |
| [#23](https://github.com/bitwisecook/tcl-lsp/issues/23) | CLOSED | Rule W210 Flagging Incorrectly |
| [#40](https://github.com/bitwisecook/tcl-lsp/issues/40) | CLOSED | Static analysis scope |
| [#42](https://github.com/bitwisecook/tcl-lsp/issues/42) | CLOSED | Adding aliases to known list of symbols |
| [#61](https://github.com/bitwisecook/tcl-lsp/issues/61) | CLOSED | Commands of tcltest package are not highlighted |
| [#62](https://github.com/bitwisecook/tcl-lsp/issues/62) | CLOSED | W210 false triggering |
| [#87](https://github.com/bitwisecook/tcl-lsp/issues/87) | CLOSED | W210 false triggering in dict context |
| [#189](https://github.com/bitwisecook/tcl-lsp/issues/189) | CLOSED | lower: namespace_imports/exports recorded from dead code can globalise calls incorrectly |
| [#268](https://github.com/bitwisecook/tcl-lsp/issues/268) | CLOSED | WASM runtime tests: cmds/{eval,proc,namespace,oo} + interp/* (wave 5) |
| [#276](https://github.com/bitwisecook/tcl-lsp/issues/276) | CLOSED | WASM runtime: tracking — upstream Tcl 9 I/O tcltest suite parity |
| [#280](https://github.com/bitwisecook/tcl-lsp/issues/280) | CLOSED | tcltest 2.5 bootstrap fails on WASM runtime — blocks upstream tcltest sweeps |
| [#281](https://github.com/bitwisecook/tcl-lsp/issues/281) | CLOSED | clock: remaining tcltest gaps — scan -format, clock-5 free-form, mathop ensemble |
| [#310](https://github.com/bitwisecook/tcl-lsp/issues/310) | CLOSED | Feature request - file scope suppressing |
| [#327](https://github.com/bitwisecook/tcl-lsp/issues/327) | CLOSED | tcltest: 8 Tcl 9 .test files still failing the in-tree gate |
| [#637](https://github.com/bitwisecook/tcl-lsp/issues/637) | CLOSED | Issue with syntax highlighting on VSCode related to shadowing build-ins |
| [#756](https://github.com/bitwisecook/tcl-lsp/issues/756) | CLOSED | W210 variable read before it is set |
| [#776](https://github.com/bitwisecook/tcl-lsp/issues/776) | CLOSED | tcltest package highlightning |
| [#936](https://github.com/bitwisecook/tcl-lsp/issues/936) | CLOSED | VM parity follow-up: trace-aware compilation (16.3) and interp-arena ownership for full cross-interp alias parity (16.2) |
| [#958](https://github.com/bitwisecook/tcl-lsp/issues/958) | CLOSED | preview version :  The ::tcl::mathfunc namespace cannot find any references when functions are added in expr. |
| [#973](https://github.com/bitwisecook/tcl-lsp/issues/973) | CLOSED | W123 doesn't fire for a call through a proc/class/alias/rename target deleted earlier in the file |
| [#979](https://github.com/bitwisecook/tcl-lsp/issues/979) | CLOSED | Investigate: does `namespace ensemble configure -map` redirection break interprocedural call-site resolution? |
| [#980](https://github.com/bitwisecook/tcl-lsp/issues/980) | CLOSED | `uplevel #0 { ... }` body resolves against the wrong namespace in interprocedural call-site analysis (confirmed, root cause identified) |
| [#981](https://github.com/bitwisecook/tcl-lsp/issues/981) | CLOSED | Bare classmethod/object-command dispatch matching is namespace-blind (can cross-link the wrong class) |
| [#988](https://github.com/bitwisecook/tcl-lsp/issues/988) | CLOSED | 11 private ::tcl:: ensemble-backing sub-namespaces are undocumented and unmodeled |
| [#1002](https://github.com/bitwisecook/tcl-lsp/issues/1002) | CLOSED | Document that rename/interp-alias of apply (or any registry-special command) defeats ArgRole-driven handling (e.g. LambdaLiteral) |
| [#1006](https://github.com/bitwisecook/tcl-lsp/issues/1006) | CLOSED | W123 alias/rename-target checks use file-end-only deletion gating, not call-site/conditional-aware (follow-up to #973) |
| [#1007](https://github.com/bitwisecook/tcl-lsp/issues/1007) | CLOSED | Arity checker (E002/E003) silently abstains when a rename/alias deletion sits inside a never-triggered proc body |
| [#1009](https://github.com/bitwisecook/tcl-lsp/issues/1009) | CLOSED | Constant-$cmd dispatch settlement resolves through a renamed/deleted-away proc, class, alias, or rename target with no deletion gate |
| [#1022](https://github.com/bitwisecook/tcl-lsp/issues/1022) | CLOSED | W210 false-fires inside a dynamically-resolved -safe interp eval body on a plain set-then-read (pre-existing, predates PR #1020's rebase) |
| [#1027](https://github.com/bitwisecook/tcl-lsp/issues/1027) | CLOSED | namespace import resolved against final export state, not per-import-site snapshot (Codex review, PR #1020) |
| [#1031](https://github.com/bitwisecook/tcl-lsp/issues/1031) | CLOSED | `Traits` bit 61 is assigned twice: `SAFE_INTERP_HIDDEN` aliases `TRANSFERS_CONTROL` (the `u64` is full) |
| [#1051](https://github.com/bitwisecook/tcl-lsp/issues/1051) | CLOSED | Multi-word `eval` (word-concatenation form) draws false E002/W210 — the analyser reads only the first argument as the script |
| [#1056](https://github.com/bitwisecook/tcl-lsp/issues/1056) | CLOSED | VM: `namespace inscope` space-joins its trailing args — C Tcl list-appends them (`concat script [list args…]`) |
| [#1058](https://github.com/bitwisecook/tcl-lsp/issues/1058) | CLOSED | runtime/rust: command-resolution conformance vector fails — braced proc name corrupts dispatch (`invalid command name "if"`) |
| [#1064](https://github.com/bitwisecook/tcl-lsp/issues/1064) | CLOSED | Go-to-definition ignores `rename` for procs and classes (single-file path) — `resolve_class_target_at` / `lookup_alias` need the rename hop |
| [#1070](https://github.com/bitwisecook/tcl-lsp/issues/1070) | CLOSED | False W210 on apply lambda parameters in bare-statement position: `apply {{a} {puts $a}} 5` reads `a` as read-before-set |
| [#1077](https://github.com/bitwisecook/tcl-lsp/issues/1077) | CLOSED | lower_proc homes a body-defined proc to its lexical namespace, not its defining namespace (W214 names the wrong FQN) |
| [#1088](https://github.com/bitwisecook/tcl-lsp/issues/1088) | CLOSED | Namespace names are not navigable symbols: `namespace children ::tomato` resolves to nothing |
| [#1093](https://github.com/bitwisecook/tcl-lsp/issues/1093) | CLOSED | Namespace-variable rename refusal for dynamic names is blunter than needed (per-site provenance would allow safe renames) |
| [#1096](https://github.com/bitwisecook/tcl-lsp/issues/1096) | CLOSED | `namespace qualifiers` / `namespace tail` have no const_fold (blocks the ticklecharts `${ns}::setdef` chain) |
| [#1098](https://github.com/bitwisecook/tcl-lsp/issues/1098) | CLOSED | Block-scoped `export`/`unexport` inside `self { … }` abstains — `set_member_visibility` flips both instance and class-side tables by bare name |
| [#1103](https://github.com/bitwisecook/tcl-lsp/issues/1103) | CLOSED | Import-edge lifecycle is not modelled: `namespace forget`, `-force` shadowing, source rename/delete following, import chains |
| [#1108](https://github.com/bitwisecook/tcl-lsp/issues/1108) | CLOSED | Variable reference completeness: `[set X]` reads unrecorded in scope references; cursor inside a brace-literal name answers empty |
| [#1113](https://github.com/bitwisecook/tcl-lsp/issues/1113) | CLOSED | Namespace navigation coverage refinements: implicit parents, `namespace path` lists, computed targets, apply-lambda namespace element |
| [#1114](https://github.com/bitwisecook/tcl-lsp/issues/1114) | CLOSED | Namespace rename tier: renaming a namespace must rewrite every qualified name beneath it, with the refusal gate for what it can't prove |
| [#1137](https://github.com/bitwisecook/tcl-lsp/issues/1137) | CLOSED | Bareword command resolution in `definition`/`hover`: no command-position gate (wrong answers) and no `::oo::Helpers` / cross-file tail fallback (missed answers) |
| [#1142](https://github.com/bitwisecook/tcl-lsp/issues/1142) | CLOSED | Un-hooked command bodies get a naive walk: false W210/E003 and inert navigation inside a `tcl::OptProc`-shaped definer body |
| [#1194](https://github.com/bitwisecook/tcl-lsp/issues/1194) | CLOSED | Minifier-generated alias variables can shadow live Tcl variables |
| [#1218](https://github.com/bitwisecook/tcl-lsp/issues/1218) | CLOSED | Cosmetic provider polish: leading-BOM W123, and namespace eval selectionRange spans the whole body |
| [#1237](https://github.com/bitwisecook/tcl-lsp/issues/1237) | CLOSED | False W210 on a braced data argument: `puts {set y 1; return $y}` reports read-before-set |
| [#1245](https://github.com/bitwisecook/tcl-lsp/issues/1245) | CLOSED | `is_dynamic_word` on a reconstructed braced word: `namespace path {$ns ::a}` is skipped entirely |
| [#1246](https://github.com/bitwisecook/tcl-lsp/issues/1246) | CLOSED | Cross-document namespace navigation does not answer implicit parents |
| [#1247](https://github.com/bitwisecook/tcl-lsp/issues/1247) | CLOSED | W210 false-fires again on a `dict update` value-var: `fp_depth::bug_dict_update_value_var` is red |
| [#1260](https://github.com/bitwisecook/tcl-lsp/issues/1260) | CLOSED | W210 false positive: a `$var` inside a braced `foreach` / `lmap` list word is read as a substitution |
| [#1261](https://github.com/bitwisecook/tcl-lsp/issues/1261) | CLOSED | `namespace path` list entries have no per-element span, so the namespace rename tier refuses instead of rewriting them |
| [#1262](https://github.com/bitwisecook/tcl-lsp/issues/1262) | CLOSED | Dynamic variable-name provenance stops at the written word — no per-site constant-value record survives analysis |
| [#1301](https://github.com/bitwisecook/tcl-lsp/issues/1301) | CLOSED | `refresh_source_rehoming_locked` never converges for a document `source`d from the global namespace, dropping every `WorkspaceIndex` `Derived` memo on every navigation request |
| [#1302](https://github.com/bitwisecook/tcl-lsp/issues/1302) | CLOSED | The workspace import-conflict gate does not know about registry builtins: a wildcard `namespace import` binds a name real Tcl refuses to import, giving find-references a false positive on every builtin the source namespace exports |
| [#1328](https://github.com/bitwisecook/tcl-lsp/issues/1328) | CLOSED | `runtime/rust` diverges from C Tcl on `namespace eval` + `append` to a global, and `dict lappend` onto a non-dict silently succeeds |
| [#1388](https://github.com/bitwisecook/tcl-lsp/issues/1388) | CLOSED | Wire `RepeatedArgLayout::optional_leading_word` to `upvar` and collapse the four hand-rolled scope-alias layout parsers |
| [#1403](https://github.com/bitwisecook/tcl-lsp/issues/1403) | CLOSED | safe_on_uninit is never lifted from the registry into the IR, so W210's self-init suppression is dead |
| [#1408](https://github.com/bitwisecook/tcl-lsp/issues/1408) | CLOSED | Three compiler integrations are declared but never wired: memory-SSA aliasing, dataflow alias/clobber edges, and execution-intent |
| [#1421](https://github.com/bitwisecook/tcl-lsp/issues/1421) | CLOSED | command_segment_on_line's expand-selection semicolon scan ignores {}/[] nesting, truncating the selection mid-word |
| [#1427](https://github.com/bitwisecook/tcl-lsp/issues/1427) | CLOSED | `dict` in `tcl-vm` never collapses duplicate keys: two local `dict_pairs` copies do first-match lookup where `ValueOps::dict_pairs` (and C Tcl) take the last value |
| [#1442](https://github.com/bitwisecook/tcl-lsp/issues/1442) | CLOSED | `namespace origin` in tcl-vm returns the *caller's* qualification, never the import source or the resolved command — and `namespace which -variable` answers with proc locals |
| [#1446](https://github.com/bitwisecook/tcl-lsp/issues/1446) | CLOSED | `namespace export -clear` never clears in tcl-vm and both runtimes ignore the registry's tclsh-pinned `max_leading_option_words: Some(1)` for `export`/`import` |
| [#1447](https://github.com/bitwisecook/tcl-lsp/issues/1447) | CLOSED | `TclPreventAliasLoop` implemented in tcl-vm but absent from runtime/rust: `interp alias {} a {} b; interp alias {} b {} a; a` installs fine and recurses natively without bound |
| [#1451](https://github.com/bitwisecook/tcl-lsp/issues/1451) | CLOSED | Namespace deletion is implemented twice with different teardown: tcl-vm leaves `namespace path` entries, `namespace unknown` handlers and ensemble commands behind, and resurrects them when the namespace is recreated |
| [#1453](https://github.com/bitwisecook/tcl-lsp/issues/1453) | CLOSED | Ensemble subcommand resolution and the `namespace ensemble create` option table are hand-rolled twice (tcl-vm inline, runtime/rust `ensemble.rs`) instead of `tcl-cmd-core::prefix`, and disagree with C on which options exist and abbreviate |
| [#1456](https://github.com/bitwisecook/tcl-lsp/issues/1456) | CLOSED | scan_var_name / parse_var_ref_parts stop after two colons, splitting $a:::b into var a:: plus literal :b |
| [#1459](https://github.com/bitwisecook/tcl-lsp/issues/1459) | CLOSED | simple_var_name (wasm/backend.rs) and variable_name (wasm/leaf_invoke.rs) are byte-for-byte the same $name/${name} extractor, twice |
| [#1467](https://github.com/bitwisecook/tcl-lsp/issues/1467) | CLOSED | `tcl-fuzz` can pin the subject engine's Tcl release but not the reference's or the replay's, and the pinned release is not part of the findings namespace |
| [#1469](https://github.com/bitwisecook/tcl-lsp/issues/1469) | CLOSED | VS Code/JetBrains/Sublime hand-roll number/escape/variable regexes that miss 9.0 forms and aren't colon-run aware |
| [#1492](https://github.com/bitwisecook/tcl-lsp/issues/1492) | CLOSED | runtime/rust: a missing alias target errors directly instead of consulting `unknown` |
| [#1575](https://github.com/bitwisecook/tcl-lsp/issues/1575) | CLOSED | Unset-trace firing sites are incomplete: proc-frame teardown (runtime/rust), namespace teardown (tcl-vm), and per-element firing on whole-array unset (both) |
| [#1577](https://github.com/bitwisecook/tcl-lsp/issues/1577) | CLOSED | runtime/rust: lassign/catch/regexp/scan/binary scan/foreach write array-element variable names as literal scalars |
| [#1582](https://github.com/bitwisecook/tcl-lsp/issues/1582) | CLOSED | tclvm: `upvar` linking a proc local to a namespace variable is silently accepted — C raises "can't create namespace variable that refers to procedure variable" |
| [#1584](https://github.com/bitwisecook/tcl-lsp/issues/1584) | CLOSED | VM namespace-surface validation gaps: declared-unset variables invisible, `namespace which` skips option/arity checks, import self/unknown-source checks missing, `namespace delete ::` errors |
| [#1585](https://github.com/bitwisecook/tcl-lsp/issues/1585) | CLOSED | Constant-folding family folds by dialect only: a renamed, deleted, or proc-shadowed `dict`/`list`/`format` still gets the compile-time fold |
| [#1593](https://github.com/bitwisecook/tcl-lsp/issues/1593) | CLOSED | W210 for classvariable variable |
| [#1604](https://github.com/bitwisecook/tcl-lsp/issues/1604) | CLOSED | compiler: ~20 free-text `${…}` scanners hard-code the 8.x first-`}` rule (and ~15 more use a last-`}` rule) across optimiser/dataflow/taint/analyser — #1568's class, outside its declared scope |
| [#1610](https://github.com/bitwisecook/tcl-lsp/issues/1610) | CLOSED | registry: `ENSEMBLE_CREATE_OPTIONS` serves both `namespace ensemble create` and `configure`, but the real option sets differ — completion offers a guaranteed error and hides a valid option |
| [#1612](https://github.com/bitwisecook/tcl-lsp/issues/1612) | CLOSED | analyser: `namespace export` records patterns past an invalid one — C aborts the loop at the first bad pattern |
| [#1613](https://github.com/bitwisecook/tcl-lsp/issues/1613) | CLOSED | runtime: `namespace exists`/`parent`/`children` still take the lossy `&str` route — byte-valued machinery now exists, convert them |
| [#1616](https://github.com/bitwisecook/tcl-lsp/issues/1616) | CLOSED | VM: quoted variable names in `set` are stored under their unsubstituted source spelling — `set "a\\" K` creates `a\\` (3 bytes) where tclsh creates `a\` (2 bytes) |
| [#1625](https://github.com/bitwisecook/tcl-lsp/issues/1625) | CLOSED | Extension→dialect registration is only partially generator-driven: 6 drifted hand-maintained surfaces (.tmsh can't activate VS Code; .irules not indexed or recognised by f5-cli; Sublime per-dialect scopes wrong) |
| [#1703](https://github.com/bitwisecook/tcl-lsp/issues/1703) | CLOSED | TclOO method references miss list/namespace-code callback prefixes in CommandPrefix slots |
| [#1737](https://github.com/bitwisecook/tcl-lsp/issues/1737) | CLOSED | tclvm: Tcltest driver bypasses Tcl 9 init.tcl and installs a hand-picked startup subset |
| [#1748](https://github.com/bitwisecook/tcl-lsp/issues/1748) | CLOSED | TclVM namespace reparse and child hash state survive live mutation |
| [#1928](https://github.com/bitwisecook/tcl-lsp/issues/1928) | CLOSED | info consts enumerates aliases and misses namespace global fallback |
| [#1935](https://github.com/bitwisecook/tcl-lsp/issues/1935) | CLOSED | Rename refused as a namespace collision against a variable that is not in any document |
| [#1951](https://github.com/bitwisecook/tcl-lsp/issues/1951) | CLOSED | range_semantic_tokens_no_spurious_refresh_when_converged is flaky: its request census is not scoped to the round it asserts about |
| [#2072](https://github.com/bitwisecook/tcl-lsp/issues/2072) | CLOSED | Three tests assert less than their names promise (`irules_alias_dialect_accepted`, entry-protocol lowering, `empty_source_returns_empty_chain`) |
| [#2142](https://github.com/bitwisecook/tcl-lsp/issues/2142) | CLOSED | A `try` statement erases the definiteness of names bound before it: W210 after `try … finally`, and O109 deletes the `finally` body's store |
| [#2164](https://github.com/bitwisecook/tcl-lsp/issues/2164) | CLOSED | O100 and O103 disagree about a shadowed builtin in one statement: `set v [llength {a b c}]; return $v` becomes `set v 99; return 3` |
| [#2168](https://github.com/bitwisecook/tcl-lsp/issues/2168) | CLOSED | A computed `rename` still miscompiles a shadowed builtin — and O102 proves the names in the same run without feeding them back |
| [#2185](https://github.com/bitwisecook/tcl-lsp/issues/2185) | CLOSED | tcltest parity sweep omits 68 upstream Tcl 9 test files, including portable core suites |
| [#2217](https://github.com/bitwisecook/tcl-lsp/issues/2217) | OPEN | O103 folds a pure-proc call without the `expr`-trust gate O110 consults: a shadowed `expr` is evaluated as the builtin |
| [#2246](https://github.com/bitwisecook/tcl-lsp/issues/2246) | OPEN | The W210 `dict with` harvest's same-block literal fallback reads through an intervening `dict unset` |
| [#2270](https://github.com/bitwisecook/tcl-lsp/issues/2270) | OPEN | An unscoped pack `-override` loses to a scoped shipped spec in every profile-aware lookup (`best_visible` prefers scoped specs) |
| [#2290](https://github.com/bitwisecook/tcl-lsp/issues/2290) | OPEN | Another W210 false positive |
| [#2323](https://github.com/bitwisecook/tcl-lsp/issues/2323) | OPEN | Writes inside a `catch` body nested in a command substitution are invisible to the value lattice: wrong I230 and false W210 |
| [#2335](https://github.com/bitwisecook/tcl-lsp/issues/2335) | OPEN | W210 false positive: a read inside a loop after a call the module cannot see is reported as read before set |
| [#2371](https://github.com/bitwisecook/tcl-lsp/issues/2371) | OPEN | W210 is silent on a read after `unset` inside an `info exists` guard |
| [#2372](https://github.com/bitwisecook/tcl-lsp/issues/2372) | OPEN | The W210 KCS note promises `info vars` and `catch {set _ $X}` idiom narrowing that the analyser does not do |
| [#2374](https://github.com/bitwisecook/tcl-lsp/issues/2374) | OPEN | W210 never reports an array element read once any element of the array is set |
| [#2376](https://github.com/bitwisecook/tcl-lsp/issues/2376) | OPEN | W210 fires on the short-circuited read inside an `info exists` guard's own condition, and its per-variable dedupe then hides a real unguarded read |
| [#2379](https://github.com/bitwisecook/tcl-lsp/issues/2379) | OPEN | W210 reports `array size`, `array names` and `array get` on a possibly unset array as a read before set; tclsh returns 0 or the empty list |
| [#2441](https://github.com/bitwisecook/tcl-lsp/issues/2441) | OPEN | W210 is silent on a read after a `foreach` over an empty list, although I230 already decides `[info exists x]` false there |
