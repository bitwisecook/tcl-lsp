# Deferred code-review findings

Findings from external code review that are **verified real** but deferred,
with root cause, fix approach, and why they were not fixed immediately. Fixed
findings are in the git history (search the relevant commit messages); this file
tracks only what remains. Each entry notes whether it is a **false positive**
(shows a wrong diagnostic — highest priority), a **false negative** (misses a
real one), or **sound-but-imprecise / design**.

Ground truth throughout is C tclsh 9.0.3.

---

## A. Incremental-analysis staleness + chunk cache (cluster) — FIXED (1–4); §5 reclassified

All four incremental≡full *divergences* are fixed and pinned by
`TestIncrementalStaleness` (which compares diagnostic **positions**, not just
codes — the gap that let these through).

1. **Chunk cache hashed only command text. — FIXED.** `find_first_dirty_chunk`
   now compares `start_offset` as well as `source_hash`, so a chunk whose text
   is unchanged but which *moved* (blank line / comment / directive inserted
   above) is dirty from the shift point (an append shifts nothing, so it stays
   incremental). The dialect angle is covered by a dedicated guard: `update()`
   compares the resolved `dialect_hint` and forces a full rebuild (clearing the
   per-proc/interproc caches) when it changes — covering both a directive *add*
   (position shift) and a same-length *replace* (`tcl8.4`→`tcl8.6`).
2. **Proc cache reused stale ranges. — FIXED.** `_proc_cache_key` /
   `_build_proc_cache` now include the proc's start **line+char** (the LSP-
   visible position), so a moved-but-unchanged proc is recomputed with correct
   positions; an offset-only shift (same-line length change before it) keeps the
   displayed line/char correct, so the unit is still reused. `force_reanalyse`
   now also clears the per-proc/interproc caches (their keys omit the dialect).
3. **Analyser snapshots not independent for TclOO. — FIXED.** `ClassDef.copy()`
   copies the mutable method/var containers; `copy_for_snapshot` and
   `_copy_tree` use it (sharing one copy per class via an id-map so the live
   `scope.classes[x] is all_classes[y]` identity is preserved). A later
   `oo::define` no longer mutates an earlier snapshot.
4. **Restored analysis dropped cumulative state. — FIXED.** `AnalyserSnapshot`
   now carries `objdefined_vars` and `ensemble_namespaces`; `snapshot()`/
   `restore()` round-trip them, so `oo::objdefine`/ensemble state survives an
   edit to a later chunk.
5. **"Restored analysis skips inline stubs in dirty chunks" — RECLASSIFIED (not
   an incremental divergence).** Verified: a `DocumentState` *full* rebuild and
   the incremental path **agree** (both omit the stub-body W123 that standalone
   `get_diagnostics` reports), so the incremental≡full contract holds. The real
   gap is that the `DocumentState` analysis path doesn't process an inline
   `# tcl-lsp: stub` body the way `get_diagnostics` does — a separate
   DocumentState-vs-standalone stub-handling issue, not staleness. Left as a
   distinct follow-up.

## B. TclOO class extraction — not scope/execution aware

6. **`_extract_class_names` ignores scope/execution. — WON'T FIX (FP risk
   dominates; same trap as §E.10/§E.11).** Real: a class in `if {0} {…}` or an
   unrun proc is never defined (tclsh errors `invalid command name "Foo"` on its
   use), so the over-broad registration masks a genuine diagnostic.  But the fix
   — register only "reachable" classes — cannot statically tell "never
   reachable" (`if {0}`, unrun proc) from the **dominant** "deferred/conditional
   but does run" pattern: classes defined inside a proc that *is* called, a
   `namespace eval`, or `if {[package vsatisfies …]} { oo::class create … }`.
   The corpus has 231 `oo::class`/`oo::define` sites, many in procs/namespaces;
   narrowing registration would drop their known-class status and fire W307/W308
   FPs on legitimate objects — net-negative, with the rare never-run-class typo
   the only genuine catch.  The conservative (register-anything-defined) behavior
   is the right trade-off.  A sound fix needs interprocedural call-reachability
   (does `define` ever run?) — a much larger effort, low payoff.
