# Python → Rust parity audit (2026-06-22)

> A focused follow-up to the [workspace deep review](workspace-deep-review-2026-06-22.md):
> does the Rust rewrite faithfully reproduce (or improve on) the Python
> implementation, and is anything missing? Covers the **command registry**
> (every command and property), **diagnostics** (E/W/IRULE/… codes),
> **optimisations** (O-codes), and **compiler passes / analyses / LSP features**.
> Reviewed on branch `claude/exciting-planck-q7rj94`. Review-only — no source
> changed.
>
> **Method.** The committed registry-parity tooling
> (`scripts/registry-audit/{run_all.sh,dump_python.py,compare.py}`) and the
> 136 KB audit doc [`../../../rust-rewrite-registries.md`](../../../rust-rewrite-registries.md)
> are **stale** — that tooling was deleted in the rebase `94858698` and the doc
> is a snapshot from before the registry grew (it records `tcl` at 126 commands;
> it is now 233). So this audit re-derives parity **freshly** from the current
> trees: it dumps both registries (the surviving Rust `dump_specs` example + a
> rebuilt Python dumper), diffs names and per-property presence per dialect
> group, drives the live `tcl`/`f5` CLIs of *both* backends, and reproduces every
> headline claim against running code.

## Verdict

**Registry parity is, today, essentially complete and the data is good** — far
better than the stale audit doc implies. Across all 14 dialect groups the Rust
front-end is missing **exactly one** baseline command (`ledit`), every other
command is present, and the per-property data (arity, subcommands, options,
side-effects, taint, `event_requires`, arg-roles, return-types, traits, hover)
is at parity or richer on the Rust side. The meta-registries (events, profiles,
protocol namespaces, BIG-IP objects) match.

The real story is **the safety net, not the data**: the machinery that *proves*
Python↔Rust registry parity was deleted, and the remaining checks have a blind
spot that lets the one real gap (`ledit`) — and any future drift — slip through
unguarded. The diagnostics / optimisation / feature parity sections below carry
the higher-impact functional gaps.

---

## 1. Command registry — commands and properties

### 1.1 Command-name parity (per dialect group, current)

Fresh diff of the Python spec factories vs the Rust `dump_specs` for all 13
groups:

| group | Python | Rust | missing in Rust | Rust-extra |
|---|--:|--:|---|---|
| tcl | 228 | 233 | **`ledit`** | `auto_load_index`, `tcl::mathop` (ensemble), `tclLog`, `tclPkgSetup`, `tclPkgUnknown`, `disabled_in_irules` (marker) |
| stdlib | 225 | 225 | — | — |
| tcllib | 206 | 206 | — | — |
| irules | 1015 | 1015 | — | — |
| iapps | 49 | 49 | — | — |
| tk | 55 | 56 | — | +1 |
| expect | 35 | 35 | — | — |
| sdc-base / synopsys / cadence / xilinx / quartus / mentor | = | = | — | — |

The stale audit's "§1 tcl NAMES DIFFER (104 missing)" and "§2 irules DATA GAPS
(severe)" are **resolved**: the `tcl::mathop` ensemble, the `auto_*` family, the
named library commands, and the `regexp::quote` `::`-spelling were all restored
(both `regexp::quote` and `regex::quote` now resolve in Rust — the old `::`↔`_`
mismatch is gone). The Rust-extra entries are internal library procs or newer
Tcl 9.0 commands; the `tcl::mathop` ensemble is a deliberate modelling choice.

**The single real gap — `ledit`** (verified end-to-end):

- `ledit` is a Tcl 9.0 list-edit command, registered in Python (`tcl9.0`,
  `Arity(min=3, max=∞)`) and present in the committed baseline
  (`tests/baselines/registry/commands.csv:2293` → `tcl9.0,ledit,3,,0,0`), but
  **absent from the Rust registry** (the Rust `tcl registry-dump` has 703
  resolvable commands; `ledit` is not among them).
