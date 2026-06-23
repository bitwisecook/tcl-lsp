# Deep review — SRV-INCREMENTAL (#692, per-edit incremental pipeline + cross-file diagnostics)

> A focused correctness / style / integration review of commit `d40b2f0d`
> ("SRV-INCREMENTAL: per-edit incremental pipeline + cross-file diagnostics", #692),
> +4,957 / −479 across 33 files. Reviewed on branch `claude/exciting-planck-q7rj94`
> with `origin/rust` merged in; claims anchored file:line. Design spec:
> [`../srv-incremental/README.md`](../srv-incremental/README.md).
>
> What shipped: **Task 1** persisted incremental `LineIndex::apply_edit`; **Task 2a**
> per-function check memo `function_checks` (`body_offset`-rebased); **Task 2b**
> incremental interprocedural taint (`proc_taint_solve` / `proc_summary_cascade` +
> dirty-set worklist + dup-build); **Task 6** cross-file W123 suppression + E002/E003
> arity across procs/classes/aliases/ensembles, behind the `xcDiagnostics` opt-in
> (off by default). Tasks 3/4/5/7 are deferred or blocked (documented).

## Verdict

**This is high-quality, correctness-conscious work, and it lands clean.** Across three
independent adversarial reviews plus a from-scratch test run, **no correctness
regression was found**; the off-by-default guarantee is airtight; the reuse discipline
is excellent; and the named gates pass — including the three `#[ignore]`'d **corpus
differentials I ran here** (`compiler_check`/`file_analysis`/`file_decls` memo ==
uncached, byte-identical over ~1,500 tcllib + Tcl 8.4–9.0 files; 54 s / 17 s / 36 s,
all green) and the 38 fast gates (firewalls, `incremental == fresh`, the 5,000-edit
`apply_edit` fuzz, the graphops soundness regression). The design doc is exceptional —
measurement-tagged, honest about reverted attempts and open risks.

The findings are **refinements, not blockers**, and the two that matter are
*performance/precision* claims that are overstated, plus one *style* regression:

1. **(perf) The `project_command_arities` firewall leaks** — it reads `item_sigs`,
   which carries an absolute `name_span`, so a body edit to any non-last proc *does*
   recompute the project arity table (empirically `1×`, not the doc's claimed `0×`).
   Contained downstream by salsa output-equality cutoff, so cross-file *diagnostics*
   don't fan out — but the O(project-decls) table rebuild is real, and the firewall
   test missed it because its fixtures are single-proc-per-file.
2. **(precision) The push path doesn't refresh open→open cross-file edits** — editing
   an open *definer* B does not reschedule an open *caller* A's published diagnostics
   (`did_change` lacks the `reschedule_xc_open_documents` call the watched-file/folder
   paths have), so A shows stale cross-file state until A is touched. Off by default and
   "no-worse-than-today," but inconsistent.
3. **(style) God-code growth** — the +1,365 / +680 lines went *entirely* into the
   existing `tcl-lsp-db/src/lib.rs` (now 2,694 lines, still one file) and the 8.8k-line
   server `lib.rs`; the cohesive cross-file (~260 LOC) and 2b-memo clusters should be
   `cross_file.rs` / `taint_memo.rs` modules. Well-factored at the function level, not
   the module level.

Plus a latent fragility (the 2a `(0,0)` sentinel value-collision — doesn't fire today)
and the test-net gaps the doc itself flags (no random-edit checks fuzzer; the soundness
guard is debug-only; the `apply_edit` fuzz is ASCII-only). Details and a prioritised
fix list below.

## 1. Correctness — the per-function & interprocedural-taint memos (Task 2a / 2b)

**Verdict: sound in practice; verified byte-identical to the uncached path over the
~1,500-file corpus.** One *latent* (non-firing) unsoundness and a real residual-risk
surface from the test net.

- **2a `body_offset` rebasing is correct** (`tcl-lsp-db/src/lib.rs:1101-1138`). Every
  offset-0 diagnostic from `function_checks` → `function_nontaint_checks`
  (`compiler_checks.rs:275`) carries exactly one span and `fixes: []` /
  `replacement: None`, and the rebase adds `body_offset` once — numerically identical to
  the whole-module `rebase_function_unit`/`abs_span` path.
- **LATENT (fix-worthy): the `(0,0)` sentinel is a value-collision, not a None-test**
  (`lib.rs:1118`). A `None`-span constant branch lowers to `(0,0)` and is correctly
  skipped; but if a *real* branch span were ever `Span::new(0,0)` (zero-width at body
  offset 0), the whole-module path would shift it to `(body_offset, body_offset)` while
  the memo leaves it `(0,0)` → divergent diagnostic. Traced both producers
  (`sccp.rs:589-604`, `:703-710`): `cb.span` is always a genuine condition span
  (`:674` requires `Some`), so it never fires — confirmed by the corpus differential
  (guard never trips). It is nonetheless fragile to a future lowering change. **Fix:**
  thread `Option<Span>` / a non-collidable sentinel out of `function_checks` instead of
  testing `== (0,0)`.
- **2b dup-build is sound.** `proc_taint_solve` re-derives the `'db`-interned per-proc
  keys via `build_unit_with_keys` (forced by salsa's `'static` return constraint — a
  real, documented limitation), and its per-proc `function_lattice`/`taint_cascade`
  demands hit the **same memos** the shared `compilation_unit` build populated (same
  `cfg_key`), so it pays only ~28 ms whole-file reassembly, not a re-lower, and cannot
  diverge.
- **The graphops soundness fix is correct** (`lib.rs:989-996`). It seeds **every** proc
  in the resolution domain (`deps_key.known_procs`, the complete module name set) to the
  clean ⊥ `ProcTaintSummary::untainted`, then overlays the reachable real summaries —
  closing the over-taint hole where an *absent* (vs present-clean) resolved callee made
  `propagate_taints` fall through `summaries.get(&target)?` (`taint.rs:651`) to a
  conservative bare-argument join. No path leaves a proc absent. Pinned by
  `compiler_check_memo_matches_uncached_graphops` (debug guard live).
- **The worklist terminates** (`taint_interproc.rs:574-657`): a monotone product-lattice
  LUB of finite height, re-queueing callers only on change — mutual recursion converges
  via the worklist (salsa memoises only the per-proc infer, so no `cycle_fn`/panic
  risk).
- **Salsa keys are complete** (`FnLatticeKey`, `TaintSummaryKey`, `SummaryDepsKey`): a
  callee summary change re-keys exactly the callers that reach it; dialect is in every
  key; the registry is a deterministic dialect-keyed side-table (no mutable input, no
  staleness); offset-variant data appears only at aggregation, never interned. No
  missing-dependency staleness found.

**Residual risk (honest, and the doc admits it):** cross-edit checks-path soundness
rests on (i) the **cold** corpus differential (rebuilds fresh per file — cannot see a
*missed invalidation*), (ii) **one hand-written** 4-edit / 3-proc sequence
(`taint_cascade_matches_uncached_under_edits`, `lib.rs:1645`), and (iii) the
**debug-only** fixpoint guard (`#[cfg(debug_assertions)]`, `taint_interproc.rs:633` —
release ships unguarded). The class of bug that would slip: a cross-edit staleness on a
proc-graph shape not in the 3-proc fixture (deep closures, mutual recursion,
command-substitution callees, variadic-arity edges, class/method interactions), in
release. The `(0,0)` collision above is a second member of that class. The missing
**random-edit `compiler_check_diagnostics` vs fresh-whole-unit fuzzer** is the single
highest-value gate to add — the doc's own principle ("correctness must rest on
incremental==fresh fuzzing, never on the assumption that an edit is local") points
straight at it.

## 2. Correctness — the cross-file cascade (Task 6)

**Verdict: off-by-default airtight; arity conservatism fully correct; one perf-firewall
leak and one open→open staleness gap.**

- **Off-by-default is proven safe.** `xcDiagnostics ∈ DEFAULT_OFF` (`lib.rs:778`),
  resolved `unwrap_or(false)`. Both demand sites — push `run_diagnostics_core`
  (`lib.rs:432-437`) and pull `full_diagnostics_for` (`lib.rs:2880-2922`) — gate the
  `project_diagnostics` / `apply_cross_file_resolution` calls behind the toggle; the
  `Project` *input* is maintained unconditionally but that is input bookkeeping, not
  query execution. **No cross-file query runs when off ⇒ genuinely zero behaviour
  change.**
- **LEAD FINDING (perf, empirically verified): the `project_command_arities` firewall
  leaks.** The doc/verification-table claim "a body edit re-runs the arity table 0×" is
  **false for any multi-proc file.** `ItemSig` (`analyser/item_tree.rs:88-99`) carries an
  absolute `name_span: Span` and derives `PartialEq`; `project_command_arities` iterates
  the full `item_sigs` (`lib.rs:289`), not the span-free `file_decls`. A probe over a
  2-proc file confirmed: a body edit to the **first** proc shifts the second's
  `name_span` → `item_sigs` **and** `project_command_arities` each re-execute `1×` (only
  a *last*-proc edit gives the claimed `0×`). So a keystroke in an 80-proc utility
  rebuilds the whole project's O(project-decls) arity table. **It is contained
  downstream:** the table recomputes to an *equal* value, so salsa output-equality
  cutoff keeps other files' `project_diagnostics` at `0×` — the end-to-end "don't wake
  the workspace" guarantee survives, but via output cutoff, not the input firewall the
  doc describes. The firewall *tests* (`project_command_arities_firewall` `lib.rs:1874`,
  `project_proc_names_firewall` `:1812`) use **single-proc-per-file** fixtures, which is
  exactly why the leak wasn't caught. (`project_proc_names`, which reads a span-free
  `BTreeSet<String>`, genuinely backdates — firewall real there.) **Fix:** key the arity
  table on a span-free projection of `item_sigs` (name + params/arity only), or make
  `project_command_arities` read `file_decls` + a span-free arity table.
- **REAL STALENESS GAP (precision): open→open push edits aren't refreshed.**
  `reschedule_xc_open_documents` (`lib.rs:1482`) re-publishes *other* open xc-documents
  after a cross-file domain change, and is called from `did_change_watched_files`
  (`:3482`) and `did_change_workspace_folders` (`:3439`) — but **not from `did_change`**
  (`:3308` schedules only the edited doc). So with caller A and definer B both open
  buffers, editing B's signature does not refresh A's *published* push diagnostics until
  A is itself touched. The salsa edge is correct (A's `project_diagnostics` *is*
  dirtied); the **trigger** is missing. Off-by-default and "no-worse-than-today," but
  inconsistent with the explicit reschedule the watched-file/folder paths got. **Fix:**
  call `reschedule_xc_open_documents` from `did_change` when the edited doc's
  `file_decls`/signature set changed (gate on xc + a decl-change check to avoid waking
  on every keystroke).
- **Arity conservatism is fully correct** — every documented guard is present and
  tested: `{*}`-expansion → `argc=None` skip (`walker.rs:67`, `commands.rs:279`,
  `lib.rs:412`); non-proc resolution → no arity (`lib.rs:301-305,411`); mixed
  proc/non-proc tail → arities dropped (`:309-317`); tail collision → `min(mins)` /
  `max(maxes)` so only counts outside *every* candidate flag (`:338-345`); trailing
  `args` → unbounded `max`. No off-by-one (verified across (2,2)/(1,2)/(1,∞)/(0,0) by
  `cross_file_arity_edge_cases`). Arity is W123-toggle-independent and honours the
  disabled-code filter.
- **`incremental == fresh` for cross-file is real and strong:**
  `project_diagnostics_incremental_matches_fresh_under_edits` (60 edits to the definer)
  and `..._both_files_edited` (80 edits to both) byte-compare against a from-scratch
  `TclDatabase` rebuild. Rename/delete invalidation is correct on the salsa edge
  (disk path proven by `cross_file_resolves_against_disk_backed_file` /
  `_drops_disk_backed_file_when_gone`). Not covered (doc admits): corpus-scale
  `source`/`package require` graphs and per-symbol precision — the heuristic-edge risk
  stays open.

## 3. The incremental `LineIndex` (Task 1) + offset-type safety

**Verdict: correct — splice math empirically verified, byte/UTF-16 units consistent.**

- **`apply_edit` splice math is sound** (`tcl-lexer/src/line_index.rs:97-125`). An
  independent reimplementation + 12 adversarial edits (delete-across-`\n`,
  insert-`\n`-at-0, replace-`\n`-with-non-newline, edit-ending-on-a-newline, multi-`\n`
  replace, full-content replace) were **all byte-identical to a `LineIndex::new`
  rebuild.** Signed delta is computed in `i64` with a checked `u32::try_from`
  (`:120-122`) — underflow is provably impossible (region 3 only shifts starts `>
  old_end`, min result `start+1 ≥ 1`). Boundary newlines counted once; `start[0]==0`
  never lost or duplicated.
- **Byte/UTF-16 safety is handled at the one place it matters.** `apply_edit` is
  consistently byte-based, and the live caller `apply_content_change_indexed`
  (`tcl-lsp-server/src/lib.rs:5696-5722`) converts the LSP UTF-16 `(line, character)` to
  **byte** offsets via `offset_at_utf16` *first*, then feeds those identical byte offsets
  to both `apply_edit` and the `String` splice. No unit mismatch on non-ASCII edits —
  this is exactly the discipline the type-system review (`coherence-and-coverage` §2)
  recommended, applied correctly here.
- **Gate gap (low-risk): ASCII-only fuzz.** `apply_edit_matches_rebuild_under_fuzz`
  (5,000 edits) and the live-path consistency test are real and assert byte-identity to
  rebuild, but the fuzz alphabet is `'x'`/`'\n'` — no test drives `apply_edit` /
  `apply_content_change` with **non-ASCII** `new_text`. Low risk (the math is pure byte
  arithmetic and the UTF-16→byte conversion has its own surrogate-pair tests), but the
  end-to-end non-ASCII ranged-edit path is technically unfuzzed. Add a multi-byte
  codepoint to the fuzz alphabet.

## 4. Style & integration

**Verdict: excellent reuse; one clear style regression (god-code growth).**

- **Reuse is the commit's best aspect — no reimplementation.** Cross-file resolution
  reuses the pre-built-but-unwired signature firewall (`item_tree`/`item_sigs`/
  `file_decls`) exactly as designed — no new parsing. `proc_taint_solve`'s dup-build
  routes through the *same* `function_lattice`/`taint_cascade` memos. The diagnostics
  merge reuses `push_taint_and_module_checks` + `sort_diagnostics`, and
  `compiler_checks.rs` **extracts** `function_nontaint_checks` / `run_all_checks_with_solved`
  as `pub` reusable functions (with `run_all_checks` becoming a thin wrapper) rather than
  copying. `argc` / `unresolved_command_sites` are clean additive fields on the existing
  analyser collection. E002/E003 reuse the analyser's own codes (with an explicit comment
  *not* to reuse the unrelated W124).
- **God-code growth — the one style regression.** The +1,365 (`tcl-lsp-db/src/lib.rs`,
  now 2,694 lines, still the crate's only module) and +680 (server `lib.rs`, already an
  8.8k god-file) went *entirely* into the existing mega-files. The cross-file surface
  (`Project`, `project_proc_names`, `project_command_arities`, `apply_cross_file_resolution`,
  `project_diagnostics`, ~260 cohesive LOC) is a textbook `cross_file.rs`/`project.rs`
  module; the 2b cluster (`build_unit_with_keys`, `proc_summary_cascade`,
  `proc_taint_solve`, `function_checks`, the `*Key` types) a natural `taint_memo.rs`.
  Well-factored at the function level, not the module level — extract before the next
  feature compounds it.
- **Clippy: +1 net production allow, justified.** The server gains one
  `#[allow(clippy::too_many_lines)]` on a long handler; the db gains two
  `cast_possible_truncation` in fuzz/test helpers (commented). The rest are on test/
  example code with one-line rationales. Not a discipline regression.
- **Naming consistent** with the existing salsa vocabulary (`file_analysis_incremental`,
  `taint_cascade`, `function_lattice` ↔ `project_diagnostics`, `proc_taint_solve`,
  `proc_summary_cascade`). **Experiment/test hygiene clean** — `experiment-pipeline/`
  and `experiment-xfile/` declare their own `[workspace]` and aren't members (no
  main-build bloat); `gen_fixture.py` is a pure formatting change.