7. **Namespaced / absolute class names mishandled. — FIXED.** Both failure
   modes are resolved: the analyser (`_handle_oo_class_command` /
   `_handle_oo_define_command` / `_handle_snit_type_command`) now composes the FQ
   name via `_qualify_oo_name` (= `_namespace_from_scope` + `normalise_qualified_
   name`), so `oo::class create ::Foo` records `::Foo` (not `::::Foo`) and a
   relative name resolves against the enclosing namespace (`::ns::Foo`,
   `::a::b::C`); and the compiler side now resolves relative-in-namespace object
   typing — `_extract_class_names` recurses `namespace eval` `IRBlock`s and
   records `::ns::Foo`, while `_return_type_for_command` resolves a relative
   `[Foo new]` against the call site's namespace (derived from `cfg.name` in
   `_type_propagation`). Result: `$o nosuch` on `[::Foo new]` and on a relative
   `[Foo new]` inside `namespace eval ns` both get W308; valid methods stay
   silent. Corpus delta: W307 −1, W308 ±0, nothing else; bytecode identity
   unchanged.

---

## C. Lazy evaluation in the analyser's expr command walk

8. **W123 on `[cmd]` in a dead `&&`/`||`/ternary arm. — FIXED (constant-guard
   pruning, NOT `walk_eager`).** `_analyse_expr` now parses the expr (behind a
   cheap `[`-and-operator pre-filter) and, via `expr_ast.dead_command_ranges`,
   skips W123 only for command subs in arms made unreachable by a **literal-
   constant** guard — `0 && [cmd]`, `1 || [cmd]`, `0 ? [cmd] : x`, `1 ? x :
   [cmd]`.  Crucially this is *not* `walk_eager`: that treats every `&&`/`||`
   right operand as dead and would suppress W123 on the common
   `$cond && [unknownCmd]` (an FN regression).  A non-constant guard leaves both
   arms live, so `$cond && [missing]` and `1 && [missing]` (which tclsh errors
   on) still warn.  Position-precise (matches command-sub `(start,end)`), so no
   text-collision over-suppression.  Corpus delta: 0 (W123 is opt-in and
   literal-constant guards are near-zero frequency); pinned by unit tests.

---

## D. Soundness edges

9. **`overlap` under-approximates two aliases of the *same* caller var. —
   FIXED.** `compiler/place.py:overlap` now overlaps two non-dynamic
   `UPVAR_ALIAS` places on `owner` alone (the resolved caller var), so
   `upvar 1 caller_x a; upvar 1 caller_x b` are a must-alias regardless of the
   local name; distinct owners stay disjoint. `test_place.py` covers both.

---

## E. Precision false-negatives (lower priority than FPs)

10. **Dynamic array-index writes miss read-before-set. — WON'T FIX (corpus
    net-negative, like §E.11).** The finding is real — `set a($i) 1` reads `$i`
    (tclsh errors when unset) — and the fix (adding the index var to SSA
    `_uses`) is *sound*.  But the corpus delta was **+1 W210 and +1 S101, both
    false positives, zero genuine catches**: a dynamic array index in real code
    is almost always a loop/conditional var (`set arr($key) …`, `set DATA([list
    … $ip]) …`) that *is* set but not *provably* so, so it surfaces (a) the
    pre-existing read-before-set loop-carried limitation — confirmed identical
    for a scalar read at the same site (`copyops.tcl:82` `$key`), and (b) a
    downstream shimmer FP once the index var becomes a tracked use
    (`bench_read.tcl:134` `$ip`).  A genuinely-unset index is a rare typo, so the
    fix is FP-dominated in practice.  The existing **suppress-only** Place-bridge
    recovery (dead-store/unused) stays — it's the safe direction; promoting index
    reads into W210/shimmer is not.  Reverted.