- **Impact, reproduced.** `tcl diag --dialect tcl9.0` on `ledit L 1 2 X Y`:
  - Python: no command-level diagnostic (it knows `ledit`).
  - Rust: **`W123 Unknown command 'ledit'; did you mean 'exit'?`** — a
    false-positive plus a misleading suggestion on valid Tcl 9.0 code, and
    `ledit`'s variable read/write of `L` is not modelled (no arity check, no
    completion, no hover).
- This is the only command-presence divergence in the whole registry.

### 1.2 Property completeness (per command)

Aggregated per group, counting commands that carry each property, Python vs Rust.
After accounting for two **modelling differences** that make a naïve count
misleading, **every property is at parity or richer on the Rust side**:

- **`side_effects`** — Python stores side-effect hints per `FormSpec`; Rust
  flattens them onto the spec. A per-form count understates Python and overstates
  nothing — the real data is present on both (irules 1002 ≈ 1002, restored).
- **`lowering`** — Python dispatches lowering via `compiler/codegen/` modules and
  does *not* stamp a hook id on the spec; Rust *does* stamp `lowering_hook` ids
  (tcl 23, irules 2). This is the documented "§5 modelling diff", not a Rust gap.
- **`event_requires`** — the one row that *looked* like a gap (Python 1011 vs Rust
  448) is an **artifact**: 1011 Python iRules commands carry an `EventRequires`
  object, but **only 448 have a non-empty field** (the other 563 are empty
  defaults). Rust carries `event_requires` for exactly those 448. **At parity.**

The only residual deltas are `ledit` itself (tcl `forms` 213/211, `arg_types`
22/21 — `ledit` + a `::tcl::unsupported::corotype` `::`-spelling variant) and
tk's one extra command. Nothing else is dropped.

### 1.3 Flags / traits coverage

Python models many capabilities as individual `CommandSpec` booleans
(`creates_dynamic_barrier`, `evaluates_code`, `opens_channel`, `byte_compiled`,
`not_proc_factory`, `frameless_runtime`, `diagram_action`,
`warn_without_terminator`, …); Rust consolidates target-neutral properties into
a `Traits` bitfield plus typed fields. Backend-specific emission flags are
intentionally excluded: emission is a property of a selected backend region,
not a Tcl command. The remaining Python flags have a Rust home — the obvious
ones map 1:1 to `Traits` bits (`CREATES_BARRIER`, `EVALUATES_CODE`,
`OPENS_CHANNEL`, …), and the rest resolve to `command_snapshot.rs` / `registry.rs`
fields and query methods (`is_byte_compiled`, `is_not_proc_factory`,
`is_diagram_action`, …). Rust is additionally **richer**: it adds `Traits` bits
Python expresses elsewhere (`DEFINES_PROCEDURE`, `DESTROYS_VARIABLE`,
`READS_BEFORE_WRITE`, `CREATES_SCOPE_ALIAS`, `RETURNS_PATH`,
`PRODUCES_CANONICAL_LIST`, …). No missing flag found.

### 1.4 Meta-registries