- **Minor lock-order inconsistency** (`did_change_watched_files` DELETED branch,
  `lib.rs:3460-3467`): takes `workspace_index` ahead of the db and doesn't hold
  `documents` across both updates, unlike the five other sites (which keep
  `documents → db → workspace_index`). Not a deadlock (no nesting; the guard drops
  before db is acquired), race window benign for non-open files — but worth aligning to
  the invariant.

## 5. Verification gates — do the tests prove what the doc claims?

Run here against the merged tree:

| Gate | Result |
|---|---|
| 38 fast unit gates (`tcl-lexer` + `tcl-lsp-db`), incl. `apply_edit` 5000-fuzz, both firewalls, `incremental==fresh`, graphops | **all pass** (33/0 in the db lib bin) |
| `compiler_check_memo_matches_uncached_over_corpus` (`#[ignore]`, ~1500 files) | **pass** (54 s, byte-identical) |
| `file_analysis_corpus` / `file_decls_corpus` (`#[ignore]`) | **pass** (17 s / 36 s) |

The named gates exist and assert what they claim. The **documented holes** (real,
ranked by exposure): (1) **no random-edit fuzzer for `compiler_check_diagnostics`** —
cross-edit checks soundness rests on the cold corpus + one hand sequence + a debug-only
guard (§1); (2) the **firewall fixtures are single-proc**, which hid the
`project_command_arities` leak (§2); (3) **`apply_edit` fuzz is ASCII-only** (§3); (4)
**no corpus-scale multi-file** differential, so the cross-file heuristic-edge risk is
unsettled (doc admits). None blocks the merge (off-by-default, corpus differentials
green); each is a gate to add before the corresponding surface is leaned on harder.