11. **W113 `::`-anywhere suppression hides Tcl 9 namespaced builtins. —
    WON'T FIX (confirm-correct audit; the blanket `::`-drop is the right
    trade-off).** Attempted the reviewer's fix — keep a *core* qualified builtin
    (no `required_package`) and only suppress library commands (which carry
    one). It is **net-negative**: the registry's "core" set (no
    `required_package`) includes bundled-Tcl *script* commands the stdlib itself
    defines, so it flagged `tmp/tcl9.0.3/library/tm.tcl:358 proc ::tcl::tm::roots`
    — a false positive (that file *is* the command's definition, not a shadow).
    The corpus delta was W113 +1, all FP. There is no reliable static signal to
    tell a C-builtin a user must not redefine (`::tcl::process`) from a bundled
    script command defined in its own source (`::tcl::tm::roots`): both have no
    `required_package` and `dialects` is not a C-vs-script marker (plain `set`
    has `dialects=None` too). So the blanket `::`-drop stands — it accepts the
    near-never user-shadow FN (`proc ::tcl::process {}` in user code) to avoid
    flagging every stdlib/package self-definition. Reverted.
12. **W232 measured source-text length, not Tcl character length. — FIXED.**
    `_string_length_map` now resolves backslash escapes (`backslash_subst`) for a
    quoted/bare value (`IRAssignValue.value_needs_backsubst`), so `"a\nb"` is 3
    Tcl chars and `string index $s 3` correctly fires W232; a braced value
    (`IRAssignConst`) and a bare value with no escapes keep their source length
    (`{a\nb}` stays 4, index 3 in range).  The quoted-vs-braced word type *was*
    available after all — the `value_needs_backsubst` flag on `IRAssignValue`.
    tclsh-9.0.3-verified; corpus delta 0 (no escaped-string dynamic-index
    out-of-range in the corpus); pinned by unit tests.

---

## F. Algorithmic / sound-but-imprecise (design notes)

13. **`type_join` is non-associative across >2 incompatible types. — DEFERRED
    (no observable bug; risk > value).** `STRING ⊔ INT ⊔ DOUBLE` depends on fold
    order, but a prior commit (`b08f2c4`) sorted the predecessors, so the output
    is already **deterministic and sound** — this is a design-cleanliness issue,
    not a user-visible bug. A true-LUB rewrite (collect leaf types → numeric-
    promotion closure → OVERDEFINED iff `>2` types or an incompatible non-numeric
    pair) would change `SHIMMERED`-vs-`OVERDEFINED` results for some 3+-type
    merges, shifting S100/S101/S102 shimmer output corpus-wide with no clear
    net-positive (the §E.10/§E.11 pattern). Not worth the shimmer churn until a
    concrete wrong-output case motivates it.
14. **Interval domain handles only ADD/SUB/MUL/NEG + comparisons. — DEFERRED
    (capability, ~zero corpus value).** DIV / MOD / SHIFT / BIT_* / POW fall
    through to TOP (sound — conservative). Extending them is *additive precision*
    with no FP risk, but ~zero corpus impact: clean libraries have no provable
    divide-by-zero, and `$x % $n` narrowing to `[0, n-1]` can only *confirm*
    in-range (TOP already produced no false W230 there), so it neither adds nor
    removes findings on real code. Real effort (correct interval arithmetic for
    5+ operators incl. sign/zero handling) for theoretical gain — deferred until a
    motivating buggy-code case appears.
15. **`_trace_target` misses array-element traces. — NOT A BUG (verified).**
    `build_resolve_context` records the trace target via
    `normalise_var_name("arr(elem)")` → base `arr`, and the `arr(elem)` write
    place is observed=True. The reviewer's reading was a stale-checkout artifact;
    confirmed correct in the current tree (`def_places` shows `observed=True`).
16. **`_collect_upvar_aliases` accepts embedded substitutions in the target. —
    FIXED.** A target containing `$`/`[` *anywhere* (not just as a prefix —
    `upvar 1 prefix_$x var`) is now recorded as dynamic (`""`).
17. **`namespace eval $ns {proc …}` registers inner procs under the enclosing
    namespace**, not `$ns` — cross-reference / W308 / go-to-definition for inner
    symbols are approximate. Accepted as a documented best-effort limitation.
18. **`Place.base` drops `dynamic`/`name_reads`** when projecting an
    `ARRAY_ELEM` to `ARRAY_WHOLE`; no current consumer relies on it, but a future
    one would lose the alias uncertainty. Fix when a consumer needs `.base`.
19. **`_qualified_variable_alias_tails` and `_read_before_set` foreach-varlist
    parsing** assume strict IR normalisation (`range(0, len, 2)` pairing; naive
    `.split()` of a varlist). Low impact given current lowering; revisit if the
    IR ever delivers un-paired or brace-retained forms.
20. **`check_loop_termination` (W242) is text-level** (`"[" not in cond_text`),
    so `while {$x < $arr([0])}` (literal array index) disables the check. Minor.

---

## Already fixed in this review round (for reference, not deferred)

- Lazy-eval FPs in the interval/bounds/divide-by-zero walkers (W230/W233).
- W307 self-ref exemption scoped to snit bodies (+ `hull`); factory-object
  provenance scoped to the defining proc.
- Qualified-builtin arg-role miss (`::info exists`); return-terminator
  read-before-set (W210 on `return $x`).
- eval-mutation SCCP invalidation; try non-`on ok` handler reads.
- Literal array-element command-sub writes (`catch {…} msg(0)`); `string insert`
  negative-index W232.
- Stale caller cache on callee upvar change; dependency-fingerprint
  control-flow-head coverage.
- Dynamic array-read place (UNKNOWN overlap); `is_pure_var_ref` concatenation /
  W301 token-count.