| registry | Python | Rust / baseline | status |
|---|--:|--:|---|
| iRule events (`EVENT_PROPS`) | 176 | 176 (events.csv) | ✓ (9/9 props verified by the contract test) |
| F5 profiles (`PROFILE_SPECS`) | 65 | 65 (profiles.csv) | ✓ |
| protocol namespaces | 113 | 113 | ✓ |
| BIG-IP objects (`OBJECT_KIND_SPECS`) | 798 | 798 (objects.csv) | ✓ (reconciled; the audit's "992" was a pre-trim number) |

### 1.5 Safety-net finding (the real issue)

The data is good, but **nothing currently proves Rust↔Python registry parity**:

- The full-property comparison tooling (`dump_python.py` + `compare.py` +
  `run_all.sh`) that originally found the "severe data gaps" was **deleted in the
  rebase `94858698`**; only the one-way *generators*
  (`gen_bigip_rust.py`, `gen_event_descriptions.py`,
  `reconcile_irules_dialects.py`) survive. The 136 KB audit doc is a stale
  snapshot from that commit.
- The Python registry presence test
  (`tests/registry_contract/test_registry_presence.py::test_command_dump_matches_csv`)
  asserts `dump == csv` — but `run_tcl_json` shells out to the **Python** CLI
  (`sys.executable -m tooling.tcl.main`, `_harness.py:71`), so it checks
  *Python == baseline*, **never Rust == baseline**.
- The Rust-side `rust/tcl-cli/tests/cli_parity.rs` test
  (`registry_dump_faithful_subset_matches_python`) compares only a **"faithful
  subset"** against a Python-captured golden — it asserts the commands that *are*
  present match, not that the set is **complete**.

Net: **the Rust registry has no completeness gate against the Python source of
truth.** `ledit` is the live proof — it is in the baseline, in Python, and in no
Rust test's assertion, so its absence ships undetected. **Recommendation:** add a
Rust-front-end variant of `test_command_dump_matches_csv` (drive the native
`tcl registry-dump` and assert `== commands.csv`), or restore a trimmed
`compare.py` into `test-slow`. This is a one-test fix that would have caught
`ledit` and will catch the next drift.

---

## 2. Optimisation codes (O100–O130)

**Presence parity is complete and profile-gating is exact.** All 31 Python
`@opt` codes are covered: the Rust optimiser *pass* emits 28 directly, and the
other three (O105, O106 via the GVN diagnostics path; O111 via the diagnostics
layer) are emitted the same way on *both* backends — so there are **zero missing
optimiser transforms**. The per-code `opt_category` map (`shared/codes.py` vs
`optimiser/profiles.rs::OPT_CATEGORIES`) matches with **0 mismatches** —
readability/standard/full/aggressive/off gate identical code sets.

The parity gap is entirely **correctness**: **four Rust-only miscompiles**, one
shared latent bug, and one cosmetic label divergence. Each was verified
end-to-end (built reproducers, `tclsh` oracle).

| Code | Defect | Scope | Status |
|---|---|---|---|
| **O122** | Braced `lassign {…}` instead of `[list …]` → params get literal strings; with `[...]` args, a **hard `tclsh` runtime error** (`list element in braces followed by "]"`) | **Rust-only** | reproduced |
| **O109 / O126** | Dead-store elimination has no `::`-qualified-name guard (`emit_dead_stores_and_unused`, `elimination.rs:482`); deletes a `set ::g V` read by another proc via `global g`. Rust's *own* coupling path (`manager.rs:350`) and analyser (`diagnostics.rs:5936`) guard `::` — only this pass omits it | **Rust-only** | reproduced |
| **O129** | Builtin const-fold trust gate exists (`propagation.rs:1465`) but `command_mutations` is populated only in the test-path `optimise_raw`, never in production `optimise_unit` (`manager.rs:70`) → folds `[string length foo]` after `rename string {}` | **Rust-only** | reproduced |
| O103 | Folds a pure proc that returns a constant on one path and falls off the end otherwise (summary path inspects only explicit `Return`s) → `[g 0]` folds to the constant instead of `""` | **shared** (Python folds identically — not a regression) | reproduced both sides |
| O105 | The `"…$x…"` const-propagation rewrite is tagged **O100** by Rust (whole-string span) vs **O105** (token span) by Python — behaviourally equivalent, cosmetic | label/span | — |

These four Rust-only miscompiles are the same family as the workspace review's
*Cross-cutting theme D*, now pinned to exact codes with one-to-few-line fixes.
**The parity doc `docs/rust-optimiser-parity.md` is stale**: its "verified" claims
for the `::`-global guard (O109/O126) and the O129 trust gate describe logic that
exists but is **not wired into the production path**.

## 3. Compiler passes, analyses, and LSP features

High parity overall — and **both tracking docs are stale in opposite
directions**: `compiler-pipeline-parity.md` *overstates* gaps (≥10 of its
"missing" rows are now implemented — IRULE1001 wired, snit + TclOO body-walks
present, O128/O130 emitted, SSA complexity-guard present, …), while
`rust-rewrite.md`'s "landed ✅" claims are accurate *except* they don't flag that
some landed code is not wired into a production pipeline. The genuine remaining
gaps, ranked by impact:

1. **The general proc inliner is built, tested, and unwired (highest-impact
   compiler gap).** `inlining/mod.rs::inline_module` (~the full splice inliner) has
   **zero production callers** and there is **no `PassId::Inline`** in the
   optimiser pipeline (`optimiser/mod.rs` `PassId` enum) — only tests call it.
   Worse, the **`var_escape` driver + `pure_leaf` analysis** (`var_escape/api.rs`)
   is invoked *only* from that unwired inliner, so var-escape produces nothing in
   production either. Two substantial, fully-ported subsystems are dead weight in
   the live pipeline. (Soundness-safe — "dead in pipeline", not a miscompile.)
2. **`ProcArgTrait::DynamicNameLocal` is missing** (`analyser/types.rs:158` has 6
   variants, no `DynamicNameLocal`). Python uses it 10+ times
   (`compiler/proc_arg_traits.py:140…`) to refine `VAR_READ` and suppress
   caller-side **W211 / W214 / dead-store false positives** on `set $p` / `scan` /
   `lassign` / `regsub`. Its absence re-opens the PR #498/#499 false positives —
   a single missing property that degrades several diagnostics. **✅ Closed
   2026-06-25:** the `DynamicNameLocal` variant landed in `analyser/types.rs`
   with `param_traits.rs` emitting it from the `VarWrite`/`VarRead` /
   variadic-var-write / `regsub` arms, restoring the suppression.
3. **Type hierarchy returns nothing** (`tcl-lsp-server/src/lib.rs:3878`). The core
   provider `type_hierarchy.rs` is ready, but the handler is a no-op — **blocked by
   tower-lsp 0.20** not exposing the methods. No supertype/subtype navigation for
   TclOO/snit in any editor.
4. **The BIG-IP LSP surface is partial.** Rust dispatches BIG-IP *documentSymbols*
   by dialect, but code-actions / references / document-links fall back to
   generic-Tcl (Python has `_bigip_code_actions/_refs/_links`), and ~6 BIG-IP
   `execute_command` verbs (`extractRule`, `listRules`, `extractLinkedObjects`,
   `bigipCleanup`, `renamePartition`, `writeRuleBack`) plus a few tooling verbs
   have no Rust arm (`lib.rs:4664`). Degrades F5-config editing/navigation.
   **🟢 Mostly closed 2026-06-25:** the three provider fallbacks are replaced —
   `tcl-bigip::refs` (find-references), `tcl-bigip::links` (document-links), and
   a BIG-IP code-action provider — and "Generate docstring" reached parity with
   the Python `generate_stub`; the `execute_command` verbs that delegate to
   `tcl-bigip-query` (`renamePartition`/`writeRuleBack`/…) remain the residual.
5. **`tcl-wasm` codegen is eval-fallback and lacks `--link`.** Folded into `tcl
   compwasm`; the emitter is the Phase-1 eval-fallback (~1 KB of IR vs Python's
   ~13-module WASM package) and there is no Binaryen `wasm-merge` bundling for a
   standalone self-contained `.wasm`. The WASM IR-rewriting passes (`passes/{dce,
   gvn,interp_boundaries}.py`) are unported. RT-WASM track 🟡, out of the LSP path.
6. **Lowering precision/soundness residuals** — dynamic `uplevel $body` lowers to
   a generic `Statement::Call` rather than a frame-crossing `Barrier`
   (`lowering/mod.rs:1397`), and `namespace eval`'s static body is lowered then
   **discarded** before the `Barrier` (`lowering/mod.rs:1617`, losing
   `IRBlock.namespace` for unqualified-name resolution). Narrow but real.

**Otherwise faithful or better.** Every core LSP method is implemented and wired
(hover, completion, the four go-tos, references, highlight, symbols, folding,
formatting, selection range, signature help, semantic tokens full/delta/range,
code action, code lens + resolve, rename + prepare, will/did-rename, call
hierarchy, linked editing, document link, inlay hint, pull + push diagnostics,
will-save-wait-until, workspace symbols, config + workspace-folder + watched-file
notifications) — contradicting any "stubbed" assumption; type hierarchy is the
one missing feature. Every Python analysis (taint T100–T106, CFG/SSA, GVN, SCCP,
intervals, memory-SSA with upvar merge, type lattice, shimmer, side-effects, MRO)
has a Rust counterpart; the analyser semantic model, proc lookup, class hierarchy,
and signature scan are all ported. All `tcl` (~26) and `f5-query` (~27) CLI verbs
are present.

## 4. Diagnostic codes (E / W / IRULE / BIGIP / IAPP / T / S / XC)

**Presence parity is complete.** A fresh set-diff of every registered/emitted
code string (Python 155 distinct vs Rust 147) leaves **no genuinely-missing
analyser diagnostic** — every apparent Python-only code is a false positive on
inspection, and Rust is richer by one (`S110`, the byte-array-corruption check):

| candidate | verdict |
|---|---|
| E000 | not a diagnostic — the sentinel fallback in `server/commands.py:577` (`… else "E000"`) |
| E402 | non-diagnostic noise (matches the ruff code / unrelated strings) |
| IRULE2102 | **retired** — "subsumed by O105 (GVN/CSE)" (`server/features/diagnostics.py:76`, `irules_flow.py:27`) |
| S603 | WASM-codegen security code (`codegen/wasm/_bundle.py`) — in the unported RT-WASM path, not an analyser/LSP diagnostic |
| W130–W134 | **tclpkg** package-manager codes (`shared/codes.py:301`), absent from `server/`/`analyser/`. The Rust `tcl-pkg` implements the underlying checks (`errors.rs` `Category::Integrity`, lockfile/CAS, safe-mode; its comment requires output to "match the Python output") — they are simply not tagged with these `W13x` strings |

So the full E-code / W-code / IRULE / BIGIP / IAPP / T (taint T100–T106) families
are all present on the Rust side, and the editor catalogues match (generated from
the same registry).

**But there are ~18 user-visible SEVERITY-tier divergences** (verified against the
live Rust LSP) — the most impactful diagnostic finding, because they change the
*colour of the squiggle* a user sees, not whether it appears. The root cause is
mechanical: `rust/tcl-compiler/src/compiler_checks.rs` `from_taint` /
`from_irules_check` / `from_path_concat` **hardcode a single severity** instead of
mirroring Python's per-code severity maps (`_TAINT_SEVERITY`,
`_IRULES_FLOW_SEVERITY` in `server/features/diagnostics.py:54`) — and the *sibling*
`from_shimmer` already mirrors its Python map correctly (with a comment pointing
at it), which is what marks the others as a bug rather than a design choice.