Stale-comment cleanup surfaced: `check_diagnostics_rerun_whole_file_on_body_edit`'s
name/comments predate 2a (assertion still true, but it no longer describes per-proc
breadth); the `project_proc_names` doc comment says "Not yet wired into diagnostics"
while the README says it shipped (in fact `project_command_arities` does the W123 work —
`project_proc_names` is test-only).

## 6. Recommendations

Prioritised; none is a merge blocker (the feature is off by default and the soundness
gates are green).

**P1 — close the real gaps:**
1. **Build the random-edit `compiler_check_diagnostics` vs fresh-whole-unit fuzzer**
   (§1). This is the doc's own missing gate and the highest-value soundness item — it
   would catch both a cross-edit staleness *and* the latent `(0,0)` collision, and lets
   the debug-only guard's coverage be trusted in release.
2. **Fix the `project_command_arities` firewall leak** (§2): key the arity table on a
   span-free projection of `item_sigs` so a body edit truly recomputes `0×`; add a
   **multi-proc** firewall fixture so the regression can't recur silently.
3. **Reschedule open→open push diagnostics** (§2): call `reschedule_xc_open_documents`
   from `did_change` on a signature/decl change, matching the watched-file/folder paths.

**P2 — fragility & hygiene:**
4. **Replace the `(0,0)` sentinel with `Option<Span>`** out of `function_checks` (§1) —
   remove the value-collision before a future lowering change makes it fire.
5. **Extract `cross_file.rs` / `taint_memo.rs` modules** from `tcl-lsp-db/src/lib.rs`
   (§4) before the next feature compounds the 2,694-line file.
6. **Add non-ASCII to the `apply_edit` fuzz alphabet** (§3); align the
   `did_change_watched_files` lock order (§4).

**P3 — docs:** correct the `0×`-firewall claim in the design doc's verification table to
"`0×` only on last-proc / span-free queries; `1×` on `project_command_arities` until the
span-free projection lands"; refresh the stale test/query comments (§5).