- **The entire taint family is escalated to ERROR.** `from_taint` returns
  `Severity::Error` for **T100–T106, IRULE3001/3002/3003/3004/3101/3103, and
  W313** (`compiler_checks.rs` `from_taint`); Python emits these as **Warning**
  (most) or **Information** (T106, IRULE3103). On the live Rust LSP, `T101` and
  `W313` publish as **red errors** where Python publishes yellow warnings.
- **iRules connection-state severity inverted (Rust *under*-escalates):**
  `IRULE1007` / `IRULE1008` (collect-without-release / release-without-collect)
  are **Error** in Python, **Warning** in Rust (`from_irules_check` hardcodes
  `Warning`). `IRULE4004` drops **Information → Warning**.
- **`W201` path-concat** is **Hint** in Python, **Warning** in Rust
  (`from_path_concat`). **`I230` / `I231`** are **Information** in Python, **Hint**
  in Rust.

These all share one fix: give `from_taint`/`from_irules_check`/`from_path_concat`
the per-code severity table `from_shimmer` already has.

**Precision divergences (recall/false-positives), from the pass/analysis audit:**

- **W211 / W214 / dead-store false positives** — the missing
  `ProcArgTrait::DynamicNameLocal` (§3) makes Rust over-report unused/never-read
  on `set $p` / `scan` / `lassign` / `regsub` call-by-name patterns (regresses
  PR #498/#499).
- **W123 false positive on `ledit`** (§1) — the one missing command.
- **W001 range** — for `string bogus x`, Python highlights the bad subcommand
  `bogus`; the Rust LSP highlights the whole `string bogus` span.
- **W233 (divide-by-zero)** — Rust emits on SCCP-constant `[0,0]` divisors only;
  the interval path (`interval_bounds.rs` `find_divide_by_zero`) exists but isn't
  the production emitter, so interval-proven zero divisors are missed.
- **IRULE1201 / 1202 / 5002 / 5004** — linear-scan MVPs vs Python's
  path-sensitive checks (a few quick-fixes dropped, tracked C44); **IRULE4002 /
  4004** are literal-only. Lower recall, not absent.

Net: the diagnostic *surface* is faithful (0 missing codes, Rust +1), but the
**severity tiers are systematically wrong for the taint/flow/path families** —
the single most user-visible parity defect — plus the `DynamicNameLocal` false
positives and a handful of recall gaps.

> **Methodology note (important for anyone re-checking):** a bare `tcl diag`
> runs only the lighter `Analyser::analyse` pass, so it does **not** show
> taint / iRules-flow / shimmer / optimiser codes — those reach the editor only
> via the LSP path (`Analyser::analyse` ∪ `run_all_checks` ∪ `source_style`,
> `tcl-lsp-db/src/lib.rs:643`). Parity must be checked through the LSP, not
> `tcl diag` alone, or the taint/flow families look falsely "missing".

---

## Consolidated parity gaps and recommendations

| # | Gap | Kind | Impact | Fix size |
|---|---|---|---|---|
| 1 | `ledit` missing + no Rust registry-completeness gate | registry | false `W123` on Tcl 9.0; future drift unguarded | small (add command + a Rust-vs-CSV test) |
| 2 | O122 braced `lassign` | optimiser miscompile (Rust-only) | `optimiseDocument` → hard `tclsh` error / wrong values on multi-arg tail recursion | 1 line (`[list …]`) |
| 3 | O109 / O126 delete `::`-global writes | optimiser miscompile (Rust-only) | deletes cross-proc global writes → program breaks | small (`::` guard in `emit_dead_stores_and_unused`) |
| 4 | O129 trust gate unwired | optimiser miscompile (Rust-only) | folds renamed/shadowed builtins | 1 line (populate `command_mutations` in `optimise_unit`) |
| 5 | Proc inliner + var_escape unwired | dead capability | two ported subsystems produce nothing; weaker analysis | medium (wire a `PassId::Inline`) |
| 6 | `ProcArgTrait::DynamicNameLocal` missing — ✅ **landed 2026-06-25** (variant + `param_traits` uses; suppresses the caller-side false positives) | analyser precision | W211/W214/dead-store false positives (PR #498/#499) | small (add variant + uses) |
| 6b | Taint/flow/path **severity tiers** hardcoded | diagnostic UX (Rust-only) | ~18 codes: whole taint family shows **red ERROR** vs Python's Warning/Info; IRULE1007/1008 under-escalated | small (per-code map in `compiler_checks.rs`, like `from_shimmer`) |
| 7 | Type hierarchy no-op | LSP feature | no supertype/subtype nav (tower-lsp 0.20 blocked) | external (upgrade tower-lsp) |
| 8 | BIG-IP code-actions / refs / links + ~6 execute-command verbs — 🟢 **refs/links/code-actions landed 2026-06-25** (`tcl-bigip::{refs,links}` + code-action provider; "Generate docstring" parity); residual = the verbs that delegate to `tcl-bigip-query` (`renamePartition`/`writeRuleBack`/…) | LSP feature | degraded F5-config editing | medium |
| 9 | `tcl-wasm` eval-fallback + no `--link` | codegen | no real/standalone WASM (RT-WASM 🟡) | large (ongoing track) |
| 10 | Lowering: dynamic `uplevel $body`→`Call`; `namespace eval` body discarded | lowering precision/soundness | narrow analysis regressions | small–medium |
| 11 | O103 fall-off-end fold | **shared** miscompile (both backends) | folds conditionally-returning proc to a constant | small (require all-exits-return) |
| 12 | Stale tracking docs (registry audit, optimiser-parity, pipeline-parity) — 🟢 **refreshed 2026-06-25** (`rust-rewrite.md`, `rust-rewrite-test-audit.md`, and this audit's gaps #6/#8 annotated) | process | misstate the position in both directions | regenerate/retire |

**Recommendations, in priority order:**

1. **Restore a registry-completeness gate** — a Rust-front-end variant of
   `test_command_dump_matches_csv` (drive `tcl registry-dump` from the native
   binary, assert `== commands.csv`). One test; catches `ledit` and all future
   drift. Add `ledit`.
2. **Fix the four optimiser correctness regressions** (gaps 2–4 + the shared
   O103) — all small, all in `optimiser/{tail_call,elimination,manager}.rs` — and
   add before/after *execution*-equivalence cases to the differential gate so a
   semantic-preservation regression in any O-code is caught (the gate compares
   disassembly/folds today, not rewrite equivalence).
3. **Wire the inliner** (gap 5) so the ported inliner + var_escape actually run,
   or explicitly document them as analysis-only.
4. **Fix the diagnostic-fidelity gaps** — give `from_taint`/`from_irules_check`/
   `from_path_concat` a per-code severity table (copy the `from_shimmer` pattern)
   so the taint/flow/path families stop showing as red ERRORs (gap 6b), and add
   `DynamicNameLocal` (gap 6) to retire the caller-side false positives. Both are
   small and high-visibility.
5. **Regenerate or retire the three stale parity docs** (gap 12); they currently
   mislead in both directions (the registry audit and pipeline-parity overstate
   gaps; the optimiser-parity doc's "verified" guards are unwired).

The encouraging headline: after a fresh, code-grounded re-derivation, the Rust
rewrite is **at or above Python parity almost everywhere** — the registry,
diagnostics, optimisation, analysis, and LSP-feature *surfaces* are complete or
richer, and the code-identity sets match. The real residue is a small,
well-bounded set: four optimiser correctness regressions, the systematic
taint/flow **severity-tier** mismatch (cosmetic-but-jarring), two inert
subsystems (inliner / var-escape), one missing analyser trait, the
tower-lsp-blocked type hierarchy, and the in-progress WASM and BIG-IP-LSP
tracks — every one of which is concretely actionable, most with a one-to-few-line
fix.

## Related

- [`workspace-deep-review-2026-06-22.md`](workspace-deep-review-2026-06-22.md) —
  the workspace deep review (architecture/quality/correctness; *Cross-cutting
  theme D* is the optimiser-miscompile family pinned to O-codes here).
- [`../../../rust-rewrite-registries.md`](../../../rust-rewrite-registries.md),
  [`../../rust-optimiser-parity.md`](../../rust-optimiser-parity.md),
  [`compiler-pipeline-parity.md`](compiler-pipeline-parity.md) — the **stale**
  tracking docs this audit supersedes (recommend regenerating from current code).
