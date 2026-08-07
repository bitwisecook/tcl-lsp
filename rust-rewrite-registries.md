<!-- markdownlint-disable MD013 MD033 -->
# Rust-rewrite registry audit

> **Historical — the audit is complete and cannot be re-run as written.** Every
> row and every entry below is ticked, and the drift check it describes no longer
> has anything to compare: the Python `core/…` registries were deleted with the
> Python engine, and `scripts/registry-audit/` went with them, so the "reproduce"
> commands are dead. Last substantive update 2026-06-19. Kept as the record that
> the port was verified entry-by-entry; the Rust registry under
> `rust/tcl-registry/…` is now the sole source of truth, guarded by the registry
> contract tests rather than by this file.

Tracks parity between the **Python source-of-truth registries** and their
**Rust ports** during the rust rewrite, so we can detect drift over time.

- **Python baseline:** `origin/main` @ **`1e3a71b7`**
  (registries under `dialects/…`, `compiler/registry/…`; mirrored on the
  rust branch under `core/…`).
- **Rust baseline:** `origin/rust` @ **`1abc0d35`**
  (Python copy under `core/…`, Rust port under `rust/tcl-registry/…`).

Each registry row is ticked when audited and stamped with the **(main / rust)**
short hashes it was verified against. To re-check for drift later: re-run the
tooling, and if either hash has moved *and* the registry's files changed,
re-audit that row and update its stamp.

**Every individual entry** (each command / object / event) is named and ticked
in the [Entry-level status](#entry-level-status-every-registry-entry-ticked)
section — a sorted, stable list so `git diff` on this file pinpoints exactly
which entries drift between runs.

> The rust branch carries **both** the Python registries (`core/…`) and the
> Rust port (`rust/tcl-registry/…`). The comparison below is Python `core/…`
> vs Rust `rust/…` on the rust branch; the Python `core/…` copy was itself
> checked against `origin/main` for drift (see BigIP — it has drifted).

## How this was generated (reproduce)

All lists/numbers below are machine-generated, not hand-curated:

```bash
# 1. dumps a normalised JSON record per command spec, both sides, all groups
bash scripts/registry-audit/run_all.sh            # -> tmp/registry-audit/*.jsonl + *.summary.json
# 2. meta registries (events / profiles / namespaces) name + props parity
./target/debug/examples/dump_specs meta-events        # also meta-profiles, meta-namespaces, meta-events-props
# 3. regenerate the data-table section of this doc
python3 scripts/registry-audit/gen_report_md.py
# 4. regenerate the per-entry status section (names every entry)
python3 scripts/registry-audit/gen_entries.py
```

Tooling (committed):

- `rust/tcl-registry/examples/dump_specs.rs` — Rust dumper (commands + meta registries).
- `scripts/registry-audit/dump_python.py` — Python dumper (same normalised schema).
- `scripts/registry-audit/compare.py` — per-group diff + completeness matrix.
- `scripts/registry-audit/run_all.sh` — runs all 13 command groups.
- `scripts/registry-audit/gen_report_md.py` — emits the aggregate tables in this doc.
- `scripts/registry-audit/gen_entries.py` — emits the per-entry status section.

The normalised schema captures, per command: name, dialects, arity, hover
(summary / synopsis / source / examples / return_value), forms, options,
subcommands, side-effects, `event_requires`, excluded events, required
package, return type, arg-types/roles, body-kind, and codegen/lowering/
const-fold/traits presence.

---

## Master checklist

Legend — **OK**: data present & correct · **minor**: small/explainable deltas ·
**DATA GAPS**: names match but Rust drops data · **NAMES DIFFER**: command set
differs · **UNPORTED**: no Rust port exists.

### Command registries (Python `core/commands/registry/<g>` ↔ Rust `rust/tcl-registry/src/commands/<g>`)

- [x] **tcl** (1e3a71b7 / 1abc0d35) — **mostly OK** — names now **0 missing** (mathop ensemble + auto_* family + named cmds restored); 5 Rust-extra are internal/namespace-qualified dups. `forms` 109, `side_effects` 84, `pattern_type` restored. Residual value-deltas (summary/synopsis text, `return_type`) are hand-authored phrasing or Rust-richer (`return_type` 92→110); tcl hover text deliberately **not** machine-overwritten to preserve the VERIFIED annotations. Python gained the Tcl 8.7/9.0 commands (GAP-f). See §1.
- [x] **stdlib** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `required_package` 110, `side_effects` 12, `forms`, `subcommands`, `options`, `arg_roles` all restored to parity. (`traits` 23→24 is Rust-richer.)
- [x] **tcllib** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 206, `side_effects` 62, `hover_return_value` 68, `hover_examples` 25, `subcommands` 3, `arg_roles` 17 all restored to parity.
- [x] **irules** (1e3a71b7 / 1abc0d35) — **OK** ✅ (GAP-3a) — `side_effects` 1002→1002, `forms` 999→999, `hover_source_url`/`examples`/`return_value`/summaries/synopses, `event_requires` 448→448, `subcommands` 47→47, `options` 54→54 all restored to parity. Remaining deltas are Rust-richer (`-noupdate` on `HTTP::header`, extra traits/arg_roles/lowering_hooks) or in the 9 hand-tuned behavioural files (arity on `clientside`/`peer`/`serverside`, `HSL::open` return_type, `after` body_kind). See §2.
- [x] **iapps** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 49 restored; all captured dimensions at parity.
- [x] **tk** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `required_package` 55, `options` 44, `side_effects` 55, `subcommands` 17, `forms` 55 all restored to parity.
- [x] **expect** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 35, `options` 26, `arg_roles` restored to parity.
- [x] **sdc-base** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 61, `arg_roles` 3 restored to parity.
- [x] **synopsys** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 68 restored to parity.
- [x] **cadence** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 56 restored to parity.
- [x] **xilinx** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 64 restored to parity.
- [x] **quartus** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 48 restored to parity.
- [x] **mentor** (1e3a71b7 / 1abc0d35) — **OK** ✅ — `forms` 49 restored to parity.

### Meta / data registries

- [x] **iRule events** (1e3a71b7 / 1abc0d35) — **OK** ✅ `events.rs` vs `namespace_data.EVENT_PROPS`: 176=176 names, all 9 prop fields match for every event. See §3.
- [x] **F5 profiles** (1e3a71b7 / 1abc0d35) — **OK** ✅ `profiles.rs` vs `PROFILE_SPECS`: 65=65 names. See §3.
- [x] **Protocol namespaces** (1e3a71b7 / 1abc0d35) — **OK** ✅ `profiles.rs` vs `PROTOCOL_NAMESPACE_SPECS`: 113=113 names. See §3.
- [x] **BigIP object registry** (1e3a71b7 / 1abc0d35) — **OK** ✅ (GAP-e) — Python copy reconciled to the canonical `origin/main` baseline (`OBJECT_SPECS` 743→992, rich properties restored) and a from-scratch Rust object registry ported (`rust/tcl-registry/src/bigip/`, 992 specs / 10,141 properties). Audit (`audit_bigip.py`): **991 kinds, 0 missing / 0 extra, 0 property mismatches.** See §4.
- [x] **Codegen / lowering hooks** (1e3a71b7 / 1abc0d35) — **modelling diff** — Rust stamps hook IDs on specs; Python dispatches via `core/compiler/codegen/`. See §5.
- [x] **Secondary infra** (taint, type-hints, operators, stub-overlay, runtime) (1e3a71b7 / 1abc0d35) — present both sides, **light-touch** verified. See §6.

---

## Cross-cutting Rust-port gaps (systematic)

> **RESOLVED (GAP-b/c/3a/d/f, JUN05).** Items 1–7 below are fixed across
> **all 13 command registries** (every group now reports **0 completeness
> gaps**; 12 of 13 also have 0 name deltas, `tcl` has 0 missing + 5
> Rust-internal extras). `forms` (1), hover `examples`/`return_value`/
> source-URL (2), structured `side_effects` (3), structured `subcommands`
> (4), `required_package` (5), iRules `event_requires`/`event_profiles`
> (6, 448), and the truncated summaries/synopses (7) were all backfilled
> from the Python source of truth via the idempotent generators in
> `scripts/registry-audit/inject_*.py` / `gen_*.py`. The only remaining
> registry gap is **BigIP** (§4, its own object registry — reconcile +
> Rust port still outstanding). The original gap descriptions are kept
> below for the historical record.

These patterns repeated across *every* command registry and were the bulk of
the data loss. They were almost certainly mechanical port omissions, not
deliberate:

1. **`forms` / `FormSpec` data dropped everywhere.** Python carries per-form
   synopsis + options + per-form arity + arg-values on ~all commands; the Rust
   `FormSpec` is a `kind`+`synopsis` stub and is **never populated** (0 files).
   Its structured replacement `command_forms` is used by **2** commands only
   (`lset`, `incr`). → completion / per-form arity data is absent.
2. **Hover `examples` and `return_value` dropped**, and real doc **`source`
   URLs** replaced with a generic label (e.g. `"F5 iRules"`). iRules: examples
   813→0, return_value 496→0, source-URL 996→0.
3. **Structured `side_effects` hints dropped** (irules 1002→0, tcllib 62→0,
   tk 55→0, stdlib 12→0). Some coarse info may survive via `traits`
   (`PURE`/`UNSAFE`), but the structured target/reads/writes/storage detail is gone.
4. **Structured `subcommands` dropped** (irules 47→0, tk 17→0). The subcommand
   *names* survive only as free text inside the hover synopsis string.
5. **`required_package` dropped** (stdlib 110→0, tk 55→0). Rust specs no longer
   record which `package require` gates a command.
6. **`event_requires` (profiles / also_in) dropped for iRules** (448→0). The
   Rust `CommandSpec` has **no `event_requires` field at all** (the
   `EventRequires` type exists in `events.rs` but isn't attached to commands).
7. **Summaries truncated** — 124 iRules summaries are cut mid-word
   (e.g. `…sent successfully` → `…sent success`).

Not everything is loss — Rust is **richer** in a few dimensions (e.g. `tcl`
`return_type` 92→110, `subcommands` 13→18, `const_fold` 10→12, and spec-level
`traits`/codegen hooks), and it **adds newer Tcl 9.0 commands** Python lacks
(see §1).

---

## §1 — `tcl` registry (NAMES DIFFER)

Python 214 vs Rust 126: **104 missing in Rust, 16 extra**.

- **84 missing are `tcl::mathop` operators** (`+ - * / < > == …` and their
  `tcl::mathop::X` / `::tcl::mathop::X` spellings). Rust instead models these as
  a single `tcl::mathop` **ensemble** command (present in "extra"). *Modelling
  choice*, but the bare-operator and fully-qualified command entries Python
  registers are not individually resolvable in Rust.
- **8 genuinely missing library commands:** `auto_execok`, `auto_import`,
  `auto_load`, `auto_mkindex`, `auto_mkindex_old`, `auto_qualify`,
  `auto_reset`, `tcl_findLibrary`.
- **12 other missing:** `pwd`, `nextto`, `bgerror`, `pkg_mkindex`,
  `pkg::create`, `memory`, `filename`, `http`, `tcl::build-info` /
  `::tcl::build-info`, `regexp::quote` / `regex::quote`
  (Rust spells the last as `regexp_quote` — **naming mismatch `::`↔`_`**).
- **16 Rust-only ("extra") — mostly Tcl 8.7/9.0 commands Python is missing and
  should add:** `const`, `lpop`, `coroinject`, `coroprobe`, `timerate`,
  `readFile`, `writeFile`, `foreachLine`, `tcl::idna`, `tcl::process`,
  `tcl::unsupported::corotype`, plus `tcl::mathop` (ensemble),
  `regexp_quote` (naming), `disabled_in_irules` (synthetic marker).

Also: 86 of Python's 214 tcl commands have hover that the (smaller) Rust set
doesn't cover — largely a consequence of the 104 missing names.

## §2 — `irules` registry (DATA GAPS, severe)

**Name parity is perfect (1015 = 1015, 0 missing / 0 extra)** — but the port is
a names-and-one-line-summary shell. Per the completeness matrix:

`side_effects` 1002→0 · `forms` 999→0 · `hover_source_url` 996→0 ·
`hover_examples` 813→0 · `hover_return_value` 496→0 · `event_requires` 448→0 ·
`event_profiles` 373→0 · `subcommands` 47→0 · `options` 54→5 ·
`excluded_events` 4→0. Plus 124 truncated summaries and 143 differing synopses.

This is the registry most in need of a re-port that carries the full
`CommandSpec` data (and an `event_requires` field added to the Rust spec).

## §3 — iRule events / profiles / namespaces (OK ✅)

Model ports — full name parity, and for events full **data** parity too:

| Registry | Python | Rust | Names | Data |
|---|--:|--:|---|---|
| iRule events (`EVENT_PROPS`) | 176 | 176 | ✅ identical | ✅ all 9 prop fields (`client_side`, `server_side`, `transport`, `implied_profiles`, `flow`, `deprecated`, `hot`, `common`, `setup_event`) match for every event |
| F5 profiles (`PROFILE_SPECS`) | 65 | 65 | ✅ identical | spot-checked; recommend a full `ProfileSpec` field diff as follow-up |
| Protocol namespaces (`PROTOCOL_NAMESPACE_SPECS`) | 113 | 113 | ✅ identical | recommend a full `ProtocolNamespaceSpec` field diff as follow-up |

Nit: `rust/tcl-registry/src/lib.rs` doc-comment claims "247 events, 57 profiles,
87 namespaces" — **stale**; the real registries hold 176 / 65 / 113. Update the
comment.

## §4 — BigIP object registry (OK ✅ — GAP-e closed)

> **RESOLVED (GAP-e, JUN05).** Both stages are done and at parity:
>
> - **Stage 1+2 — Python reconcile to `origin/main`.** Adopted main's
>   canonical `models.py` (superset: `ValueKind` + rich
>   `BigipPropertySpec` fields) and `specs/_base.py` (`normalise_registry`
>   post-pass); added the 245 main-only specs and enriched the 539
>   drifted shared specs to full property detail (`auth_ldap` 1 → 30
>   properties); kept the 194 rust-branch-only thin stubs as a
>   non-destructive superset. **`OBJECT_SPECS` 743 → 992.** Gated on the
>   bigip object-registry test + 95 bigip consumer tests.
>   (`scripts/registry-audit/reconcile_bigip.py`.)
> - **Stage 3 — Rust port.** New `rust/tcl-registry/src/bigip/` module:
>   `ValueKind` / `BigipObjectKindSpec` / `BigipPropertySpec` (with
>   `block` nesting) / `BigipObjectSpec` / `BigipRegistry`, plus 992
>   specs / 10,141 properties as `&'static` data generated from the
>   reconciled `OBJECT_SPECS` (`gen_bigip_rust.py`). Audit
>   (`audit_bigip.py`, wired into `run_all.sh`): **991 object kinds, 0
>   missing / 0 extra, 0 property-count / property-name / value-kind /
>   module mismatches.**
>
> The original UNPORTED analysis is kept below for the record.

### Original analysis (pre-port)

- **Rust: 0 files** — the entire BigIP object/property registry is unported.
  Python (`core/bigip/registry/`) holds **743** `OBJECT_SPECS` across 748 spec
  files, each a rich `BigipObjectSpec` (kind, header types, and
  `BigipPropertySpec` properties with value-type / required / repeated /
  enum-values / min-max / pattern / references / description).
- **Python copy ↔ main has diverged (both directions):** spec file stems —
  **554 shared**, **245 only on `origin/main`** (e.g. the `analytics_*_scheduled_report`
  family, `apm_aaa_*`, `api_protection_*` — confirmed absent from core's
  `OBJECT_SPECS`), **194 only on rust-branch `core/`** (e.g. `cm_*`, `gtm_*`
  variants — verify renamed vs genuinely extra). File counts: core 748 vs main
  799. The rust branch needs a Python-side **reconcile** (not just a fast-forward)
  before the Rust port begins, or the port will bake in a stale/forked spec set.
  Full per-entry lists in the entry-level section.

Action: this is a whole-registry port (add) **plus** a Python-side reconcile —
the largest single gap.

### GAP-e progress note (JUN05, origin/main re-fetched)

`origin/main` is now fetchable in the workspace, so the divergence was
re-measured and characterised directly:

- **Divergence confirmed:** 552 shared (excl. `__init__`/`_base`), 245
  main-only, 194 core-only spec stems (core 748 vs main 799). Of the 552
  shared, **539 differ in content.**
- **Direction = core drifted to thin stubs.** main is the canonical,
  *rich* baseline: e.g. `auth_ldap` on main carries the full
  `BigipPropertySpec` set (value-type / enum-values / defaults /
  usage-flags / references); core's copy is a one-property stub. So the
  reconcile for the 539 shared + 245 main-only is "adopt main's rich
  specs."
- **The reconcile also needs a models-API merge, NOT just file copies.**
  main's specs use `BigipPropertySpec` fields that core's `models.py`
  does **not** define (`default`, `shape_kind`, `usage_flags`), and
  `models.py` itself diverged bidirectionally (~113 lines, both ways) —
  so importing main's specs into core requires first reconciling
  `core/bigip/registry/models.py` (+ `_base.py`) against main **without
  breaking the existing bigip Python consumers** (analyser / tests that
  read the current core model shape). This is a delicate, judgement-heavy
  merge, not a mechanical overwrite.
- **194 core-only (`sys_*` 73, `security_*` 62, `ltm_*` 19, `net_*` 18,
  `cm_*` 11, `gtm_*` 8, `auth_*` 1)** are absent from main — keep-vs-drop
  is intent-dependent (rust-branch additions vs renamed/removed-on-main);
  removing them is destructive and was deliberately **not** done here.

**Status: still the one remaining large gap.** Sequencing (each its own
commit, as instructed): (1) reconcile `models.py`/`_base.py` to main
additively; (2) adopt main's 245+539 rich specs, decide the 194 core-only,
regenerate the specs `__init__`, gate on `test_bigip_object_registry`;
(3) build the from-scratch Rust BigIP object registry (new
`BigipObjectSpec`/`BigipPropertySpec` types + registry + audit-dumper
support) and port all objects. Not attempted in this pass to avoid a
rushed, consumer-breaking change to an 800-spec registry + a
models-API merge.

## §5 — Codegen / lowering registry (modelling difference)

Not a data loss — a different shape:

- **Rust** stamps hook IDs on the command spec (`lowering_hook`, `codegen_hook`,
  `const_fold`). On `tcl`: 23 lowering, 7 codegen, 12 const-fold hooks.
- **Python** never sets `lowering`/`codegens` on specs (0 across `tcl`); it
  dispatches codegen via `core/compiler/codegen/bytecoded/` modules
  (`_array`, `_control`, `_dict`, `_list`, `_string`, …, `register_all`) keyed
  by command name. `const_fold` *is* on Python specs (tcl 10 vs Rust 12 — near
  parity).

To audit true codegen coverage parity, compare *the set of commands with a
bytecode handler* (Python `bytecoded` modules) against Rust `codegen_hook`
stamping — recommended as a dedicated follow-up (not a registry-data diff).

## §6 — Secondary infra registries (light-touch)

Present on both sides; counts noted, deep field diffs deferred:

| Registry | Python | Rust | Note |
|---|---|---|---|
| Taint | `taint_hints.py` (+`taint_sink_info.py`) | `taint.rs` (222 lines) | both present; per-command taint hints live on specs — recommend a dedicated diff |
| Type hints | `type_hints.py` (`TclType` ×10) | `types.rs` (`TclType`) | enum parity likely; verify variant set |
| Operators (hover) | `operators.py` (`TCL_OPERATOR_HOVER` ×8, `IRULES_OPERATOR_HOVER` ×9) | expr compiler (`tcl-compiler/src/expr_*`) | operator *hover* tables are Python-only data; check they're surfaced in Rust |
| Stub overlay | (compiler stub types/comments) | `stub_overlay.rs` (412 lines) | present; not diffed |
| Const-fold | on specs | `const_fold.rs` (501 lines) | near parity on `tcl` (10 vs 12) |

---

## Action list

### Rust port — add/restore data (by priority)

1. **BigIP registry** — port the whole thing (743+ specs). *(largest gap)*
2. **iRules `CommandSpec` re-port** — restore `side_effects`, `forms`/options,
   `subcommands`, hover `examples`/`return_value`/source-URL, and **add an
   `event_requires` field** to the Rust spec, then populate `event_requires`
   (448 commands) and `excluded_events`.
3. **Restore `forms`/`command_forms`** across all command registries (only
   `lset`/`incr` ported) — needed for completion & per-form arity.
4. **Restore `required_package`** (stdlib 110, tk 55) and structured
   `subcommands` (tk 17, tcllib 3, stdlib 2).
5. **Fix truncated iRules summaries** (124) and the differing synopses (143).
6. **`tcl` coverage:** add `pwd`, `nextto`, `bgerror`, `pkg_mkindex`, the
   `auto_*` family (8) + `tcl_findLibrary`; reconcile the `tcl::mathop` ensemble
   vs per-operator commands; fix `regexp_quote` ↔ `regexp::quote` naming.

### Python source — add/change

7. **Add Tcl 8.7/9.0 commands** the Rust port already has: `const`, `lpop`,
   `coroinject`, `coroprobe`, `timerate`, `readFile`, `writeFile`,
   `foreachLine`, `tcl::idna`, `tcl::process`.
8. **Reconcile `core/bigip/registry`** with `origin/main` before porting — 245
   main-only + 194 core-only spec stems (bidirectional divergence, not a
   fast-forward). See the bigip divergence list in the entry-level section.

### Docs / hygiene

9. Fix the stale "247 events / 57 profiles / 87 namespaces" comment in
   `rust/tcl-registry/src/lib.rs` (actual: 176 / 65 / 113).
10. Follow-ups: full `ProfileSpec` / `ProtocolNamespaceSpec` field diffs;
    codegen-handler coverage diff (§5); taint & operator-hover diffs (§6).

---

## Appendix — generated data tables

<!-- BEGIN generated: scripts/registry-audit/gen_report_md.py -->

### Command registries — name parity & verdict

| Registry | Python | Rust | Missing in Rust | Extra in Rust | Verdict |
|---|--:|--:|--:|--:|---|
| `tcl` | 224 | 229 | 0 | 5 | OK |
| `stdlib` | 225 | 225 | 0 | 0 | OK |
| `tcllib` | 206 | 206 | 0 | 0 | OK |
| `irules` | 1015 | 1015 | 0 | 0 | OK |
| `iapps` | 49 | 49 | 0 | 0 | OK |
| `tk` | 55 | 55 | 0 | 0 | OK |
| `expect` | 35 | 35 | 0 | 0 | OK |
| `sdc-base` | 61 | 61 | 0 | 0 | OK |
| `synopsys` | 68 | 68 | 0 | 0 | OK |
| `cadence` | 56 | 56 | 0 | 0 | OK |
| `xilinx` | 64 | 64 | 0 | 0 | OK |
| `quartus` | 48 | 48 | 0 | 0 | OK |
| `mentor` | 49 | 49 | 0 | 0 | OK |

### Command registries — data completeness (commands carrying each dimension)

Only dimensions where Python and Rust differ are shown. `py→rust`.

- **`tcl`** — `arg_types` 19→21, `const_fold` 10→12, `forms` 209→211, `required_package` 0→2, `subcommands` 13→18, `arg_roles` 30→36, `side_effects` 84→90, `codegen_hook` 0→7, `hover` 221→228, `hover_synopsis` 206→213, `arity_bounded` 170→178, `return_type` 102→114, `lowering_hook` 0→23, `traits` 42→109
- **`stdlib`** — `traits` 23→24
- **`tcllib`** — all captured dimensions at parity.
- **`irules`** — `options` 54→55, `lowering_hook` 0→2, `arity_bounded` 31→34, `arg_roles` 4→8, `traits` 62→72
- **`iapps`** — all captured dimensions at parity.
- **`tk`** — all captured dimensions at parity.
- **`expect`** — all captured dimensions at parity.
- **`sdc-base`** — all captured dimensions at parity.
- **`synopsys`** — all captured dimensions at parity.
- **`cadence`** — all captured dimensions at parity.
- **`xilinx`** — `arity_bounded` 18→19
- **`quartus`** — all captured dimensions at parity.
- **`mentor`** — all captured dimensions at parity.

### Command registries — value mismatches on common commands

| Registry | field | # mismatched | examples |
|---|---|--:|---|
| `tcl` | summary | 3 | `exec`, `lsearch`, `lsort` |
| `tcl` | synopsis | 38 | `apply`, `binary`, `classvariable` |
| `tcl` | body_kind | 3 | `oo::abstract`, `oo::configurable`, `oo::singleton` |
| `tcl` | return_type | 110 | `::tcl::build-info`, `after`, `append` |
| `tcl` | arity_min | 8 | `flush`, `oo::abstract`, `oo::class` |
| `tcl` | arity_max | 3 | `fcopy`, `oo::copy`, `source` |
| `stdlib` | body_kind | 1 | `tcltest::test` |
| `stdlib` | return_type | 3 | `http::requestHeaders`, `http::responseHeaders`, `http::responseInfo` |
| `tcllib` | body_kind | 5 | `snit::compile`, `snit::macro`, `snit::type` |
| `tcllib` | return_type | 6 | `ip::collapse`, `ip::is`, `ip::subtract` |
| `irules` | body_kind | 1 | `after` |
| `irules` | return_type | 1 | `HSL::open` |
| `irules` | arity_min | 1 | `peer` |
| `irules` | arity_max | 3 | `clientside`, `peer`, `serverside` |
| `sdc-base` | synopsis | 12 | `all_registers`, `create_generated_clock`, `group_path` |
| `synopsys` | synopsis | 5 | `compile`, `compile_ultra`, `create_floorplan` |
| `cadence` | synopsis | 1 | `create_floorplan` |
| `xilinx` | synopsis | 3 | `phys_opt_design`, `report_timing`, `synth_design` |
| `xilinx` | arity_min | 1 | `read_xdc` |
| `xilinx` | arity_max | 1 | `open_run` |
| `quartus` | synopsis | 2 | `execute_flow`, `report_timing` |
| `mentor` | synopsis | 4 | `add_wave`, `vcom`, `vlog` |

<!-- END generated -->

---

## Entry-level status (every registry entry, ticked)

Generated by `scripts/registry-audit/gen_entries.py`. Status codes mark data the Python entry carries that the Rust entry drops: `forms` `opt`(options) `sub`(subcommands) `sfx`(side-effects) `evtreq`/`evtprof`(event-requires/profiles) `ex`(examples) `ret`(return-value) `srcurl`(doc URL) `pkg`(required-package) `exev`(excluded-events) `hover` · `sum≠`/`syn≠`/`arity≠` mark value mismatches. `✅` = Rust carries everything Python does (for the tracked dims).

### Command registries

<details><summary><b>tcl</b> — 229 entries · 183 ✅ · 46 need work</summary>

| entry | status |
|---|---|
| `!` | ✅ |
| `!=` | ✅ |
| `%` | ✅ |
| `&` | ✅ |
| `&&` | ✅ |
| `*` | ✅ |
| `**` | ✅ |
| `+` | ✅ |
| `-` | ✅ |
| `/` | ✅ |
| `::tcl::build-info` | ✅ |
| `::tcl::idna` | ➕ rust-only |
| `::tcl::mathop::!` | ✅ |
| `::tcl::mathop::!=` | ✅ |
| `::tcl::mathop::%` | ✅ |
| `::tcl::mathop::&` | ✅ |
| `::tcl::mathop::&&` | ✅ |
| `::tcl::mathop::*` | ✅ |
| `::tcl::mathop::**` | ✅ |
| `::tcl::mathop::+` | ✅ |
| `::tcl::mathop::-` | ✅ |
| `::tcl::mathop::/` | ✅ |
| `::tcl::mathop::<` | ✅ |
| `::tcl::mathop::<<` | ✅ |
| `::tcl::mathop::<=` | ✅ |
| `::tcl::mathop::==` | ✅ |
| `::tcl::mathop::>` | ✅ |
| `::tcl::mathop::>=` | ✅ |
| `::tcl::mathop::>>` | ✅ |
| `::tcl::mathop::@` | ✅ |
| `::tcl::mathop::^` | ✅ |
| `::tcl::mathop::eq` | ✅ |
| `::tcl::mathop::in` | ✅ |
| `::tcl::mathop::max` | ✅ |
| `::tcl::mathop::min` | ✅ |
| `::tcl::mathop::ne` | ✅ |
| `::tcl::mathop::ni` | ✅ |
| `::tcl::mathop::|` | ✅ |
| `::tcl::mathop::||` | ✅ |
| `::tcl::mathop::~` | ✅ |
| `::tcl::process` | ➕ rust-only |
| `::tcl::unsupported::corotype` | ➕ rust-only |
| `<` | ✅ |
| `<<` | ✅ |
| `<=` | ✅ |
| `==` | ✅ |
| `>` | ✅ |
| `>=` | ✅ |
| `>>` | ✅ |
| `@` | ✅ |
| `^` | ✅ |
| `after` | ✅ |
| `append` | ✅ |
| `apply` | `syn≠` |
| `array` | ✅ |
| `auto_execok` | ✅ |
| `auto_import` | ✅ |
| `auto_load` | ✅ |
| `auto_mkindex` | ✅ |
| `auto_mkindex_old` | ✅ |
| `auto_qualify` | ✅ |
| `auto_reset` | ✅ |
| `bgerror` | ✅ |
| `binary` | `syn≠` |
| `break` | ✅ |
| `catch` | ✅ |
| `cd` | ✅ |
| `chan` | ✅ |
| `classvariable` | `syn≠` |
| `clock` | `syn≠` |
| `close` | ✅ |
| `concat` | `syn≠` |
| `const` | ✅ |
| `continue` | ✅ |
| `coroinject` | ✅ |
| `coroprobe` | ✅ |
| `coroutine` | `syn≠` |
| `dict` | `syn≠` |
| `disabled_in_irules` | ➕ rust-only |
| `encoding` | `syn≠` |
| `eof` | ✅ |
| `eq` | ✅ |
| `error` | ✅ |
| `eval` | ✅ |
| `exec` | `sum≠` `syn≠` |
| `exit` | ✅ |
| `expr` | ✅ |
| `fblocked` | ✅ |
| `fconfigure` | ✅ |
| `fcopy` | `arity≠` |
| `file` | ✅ |
| `fileevent` | ✅ |
| `filename` | ✅ |
| `flush` | `arity≠` |
| `for` | ✅ |
| `foreach` | `syn≠` |
| `foreachLine` | ✅ |
| `format` | `syn≠` |
| `gets` | ✅ |
| `glob` | ✅ |
| `global` | ✅ |
| `http` | ✅ |
| `if` | `syn≠` |
| `in` | ✅ |
| `incr` | ✅ |
| `info` | ✅ |
| `interp` | `syn≠` |
| `join` | ✅ |
| `lappend` | ✅ |
| `lassign` | ✅ |
| `lindex` | ✅ |
| `linsert` | `syn≠` |
| `list` | `syn≠` |
| `llength` | ✅ |
| `lmap` | ✅ |
| `load` | `syn≠` |
| `lpop` | ✅ |
| `lrange` | ✅ |
| `lremove` | ✅ |
| `lrepeat` | ✅ |
| `lreplace` | `syn≠` |
| `lreverse` | ✅ |
| `lsearch` | `sum≠` `syn≠` |
| `lseq` | ✅ |
| `lset` | ✅ |
| `lsort` | `sum≠` `syn≠` |
| `max` | ✅ |
| `memory` | ✅ |
| `min` | ✅ |
| `my` | ✅ |
| `namespace` | ✅ |
| `ne` | ✅ |
| `next` | ✅ |
| `nextto` | ✅ |
| `ni` | ✅ |
| `oo::abstract` | `syn≠` `arity≠` |
| `oo::class` | `syn≠` `arity≠` |
| `oo::configurable` | `syn≠` `arity≠` |
| `oo::copy` | `syn≠` `arity≠` |
| `oo::define` | `syn≠` `arity≠` |
| `oo::objdefine` | `syn≠` `arity≠` |
| `oo::object` | `syn≠` `arity≠` |
| `oo::singleton` | `syn≠` `arity≠` |
| `open` | ✅ |
| `package` | `syn≠` |
| `parray` | ✅ |
| `pid` | ✅ |
| `pkg::create` | ✅ |
| `pkg_mkindex` | ✅ |
| `proc` | ✅ |
| `puts` | ✅ |
| `pwd` | ✅ |
| `re_quote` | `syn≠` |
| `read` | ✅ |
| `readFile` | ✅ |
| `regex::quote` | `syn≠` |
| `regex_quote` | `syn≠` |
| `regexp` | ✅ |
| `regexp::quote` | `syn≠` |
| `registry` | ✅ |
| `regsub` | ✅ |
| `rename` | ✅ |
| `return` | ✅ |
| `scan` | `syn≠` |
| `seek` | ✅ |
| `self` | ✅ |
| `set` | ✅ |
| `socket` | `syn≠` |
| `source` | `arity≠` |
| `split` | ✅ |
| `string` | ✅ |
| `subst` | `syn≠` |
| `switch` | ✅ |
| `tailcall` | ✅ |
| `tcl::build-info` | ✅ |
| `tcl::idna` | ✅ |
| `tcl::mathop` | ➕ rust-only |
| `tcl::mathop::!` | ✅ |
| `tcl::mathop::!=` | ✅ |
| `tcl::mathop::%` | ✅ |
| `tcl::mathop::&` | ✅ |
| `tcl::mathop::&&` | ✅ |
| `tcl::mathop::*` | ✅ |
| `tcl::mathop::**` | ✅ |
| `tcl::mathop::+` | ✅ |
| `tcl::mathop::-` | ✅ |
| `tcl::mathop::/` | ✅ |
| `tcl::mathop::<` | ✅ |
| `tcl::mathop::<<` | ✅ |
| `tcl::mathop::<=` | ✅ |
| `tcl::mathop::==` | ✅ |
| `tcl::mathop::>` | ✅ |
| `tcl::mathop::>=` | ✅ |
| `tcl::mathop::>>` | ✅ |
| `tcl::mathop::@` | ✅ |
| `tcl::mathop::^` | ✅ |
| `tcl::mathop::eq` | ✅ |
| `tcl::mathop::in` | ✅ |
| `tcl::mathop::max` | ✅ |
| `tcl::mathop::min` | ✅ |
| `tcl::mathop::ne` | ✅ |
| `tcl::mathop::ni` | ✅ |
| `tcl::mathop::|` | ✅ |
| `tcl::mathop::||` | ✅ |
| `tcl::mathop::~` | ✅ |
| `tcl::process` | ✅ |
| `tcl_findLibrary` | ✅ |
| `tell` | ✅ |
| `throw` | ✅ |
| `time` | ✅ |
| `timerate` | ✅ |
| `trace` | ✅ |
| `try` | ✅ |
| `unknown` | `syn≠` |
| `unload` | `syn≠` |
| `unset` | ✅ |
| `update` | ✅ |
| `uplevel` | ✅ |
| `upvar` | ✅ |
| `variable` | ✅ |
| `vwait` | ✅ |
| `while` | ✅ |
| `writeFile` | ✅ |
| `yield` | ✅ |
| `yieldto` | `syn≠` |
| `zlib` | ✅ |
| `|` | ✅ |
| `||` | ✅ |
| `~` | ✅ |

</details>

<details><summary><b>stdlib</b> — 225 entries · 225 ✅ · 0 need work</summary>

| entry | status |
|---|---|
| `gettimes` | ✅ |
| `history` | ✅ |
| `http::cleanup` | ✅ |
| `http::code` | ✅ |
| `http::config` | ✅ |
| `http::cookiejar` | ✅ |
| `http::data` | ✅ |
| `http::error` | ✅ |
| `http::formatQuery` | ✅ |
| `http::geturl` | ✅ |
| `http::meta` | ✅ |
| `http::ncode` | ✅ |
| `http::postError` | ✅ |
| `http::quoteString` | ✅ |
| `http::reasonPhrase` | ✅ |
| `http::register` | ✅ |
| `http::registerError` | ✅ |
| `http::requestHeaderValue` | ✅ |
| `http::requestHeaders` | ✅ |
| `http::requestLine` | ✅ |
| `http::reset` | ✅ |
| `http::responseBody` | ✅ |
| `http::responseCode` | ✅ |
| `http::responseHeaderValue` | ✅ |
| `http::responseHeaders` | ✅ |
| `http::responseInfo` | ✅ |
| `http::responseLine` | ✅ |
| `http::size` | ✅ |
| `http::status` | ✅ |
| `http::unregister` | ✅ |
| `http::wait` | ✅ |
| `lgen` | ✅ |
| `lstring` | ✅ |
| `msgcat::mc` | ✅ |
| `msgcat::mcexists` | ✅ |
| `msgcat::mcflmset` | ✅ |
| `msgcat::mcflset` | ✅ |
| `msgcat::mcforgetpackage` | ✅ |
| `msgcat::mcload` | ✅ |
| `msgcat::mcloadedlocales` | ✅ |
| `msgcat::mclocale` | ✅ |
| `msgcat::mcmax` | ✅ |
| `msgcat::mcmset` | ✅ |
| `msgcat::mcn` | ✅ |
| `msgcat::mcpackageconfig` | ✅ |
| `msgcat::mcpackagelocale` | ✅ |
| `msgcat::mcpackagenamespaceget` | ✅ |
| `msgcat::mcpreferences` | ✅ |
| `msgcat::mcset` | ✅ |
| `msgcat::mcunknown` | ✅ |
| `msgcat::mcutil` | ✅ |
| `noop` | ✅ |
| `pkg::create` | ✅ |
| `pkg_mkIndex` | ✅ |
| `platform::generic` | ✅ |
| `platform::identify` | ✅ |
| `platform::patterns` | ✅ |
| `platform::shell::generic` | ✅ |
| `platform::shell::identify` | ✅ |
| `safe::interpAddToAccessPath` | ✅ |
| `safe::interpConfigure` | ✅ |
| `safe::interpCreate` | ✅ |
| `safe::interpDelete` | ✅ |
| `safe::interpFindInAccessPath` | ✅ |
| `safe::interpInit` | ✅ |
| `safe::setLogCmd` | ✅ |
| `safe::setSyncMode` | ✅ |
| `tcl::OptKeyDelete` | ✅ |
| `tcl::OptKeyError` | ✅ |
| `tcl::OptKeyParse` | ✅ |
| `tcl::OptKeyRegister` | ✅ |
| `tcl::OptParse` | ✅ |
| `tcl::OptProc` | ✅ |
| `tcl::OptProcArgGiven` | ✅ |
| `tcl::idna::decode` | ✅ |
| `tcl::idna::encode` | ✅ |
| `tcl::tm::path` | ✅ |
| `tcl::tm::roots` | ✅ |
| `tcl_endOfWord` | ✅ |
| `tcl_startOfNextWord` | ✅ |
| `tcl_startOfPreviousWord` | ✅ |
| `tcl_wordBreakAfter` | ✅ |
| `tcl_wordBreakBefore` | ✅ |
| `tcltest::bytestring` | ✅ |
| `tcltest::cleanupTests` | ✅ |
| `tcltest::configure` | ✅ |
| `tcltest::customMatch` | ✅ |
| `tcltest::debug` | ✅ |
| `tcltest::errorChannel` | ✅ |
| `tcltest::errorFile` | ✅ |
| `tcltest::getMatchingFiles` | ✅ |
| `tcltest::interpreter` | ✅ |
| `tcltest::limitConstraints` | ✅ |
| `tcltest::loadFile` | ✅ |
| `tcltest::loadScript` | ✅ |
| `tcltest::loadTestedCommands` | ✅ |
| `tcltest::mainThread` | ✅ |
| `tcltest::makeDirectory` | ✅ |
| `tcltest::makeFile` | ✅ |
| `tcltest::match` | ✅ |
| `tcltest::matchDirectories` | ✅ |
| `tcltest::matchFiles` | ✅ |
| `tcltest::normalizeMsg` | ✅ |
| `tcltest::normalizePath` | ✅ |
| `tcltest::outputChannel` | ✅ |
| `tcltest::outputFile` | ✅ |
| `tcltest::preserveCore` | ✅ |
| `tcltest::removeDirectory` | ✅ |
| `tcltest::removeFile` | ✅ |
| `tcltest::restoreState` | ✅ |
| `tcltest::runAllTests` | ✅ |
| `tcltest::saveState` | ✅ |
| `tcltest::singleProcess` | ✅ |
| `tcltest::skip` | ✅ |
| `tcltest::skipDirectories` | ✅ |
| `tcltest::skipFiles` | ✅ |
| `tcltest::temporaryDirectory` | ✅ |
| `tcltest::test` | ✅ |
| `tcltest::testConstraint` | ✅ |
| `tcltest::testsDirectory` | ✅ |
| `tcltest::threadReap` | ✅ |
| `tcltest::verbose` | ✅ |
| `tcltest::viewFile` | ✅ |
| `tcltest::workingDirectory` | ✅ |
| `testapplylambda` | ✅ |
| `testappverifierpresent` | ✅ |
| `testasync` | ✅ |
| `testbigdata` | ✅ |
| `testbignumobj` | ✅ |
| `testbooleanobj` | ✅ |
| `testbumpinterpepoch` | ✅ |
| `testbytestring` | ✅ |
| `testchannel` | ✅ |
| `testchannelevent` | ✅ |
| `testcmdinfo` | ✅ |
| `testcmdtoken` | ✅ |
| `testcmdtrace` | ✅ |
| `testconcatobj` | ✅ |
| `testcpuid` | ✅ |
| `testcreatecommand` | ✅ |
| `testdcall` | ✅ |
| `testdel` | ✅ |
| `testdelassocdata` | ✅ |
| `testdoubledigits` | ✅ |
| `testdoubleobj` | ✅ |
| `testdstring` | ✅ |
| `testencoding` | ✅ |
| `testevalex` | ✅ |
| `testevalobjv` | ✅ |
| `testevent` | ✅ |
| `testexithandler` | ✅ |
| `testexitmainloop` | ✅ |
| `testexprdouble` | ✅ |
| `testexprdoubleobj` | ✅ |
| `testexprlong` | ✅ |
| `testexprlongobj` | ✅ |
| `testexprparser` | ✅ |
| `testexprstring` | ✅ |
| `testfevent` | ✅ |
| `testfile` | ✅ |
| `testfilelink` | ✅ |
| `testfilesystem` | ✅ |
| `testfindfirst` | ✅ |
| `testfindlast` | ✅ |
| `testfstildeexpand` | ✅ |
| `testgetassocdata` | ✅ |
| `testgetindexfromobjstruct` | ✅ |
| `testgetint` | ✅ |
| `testgetintforindex` | ✅ |
| `testgetplatform` | ✅ |
| `testgetunichar` | ✅ |
| `testgetvarfullname` | ✅ |
| `testhandlecount` | ✅ |
| `testhashsystemhash` | ✅ |
| `testindexobj` | ✅ |
| `testinterpdelete` | ✅ |
| `testinterpresolver` | ✅ |
| `testintobj` | ✅ |
| `testlink` | ✅ |
| `testlinkarray` | ✅ |
| `testlistobj` | ✅ |
| `testlistrep` | ✅ |
| `testlocale` | ✅ |
| `testlongsize` | ✅ |
| `testlutil` | ✅ |
| `testmainthread` | ✅ |
| `testmsb` | ✅ |
| `testnrelevels` | ✅ |
| `testnreunwind` | ✅ |
| `testnumutfchars` | ✅ |
| `testobj` | ✅ |
| `testpanic` | ✅ |
| `testparseargs` | ✅ |
| `testparser` | ✅ |
| `testparsevar` | ✅ |
| `testparsevarname` | ✅ |
| `testpreferstable` | ✅ |
| `testprint` | ✅ |
| `testpurebytesobj` | ✅ |
| `testregexp` | ✅ |
| `testreturn` | ✅ |
| `testsaveresult` | ✅ |
| `testservicemode` | ✅ |
| `testset2` | ✅ |
| `testsetassocdata` | ✅ |
| `testsetbytearraylength` | ✅ |
| `testseterr` | ✅ |
| `testseterrorcode` | ✅ |
| `testsetmainloop` | ✅ |
| `testsetnoerr` | ✅ |
| `testsetobjerrorcode` | ✅ |
| `testsetplatform` | ✅ |
| `testsimplefilesystem` | ✅ |
| `testsize` | ✅ |
| `testsocket` | ✅ |
| `teststaticlibrary` | ✅ |
| `teststaticpkg` | ✅ |
| `teststringbytes` | ✅ |
| `teststringobj` | ✅ |
| `testtranslatefilename` | ✅ |
| `testuniclass` | ✅ |
| `testupvar` | ✅ |
| `testutfnext` | ✅ |
| `testutfprev` | ✅ |
| `testwrongnumargs` | ✅ |

</details>

<details><summary><b>tcllib</b> — 206 entries · 206 ✅ · 0 need work</summary>

| entry | status |
|---|---|
| `base64::decode` | ✅ |
| `base64::encode` | ✅ |
| `cmdline::getArgv0` | ✅ |
| `cmdline::getKnownOpt` | ✅ |
| `cmdline::getKnownOptions` | ✅ |
| `cmdline::getfiles` | ✅ |
| `cmdline::getopt` | ✅ |
| `cmdline::getoptions` | ✅ |
| `cmdline::typedGetopt` | ✅ |
| `cmdline::typedGetoptions` | ✅ |
| `cmdline::typedUsage` | ✅ |
| `cmdline::usage` | ✅ |
| `csv::iscomplete` | ✅ |
| `csv::join` | ✅ |
| `csv::joinlist` | ✅ |
| `csv::joinmatrix` | ✅ |
| `csv::read2matrix` | ✅ |
| `csv::read2queue` | ✅ |
| `csv::report` | ✅ |
| `csv::split` | ✅ |
| `csv::split2matrix` | ✅ |
| `csv::split2queue` | ✅ |
| `csv::writematrix` | ✅ |
| `csv::writequeue` | ✅ |
| `dns::address` | ✅ |
| `dns::cleanup` | ✅ |
| `dns::cname` | ✅ |
| `dns::configure` | ✅ |
| `dns::dump` | ✅ |
| `dns::error` | ✅ |
| `dns::errorcode` | ✅ |
| `dns::name` | ✅ |
| `dns::reset` | ✅ |
| `dns::resolve` | ✅ |
| `dns::result` | ✅ |
| `dns::status` | ✅ |
| `dns::wait` | ✅ |
| `fileutil::appendToFile` | ✅ |
| `fileutil::cat` | ✅ |
| `fileutil::fileType` | ✅ |
| `fileutil::find` | ✅ |
| `fileutil::findByPattern` | ✅ |
| `fileutil::foreachLine` | ✅ |
| `fileutil::fullnormalize` | ✅ |
| `fileutil::grep` | ✅ |
| `fileutil::insertIntoFile` | ✅ |
| `fileutil::install` | ✅ |
| `fileutil::jail` | ✅ |
| `fileutil::lexnormalize` | ✅ |
| `fileutil::maketempdir` | ✅ |
| `fileutil::relative` | ✅ |
| `fileutil::relativeUrl` | ✅ |
| `fileutil::removeFromFile` | ✅ |
| `fileutil::replaceInFile` | ✅ |
| `fileutil::stripN` | ✅ |
| `fileutil::stripPwd` | ✅ |
| `fileutil::tempdir` | ✅ |
| `fileutil::tempdirReset` | ✅ |
| `fileutil::tempfile` | ✅ |
| `fileutil::test` | ✅ |
| `fileutil::touch` | ✅ |
| `fileutil::updateInPlace` | ✅ |
| `fileutil::writeFile` | ✅ |
| `html::html_entities` | ✅ |
| `html::tagstrip` | ✅ |
| `ip::collapse` | ✅ |
| `ip::contract` | ✅ |
| `ip::equal` | ✅ |
| `ip::is` | ✅ |
| `ip::mask` | ✅ |
| `ip::normalize` | ✅ |
| `ip::prefix` | ✅ |
| `ip::subtract` | ✅ |
| `ip::type` | ✅ |
| `ip::version` | ✅ |
| `json::dict2json` | ✅ |
| `json::json2dict` | ✅ |
| `json::list2json` | ✅ |
| `json::many-json2dict` | ✅ |
| `json::string2json` | ✅ |
| `json::validate` | ✅ |
| `logger::disable` | ✅ |
| `logger::enable` | ✅ |
| `logger::import` | ✅ |
| `logger::init` | ✅ |
| `logger::initNamespace` | ✅ |
| `logger::levels` | ✅ |
| `logger::servicecmd` | ✅ |
| `logger::services` | ✅ |
| `logger::setlevel` | ✅ |
| `logger::walk` | ✅ |
| `math::statistics::analyse-Kruskal-Wallis` | ✅ |
| `math::statistics::autocorr` | ✅ |
| `math::statistics::basic-stats` | ✅ |
| `math::statistics::control-Rchart` | ✅ |
| `math::statistics::control-xbar` | ✅ |
| `math::statistics::corr` | ✅ |
| `math::statistics::crosscorr` | ✅ |
| `math::statistics::filter` | ✅ |
| `math::statistics::group-rank` | ✅ |
| `math::statistics::histogram` | ✅ |
| `math::statistics::histogram-alt` | ✅ |
| `math::statistics::interval-mean-stdev` | ✅ |
| `math::statistics::lillieforsFit` | ✅ |
| `math::statistics::linear-model` | ✅ |
| `math::statistics::linear-residuals` | ✅ |
| `math::statistics::map` | ✅ |
| `math::statistics::max` | ✅ |
| `math::statistics::mean` | ✅ |
| `math::statistics::mean-histogram-limits` | ✅ |
| `math::statistics::median` | ✅ |
| `math::statistics::min` | ✅ |
| `math::statistics::minmax-histogram-limits` | ✅ |
| `math::statistics::number` | ✅ |
| `math::statistics::print-2x2` | ✅ |
| `math::statistics::pstdev` | ✅ |
| `math::statistics::pvar` | ✅ |
| `math::statistics::quantiles` | ✅ |
| `math::statistics::samplescount` | ✅ |
| `math::statistics::spearman-rank` | ✅ |
| `math::statistics::spearman-rank-extended` | ✅ |
| `math::statistics::stdev` | ✅ |
| `math::statistics::t-test-mean` | ✅ |
| `math::statistics::test-2x2` | ✅ |
| `math::statistics::test-Duckworth` | ✅ |
| `math::statistics::test-Dunnett` | ✅ |
| `math::statistics::test-Kruskal-Wallis` | ✅ |
| `math::statistics::test-Rchart` | ✅ |
| `math::statistics::test-Tukey-range` | ✅ |
| `math::statistics::test-Wilcoxon` | ✅ |
| `math::statistics::test-anova-F` | ✅ |
| `math::statistics::test-normal` | ✅ |
| `math::statistics::test-xbar` | ✅ |
| `math::statistics::var` | ✅ |
| `md5::md5` | ✅ |
| `mime::buildmessage` | ✅ |
| `mime::copymessage` | ✅ |
| `mime::field_decode` | ✅ |
| `mime::finalize` | ✅ |
| `mime::getContentType` | ✅ |
| `mime::getTransferEncoding` | ✅ |
| `mime::getbody` | ✅ |
| `mime::getheader` | ✅ |
| `mime::getproperty` | ✅ |
| `mime::getsize` | ✅ |
| `mime::initialize` | ✅ |
| `mime::mapencoding` | ✅ |
| `mime::parseaddress` | ✅ |
| `mime::parsedatetime` | ✅ |
| `mime::reversemapencoding` | ✅ |
| `mime::setheader` | ✅ |
| `mime::uniqueID` | ✅ |
| `mime::word_decode` | ✅ |
| `mime::word_encode` | ✅ |
| `sha1::sha1` | ✅ |
| `sha2::sha256` | ✅ |
| `smtp::sendmessage` | ✅ |
| `snit::compile` | ✅ |
| `snit::macro` | ✅ |
| `snit::method` | ✅ |
| `snit::type` | ✅ |
| `snit::typemethod` | ✅ |
| `snit::widget` | ✅ |
| `snit::widgetadaptor` | ✅ |
| `struct::list` | ✅ |
| `struct::queue` | ✅ |
| `struct::set` | ✅ |
| `struct::stack` | ✅ |
| `textutil::adjust` | ✅ |
| `textutil::blank` | ✅ |
| `textutil::cap` | ✅ |
| `textutil::capEachWord` | ✅ |
| `textutil::chop` | ✅ |
| `textutil::indent` | ✅ |
| `textutil::longestCommonPrefix` | ✅ |
| `textutil::longestCommonPrefixList` | ✅ |
| `textutil::splitn` | ✅ |
| `textutil::splitx` | ✅ |
| `textutil::strRepeat` | ✅ |
| `textutil::tabify` | ✅ |
| `textutil::tabify2` | ✅ |
| `textutil::tail` | ✅ |
| `textutil::trim` | ✅ |
| `textutil::trimEmptyHeading` | ✅ |
| `textutil::trimPrefix` | ✅ |
| `textutil::trimleft` | ✅ |
| `textutil::trimright` | ✅ |
| `textutil::uncap` | ✅ |
| `textutil::undent` | ✅ |
| `textutil::untabify` | ✅ |
| `textutil::untabify2` | ✅ |
| `uri::canonicalize` | ✅ |
| `uri::geturl` | ✅ |
| `uri::isrelative` | ✅ |
| `uri::join` | ✅ |
| `uri::register` | ✅ |
| `uri::resolve` | ✅ |
| `uri::setQuirkOption` | ✅ |
| `uri::split` | ✅ |
| `uuid::uuid` | ✅ |
| `yaml::dict2yaml` | ✅ |
| `yaml::huddle2yaml` | ✅ |
| `yaml::list2yaml` | ✅ |
| `yaml::setOptions` | ✅ |
| `yaml::yaml2dict` | ✅ |
| `yaml::yaml2huddle` | ✅ |

</details>

<details><summary><b>irules</b> — 1015 entries · 1012 ✅ · 3 need work</summary>

| entry | status |
|---|---|
| `AAA::acct_result` | ✅ |
| `AAA::acct_send` | ✅ |
| `AAA::auth_result` | ✅ |
| `AAA::auth_send` | ✅ |
| `ACCESS2::access2_proc` | ✅ |
| `ACCESS::acl` | ✅ |
| `ACCESS::disable` | ✅ |
| `ACCESS::enable` | ✅ |
| `ACCESS::ephemeral-auth` | ✅ |
| `ACCESS::flowid` | ✅ |
| `ACCESS::log` | ✅ |
| `ACCESS::oauth` | ✅ |
| `ACCESS::perflow` | ✅ |
| `ACCESS::policy` | ✅ |
| `ACCESS::respond` | ✅ |
| `ACCESS::restrict_irule_events` | ✅ |
| `ACCESS::saml` | ✅ |
| `ACCESS::session` | ✅ |
| `ACCESS::user` | ✅ |
| `ACCESS::uuid` | ✅ |
| `ACL::action` | ✅ |
| `ACL::eval` | ✅ |
| `ADAPT::allow` | ✅ |
| `ADAPT::context_create` | ✅ |
| `ADAPT::context_current` | ✅ |
| `ADAPT::context_delete_all` | ✅ |
| `ADAPT::context_name` | ✅ |
| `ADAPT::context_static` | ✅ |
| `ADAPT::enable` | ✅ |
| `ADAPT::preview_size` | ✅ |
| `ADAPT::result` | ✅ |
| `ADAPT::select` | ✅ |
| `ADAPT::service_down_action` | ✅ |
| `ADAPT::timeout` | ✅ |
| `AES::decrypt` | ✅ |
| `AES::encrypt` | ✅ |
| `AES::key` | ✅ |
| `AM::age` | ✅ |
| `AM::application` | ✅ |
| `AM::cache` | ✅ |
| `AM::disable` | ✅ |
| `AM::expires` | ✅ |
| `AM::media_playlist` | ✅ |
| `AM::policy_node` | ✅ |
| `ANTIFRAUD::alert_additional_info` | ✅ |
| `ANTIFRAUD::alert_bait_signatures` | ✅ |
| `ANTIFRAUD::alert_component` | ✅ |
| `ANTIFRAUD::alert_defined_value` | ✅ |
| `ANTIFRAUD::alert_details` | ✅ |
| `ANTIFRAUD::alert_device_id` | ✅ |
| `ANTIFRAUD::alert_expected_value` | ✅ |
| `ANTIFRAUD::alert_fingerprint` | ✅ |
| `ANTIFRAUD::alert_forbidden_added_element` | ✅ |
| `ANTIFRAUD::alert_guid` | ✅ |
| `ANTIFRAUD::alert_html` | ✅ |
| `ANTIFRAUD::alert_http_referrer` | ✅ |
| `ANTIFRAUD::alert_id` | ✅ |
| `ANTIFRAUD::alert_license_id` | ✅ |
| `ANTIFRAUD::alert_min` | ✅ |
| `ANTIFRAUD::alert_origin` | ✅ |
| `ANTIFRAUD::alert_resolved_value` | ✅ |
| `ANTIFRAUD::alert_score` | ✅ |
| `ANTIFRAUD::alert_transaction_data` | ✅ |
| `ANTIFRAUD::alert_transaction_id` | ✅ |
| `ANTIFRAUD::alert_type` | ✅ |
| `ANTIFRAUD::alert_username` | ✅ |
| `ANTIFRAUD::alert_view_id` | ✅ |
| `ANTIFRAUD::client_id` | ✅ |
| `ANTIFRAUD::device_id` | ✅ |
| `ANTIFRAUD::disable` | ✅ |
| `ANTIFRAUD::disable_alert` | ✅ |
| `ANTIFRAUD::disable_app_layer_encryption` | ✅ |
| `ANTIFRAUD::disable_auto_transactions` | ✅ |
| `ANTIFRAUD::disable_injection` | ✅ |
| `ANTIFRAUD::disable_malware` | ✅ |
| `ANTIFRAUD::disable_phishing` | ✅ |
| `ANTIFRAUD::enable` | ✅ |
| `ANTIFRAUD::enable_log` | ✅ |
| `ANTIFRAUD::fingerprint` | ✅ |
| `ANTIFRAUD::geo` | ✅ |
| `ANTIFRAUD::guid` | ✅ |
| `ANTIFRAUD::result` | ✅ |
| `ANTIFRAUD::username` | ✅ |
| `ASM::captcha` | ✅ |
| `ASM::captcha_age` | ✅ |
| `ASM::captcha_status` | ✅ |
| `ASM::client_ip` | ✅ |
| `ASM::conviction` | ✅ |
| `ASM::deception` | ✅ |
| `ASM::disable` | ✅ |
| `ASM::enable` | ✅ |
| `ASM::fingerprint` | ✅ |
| `ASM::is_authenticated` | ✅ |
| `ASM::login_status` | ✅ |
| `ASM::microservice` | ✅ |
| `ASM::payload` | ✅ |
| `ASM::policy` | ✅ |
| `ASM::raise` | ✅ |
| `ASM::severity` | ✅ |
| `ASM::signature` | ✅ |
| `ASM::status` | ✅ |
| `ASM::support_id` | ✅ |
| `ASM::threat_campaign` | ✅ |
| `ASM::unblock` | ✅ |
| `ASM::uncaptcha` | ✅ |
| `ASM::username` | ✅ |
| `ASM::violation` | ✅ |
| `ASM::violation_data` | ✅ |
| `ASN1::decode` | ✅ |
| `ASN1::element` | ✅ |
| `ASN1::encode` | ✅ |
| `AUTH::abort` | ✅ |
| `AUTH::authenticate` | ✅ |
| `AUTH::authenticate_continue` | ✅ |
| `AUTH::cert_credential` | ✅ |
| `AUTH::cert_issuer_credential` | ✅ |
| `AUTH::last_event_session_id` | ✅ |
| `AUTH::password_credential` | ✅ |
| `AUTH::response_data` | ✅ |
| `AUTH::ssl_cc_ldap_status` | ✅ |
| `AUTH::ssl_cc_ldap_username` | ✅ |
| `AUTH::start` | ✅ |
| `AUTH::status` | ✅ |
| `AUTH::subscribe` | ✅ |
| `AUTH::unsubscribe` | ✅ |
| `AUTH::username_credential` | ✅ |
| `AUTH::wantcredential_prompt` | ✅ |
| `AUTH::wantcredential_prompt_style` | ✅ |
| `AUTH::wantcredential_type` | ✅ |
| `AVR::disable` | ✅ |
| `AVR::disable_cspm_injection` | ✅ |
| `AVR::enable` | ✅ |
| `AVR::log` | ✅ |
| `BIGPROTO::enable_fix_reset` | ✅ |
| `BIGTCP::release_flow` | ✅ |
| `BOTDEFENSE::action` | ✅ |
| `BOTDEFENSE::bot_anomalies` | ✅ |
| `BOTDEFENSE::bot_categories` | ✅ |
| `BOTDEFENSE::bot_name` | ✅ |
| `BOTDEFENSE::bot_signature` | ✅ |
| `BOTDEFENSE::bot_signature_category` | ✅ |
| `BOTDEFENSE::captcha_age` | ✅ |
| `BOTDEFENSE::captcha_status` | ✅ |
| `BOTDEFENSE::client_class` | ✅ |
| `BOTDEFENSE::client_type` | ✅ |
| `BOTDEFENSE::cookie_age` | ✅ |
| `BOTDEFENSE::cookie_status` | ✅ |
| `BOTDEFENSE::cs_allowed` | ✅ |
| `BOTDEFENSE::cs_attribute` | ✅ |
| `BOTDEFENSE::cs_possible` | ✅ |
| `BOTDEFENSE::device_id` | ✅ |
| `BOTDEFENSE::disable` | ✅ |
| `BOTDEFENSE::enable` | ✅ |
| `BOTDEFENSE::intent` | ✅ |
| `BOTDEFENSE::micro_service` | ✅ |
| `BOTDEFENSE::previous_action` | ✅ |
| `BOTDEFENSE::previous_request_age` | ✅ |
| `BOTDEFENSE::previous_support_id` | ✅ |
| `BOTDEFENSE::reason` | ✅ |
| `BOTDEFENSE::support_id` | ✅ |
| `BWC::color` | ✅ |
| `BWC::debug` | ✅ |
| `BWC::mark` | ✅ |
| `BWC::measure` | ✅ |
| `BWC::policy` | ✅ |
| `BWC::pps` | ✅ |
| `BWC::priority` | ✅ |
| `BWC::rate` | ✅ |
| `CACHE::accept_encoding` | ✅ |
| `CACHE::age` | ✅ |
| `CACHE::disable` | ✅ |
| `CACHE::disabled` | ✅ |
| `CACHE::enable` | ✅ |
| `CACHE::expire` | ✅ |
| `CACHE::fresh` | ✅ |
| `CACHE::header` | ✅ |
| `CACHE::headers` | ✅ |
| `CACHE::hits` | ✅ |
| `CACHE::payload` | ✅ |
| `CACHE::priority` | ✅ |
| `CACHE::statskey` | ✅ |
| `CACHE::trace` | ✅ |
| `CACHE::uri` | ✅ |
| `CACHE::useragent` | ✅ |
| `CACHE::userkey` | ✅ |
| `CATEGORY::analytics` | ✅ |
| `CATEGORY::filetype` | ✅ |
| `CATEGORY::lookup` | ✅ |
| `CATEGORY::matchtype` | ✅ |
| `CATEGORY::result` | ✅ |
| `CATEGORY::safesearch` | ✅ |
| `CLASSIFICATION::app` | ✅ |
| `CLASSIFICATION::category` | ✅ |
| `CLASSIFICATION::disable` | ✅ |
| `CLASSIFICATION::enable` | ✅ |
| `CLASSIFICATION::protocol` | ✅ |
| `CLASSIFICATION::result` | ✅ |
| `CLASSIFICATION::urlcat` | ✅ |
| `CLASSIFICATION::username` | ✅ |
| `CLASSIFY::application` | ✅ |
| `CLASSIFY::category` | ✅ |
| `CLASSIFY::defer` | ✅ |
| `CLASSIFY::disable` | ✅ |
| `CLASSIFY::urlcat` | ✅ |
| `CLASSIFY::username` | ✅ |
| `COMPRESS::buffer_size` | ✅ |
| `COMPRESS::disable` | ✅ |
| `COMPRESS::enable` | ✅ |
| `COMPRESS::gzip` | ✅ |
| `COMPRESS::method` | ✅ |
| `COMPRESS::nodelay` | ✅ |
| `CONNECTOR::disable` | ✅ |
| `CONNECTOR::enable` | ✅ |
| `CONNECTOR::profile` | ✅ |
| `CONNECTOR::remap` | ✅ |
| `CRYPTO::decrypt` | ✅ |
| `CRYPTO::encrypt` | ✅ |
| `CRYPTO::hash` | ✅ |
| `CRYPTO::keygen` | ✅ |
| `CRYPTO::sign` | ✅ |
| `CRYPTO::verify` | ✅ |
| `DATAGRAM::dns` | ✅ |
| `DATAGRAM::ip` | ✅ |
| `DATAGRAM::ip6` | ✅ |
| `DATAGRAM::l2` | ✅ |
| `DATAGRAM::tcp` | ✅ |
| `DATAGRAM::udp` | ✅ |
| `DECOMPRESS::disable` | ✅ |
| `DECOMPRESS::enable` | ✅ |
| `DEMANGLE::disable` | ✅ |
| `DEMANGLE::enable` | ✅ |
| `DHCP::version` | ✅ |
| `DHCPv4::chaddr` | ✅ |
| `DHCPv4::ciaddr` | ✅ |
| `DHCPv4::drop` | ✅ |
| `DHCPv4::giaddr` | ✅ |
| `DHCPv4::hlen` | ✅ |
| `DHCPv4::hops` | ✅ |
| `DHCPv4::htype` | ✅ |
| `DHCPv4::len` | ✅ |
| `DHCPv4::opcode` | ✅ |
| `DHCPv4::option` | ✅ |
| `DHCPv4::reject` | ✅ |
| `DHCPv4::secs` | ✅ |
| `DHCPv4::siaddr` | ✅ |
| `DHCPv4::type` | ✅ |
| `DHCPv4::xid` | ✅ |
| `DHCPv4::yiaddr` | ✅ |
| `DHCPv6::drop` | ✅ |
| `DHCPv6::hop_count` | ✅ |
| `DHCPv6::len` | ✅ |
| `DHCPv6::link_address` | ✅ |
| `DHCPv6::msg_type` | ✅ |
| `DHCPv6::option` | ✅ |
| `DHCPv6::peer_address` | ✅ |
| `DHCPv6::reject` | ✅ |
| `DHCPv6::transaction_id` | ✅ |
| `DIAG::test` | ✅ |
| `DIAMETER::avp` | ✅ |
| `DIAMETER::command` | ✅ |
| `DIAMETER::disconnect` | ✅ |
| `DIAMETER::drop` | ✅ |
| `DIAMETER::dynamic_route_insertion` | ✅ |
| `DIAMETER::dynamic_route_lookup` | ✅ |
| `DIAMETER::header` | ✅ |
| `DIAMETER::host` | ✅ |
| `DIAMETER::is_request` | ✅ |
| `DIAMETER::is_response` | ✅ |
| `DIAMETER::is_retransmission` | ✅ |
| `DIAMETER::length` | ✅ |
| `DIAMETER::message` | ✅ |
| `DIAMETER::payload` | ✅ |
| `DIAMETER::persist` | ✅ |
| `DIAMETER::realm` | ✅ |
| `DIAMETER::respond` | ✅ |
| `DIAMETER::result` | ✅ |
| `DIAMETER::retransmission` | ✅ |
| `DIAMETER::retransmission_default` | ✅ |
| `DIAMETER::retransmission_reason` | ✅ |
| `DIAMETER::retransmit` | ✅ |
| `DIAMETER::retry` | ✅ |
| `DIAMETER::route_status` | ✅ |
| `DIAMETER::session` | ✅ |
| `DIAMETER::skip_capabilities_exchange` | ✅ |
| `DIAMETER::state` | ✅ |
| `DNS::additional` | ✅ |
| `DNS::answer` | ✅ |
| `DNS::authority` | ✅ |
| `DNS::class` | ✅ |
| `DNS::disable` | ✅ |
| `DNS::drop` | ✅ |
| `DNS::edns0` | ✅ |
| `DNS::enable` | ✅ |
| `DNS::header` | ✅ |
| `DNS::is_wideip` | ✅ |
| `DNS::last_act` | ✅ |
| `DNS::len` | ✅ |
| `DNS::log` | ✅ |
| `DNS::name` | ✅ |
| `DNS::origin` | ✅ |
| `DNS::ptype` | ✅ |
| `DNS::query` | ✅ |
| `DNS::question` | ✅ |
| `DNS::rdata` | ✅ |
| `DNS::return` | ✅ |
| `DNS::rpz_policy` | ✅ |
| `DNS::rr` | ✅ |
| `DNS::scrape` | ✅ |
| `DNS::tsig` | ✅ |
| `DNS::ttl` | ✅ |
| `DNS::type` | ✅ |
| `DNSMSG::header` | ✅ |
| `DNSMSG::record` | ✅ |
| `DNSMSG::section` | ✅ |
| `DOSL7::disable` | ✅ |
| `DOSL7::enable` | ✅ |
| `DOSL7::health` | ✅ |
| `DOSL7::is_ip_slowdown` | ✅ |
| `DOSL7::is_mitigated` | ✅ |
| `DOSL7::profile` | ✅ |
| `DOSL7::slowdown` | ✅ |
| `DSLITE::remote_addr` | ✅ |
| `ECA::client_machine_name` | ✅ |
| `ECA::disable` | ✅ |
| `ECA::domainname` | ✅ |
| `ECA::enable` | ✅ |
| `ECA::select` | ✅ |
| `ECA::status` | ✅ |
| `ECA::username` | ✅ |
| `FIX::tag` | ✅ |
| `FLOW::create_related` | ✅ |
| `FLOW::idle_duration` | ✅ |
| `FLOW::idle_timeout` | ✅ |
| `FLOW::peer` | ✅ |
| `FLOW::priority` | ✅ |
| `FLOW::refresh` | ✅ |
| `FLOW::this` | ✅ |
| `FLOWTABLE::count` | ✅ |
| `FLOWTABLE::limit` | ✅ |
| `FTP::allow_active_mode` | ✅ |
| `FTP::disable` | ✅ |
| `FTP::enable` | ✅ |
| `FTP::enforce_tls_session_reuse` | ✅ |
| `FTP::ftps_mode` | ✅ |
| `FTP::port` | ✅ |
| `GENERICMESSAGE::message` | ✅ |
| `GENERICMESSAGE::peer` | ✅ |
| `GENERICMESSAGE::route` | ✅ |
| `GTP::clone` | ✅ |
| `GTP::discard` | ✅ |
| `GTP::forward` | ✅ |
| `GTP::header` | ✅ |
| `GTP::ie` | ✅ |
| `GTP::length` | ✅ |
| `GTP::message` | ✅ |
| `GTP::new` | ✅ |
| `GTP::parse` | ✅ |
| `GTP::payload` | ✅ |
| `GTP::respond` | ✅ |
| `GTP::tunnel` | ✅ |
| `HA::status` | ✅ |
| `HSL::open` | ✅ |
| `HSL::send` | ✅ |
| `HTML::comment` | ✅ |
| `HTML::disable` | ✅ |
| `HTML::enable` | ✅ |
| `HTML::encode` | ✅ |
| `HTML::tag` | ✅ |
| `HTTP2::active` | ✅ |
| `HTTP2::concurrency` | ✅ |
| `HTTP2::disable` | ✅ |
| `HTTP2::disconnect` | ✅ |
| `HTTP2::enable` | ✅ |
| `HTTP2::header` | ✅ |
| `HTTP2::push` | ✅ |
| `HTTP2::requests` | ✅ |
| `HTTP2::stream` | ✅ |
| `HTTP2::version` | ✅ |
| `HTTP::class` | ✅ |
| `HTTP::close` | ✅ |
| `HTTP::collect` | ✅ |
| `HTTP::cookie` | ✅ |
| `HTTP::disable` | ✅ |
| `HTTP::enable` | ✅ |
| `HTTP::fallback` | ✅ |
| `HTTP::has_responded` | ✅ |
| `HTTP::header` | ✅ |
| `HTTP::host` | ✅ |
| `HTTP::hsts` | ✅ |
| `HTTP::is_keepalive` | ✅ |
| `HTTP::is_redirect` | ✅ |
| `HTTP::method` | ✅ |
| `HTTP::passthrough_reason` | ✅ |
| `HTTP::password` | ✅ |
| `HTTP::path` | ✅ |
| `HTTP::payload` | ✅ |
| `HTTP::proxy` | ✅ |
| `HTTP::query` | ✅ |
| `HTTP::redirect` | ✅ |
| `HTTP::reject_reason` | ✅ |
| `HTTP::release` | ✅ |
| `HTTP::request` | ✅ |
| `HTTP::request_num` | ✅ |
| `HTTP::respond` | ✅ |
| `HTTP::response` | ✅ |
| `HTTP::retry` | ✅ |
| `HTTP::status` | ✅ |
| `HTTP::uri` | ✅ |
| `HTTP::username` | ✅ |
| `HTTP::version` | ✅ |
| `HTTPLOG::disable` | ✅ |
| `HTTPLOG::enable` | ✅ |
| `ICAP::header` | ✅ |
| `ICAP::method` | ✅ |
| `ICAP::status` | ✅ |
| `ICAP::uri` | ✅ |
| `IKE::auth_success` | ✅ |
| `IKE::cert` | ✅ |
| `IKE::san_dirname` | ✅ |
| `IKE::san_dns` | ✅ |
| `IKE::san_ediparty` | ✅ |
| `IKE::san_email` | ✅ |
| `IKE::san_ipadd` | ✅ |
| `IKE::san_othername` | ✅ |
| `IKE::san_rid` | ✅ |
| `IKE::san_uri` | ✅ |
| `IKE::san_x400` | ✅ |
| `IKE::subjectAltName` | ✅ |
| `ILX::call` | ✅ |
| `ILX::init` | ✅ |
| `ILX::notify` | ✅ |
| `IMAP::activation_mode` | ✅ |
| `IMAP::disable` | ✅ |
| `IMAP::enable` | ✅ |
| `IP::addr` | ✅ |
| `IP::client_addr` | ✅ |
| `IP::hops` | ✅ |
| `IP::idle_timeout` | ✅ |
| `IP::ingress_drop_rate` | ✅ |
| `IP::ingress_rate_limit` | ✅ |
| `IP::intelligence` | ✅ |
| `IP::local_addr` | ✅ |
| `IP::protocol` | ✅ |
| `IP::remote_addr` | ✅ |
| `IP::reputation` | ✅ |
| `IP::server_addr` | ✅ |
| `IP::stats` | ✅ |
| `IP::tos` | ✅ |
| `IP::ttl` | ✅ |
| `IP::version` | ✅ |
| `IPFIX::destination` | ✅ |
| `IPFIX::msg` | ✅ |
| `IPFIX::template` | ✅ |
| `ISESSION::deduplication` | ✅ |
| `ISTATS::get` | ✅ |
| `ISTATS::incr` | ✅ |
| `ISTATS::remove` | ✅ |
| `ISTATS::set` | ✅ |
| `IVS_ENTRY::result` | ✅ |
| `JSON::array` | ✅ |
| `JSON::create` | ✅ |
| `JSON::get` | ✅ |
| `JSON::object` | ✅ |
| `JSON::parse` | ✅ |
| `JSON::render` | ✅ |
| `JSON::root` | ✅ |
| `JSON::set` | ✅ |
| `JSON::type` | ✅ |
| `L7CHECK::protocol` | ✅ |
| `LB::bias` | ✅ |
| `LB::class` | ✅ |
| `LB::command` | ✅ |
| `LB::connect` | ✅ |
| `LB::connlimit` | ✅ |
| `LB::context_id` | ✅ |
| `LB::detach` | ✅ |
| `LB::down` | ✅ |
| `LB::dst_tag` | ✅ |
| `LB::enable_decisionlog` | ✅ |
| `LB::mode` | ✅ |
| `LB::persist` | ✅ |
| `LB::prime` | ✅ |
| `LB::queue` | ✅ |
| `LB::reselect` | ✅ |
| `LB::select` | ✅ |
| `LB::server` | ✅ |
| `LB::snat` | ✅ |
| `LB::src_tag` | ✅ |
| `LB::status` | ✅ |
| `LB::up` | ✅ |
| `LDAP::activation_mode` | ✅ |
| `LDAP::disable` | ✅ |
| `LDAP::enable` | ✅ |
| `LINE::get` | ✅ |
| `LINE::set` | ✅ |
| `LINK::lasthop` | ✅ |
| `LINK::nexthop` | ✅ |
| `LINK::qos` | ✅ |
| `LINK::vlan_id` | ✅ |
| `LSN::address` | ✅ |
| `LSN::disable` | ✅ |
| `LSN::inbound` | ✅ |
| `LSN::inbound-entry` | ✅ |
| `LSN::persistence` | ✅ |
| `LSN::persistence-entry` | ✅ |
| `LSN::pool` | ✅ |
| `LSN::port` | ✅ |
| `MESSAGE::field` | ✅ |
| `MESSAGE::proto` | ✅ |
| `MESSAGE::type` | ✅ |
| `MQTT::clean_session` | ✅ |
| `MQTT::client_id` | ✅ |
| `MQTT::collect` | ✅ |
| `MQTT::disable` | ✅ |
| `MQTT::disconnect` | ✅ |
| `MQTT::drop` | ✅ |
| `MQTT::dup` | ✅ |
| `MQTT::enable` | ✅ |
| `MQTT::insert` | ✅ |
| `MQTT::keep_alive` | ✅ |
| `MQTT::length` | ✅ |
| `MQTT::message` | ✅ |
| `MQTT::packet_id` | ✅ |
| `MQTT::password` | ✅ |
| `MQTT::payload` | ✅ |
| `MQTT::protocol_name` | ✅ |
| `MQTT::protocol_version` | ✅ |
| `MQTT::qos` | ✅ |
| `MQTT::release` | ✅ |
| `MQTT::replace` | ✅ |
| `MQTT::respond` | ✅ |
| `MQTT::retain` | ✅ |
| `MQTT::return_code` | ✅ |
| `MQTT::return_code_list` | ✅ |
| `MQTT::session_present` | ✅ |
| `MQTT::topic` | ✅ |
| `MQTT::type` | ✅ |
| `MQTT::username` | ✅ |
| `MQTT::will` | ✅ |
| `MR::always_match_port` | ✅ |
| `MR::available_for_routing` | ✅ |
| `MR::collect` | ✅ |
| `MR::connect_back_port` | ✅ |
| `MR::connection_instance` | ✅ |
| `MR::connection_mode` | ✅ |
| `MR::equivalent_transport` | ✅ |
| `MR::flow_id` | ✅ |
| `MR::ignore_peer_port` | ✅ |
| `MR::instance` | ✅ |
| `MR::max_retries` | ✅ |
| `MR::message` | ✅ |
| `MR::payload` | ✅ |
| `MR::peer` | ✅ |
| `MR::prime` | ✅ |
| `MR::protocol` | ✅ |
| `MR::release` | ✅ |
| `MR::restore` | ✅ |
| `MR::retry` | ✅ |
| `MR::return` | ✅ |
| `MR::store` | ✅ |
| `MR::stream` | ✅ |
| `MR::transport` | ✅ |
| `NAME::lookup` | ✅ |
| `NAME::response` | ✅ |
| `NSH::chain` | ✅ |
| `NSH::context` | ✅ |
| `NSH::md1` | ✅ |
| `NSH::mocksf` | ✅ |
| `NSH::path_id` | ✅ |
| `NSH::service_index` | ✅ |
| `NTLM::disable` | ✅ |
| `NTLM::enable` | ✅ |
| `OFFBOX::request` | ✅ |
| `ONECONNECT::detach` | ✅ |
| `ONECONNECT::label` | ✅ |
| `ONECONNECT::reuse` | ✅ |
| `ONECONNECT::select` | ✅ |
| `PCP::reject` | ✅ |
| `PCP::request` | ✅ |
| `PCP::response` | ✅ |
| `PEM::disable` | ✅ |
| `PEM::enable` | ✅ |
| `PEM::flow` | ✅ |
| `PEM::session` | ✅ |
| `PEM::subscriber` | ✅ |
| `PLUGIN::disable` | ✅ |
| `PLUGIN::enable` | ✅ |
| `POLICY::controls` | ✅ |
| `POLICY::names` | ✅ |
| `POLICY::rules` | ✅ |
| `POLICY::targets` | ✅ |
| `POP3::activation_mode` | ✅ |
| `POP3::disable` | ✅ |
| `POP3::enable` | ✅ |
| `PROFILE::access` | ✅ |
| `PROFILE::antifraud` | ✅ |
| `PROFILE::auth` | ✅ |
| `PROFILE::avr` | ✅ |
| `PROFILE::clientssl` | ✅ |
| `PROFILE::diameter` | ✅ |
| `PROFILE::exchange` | ✅ |
| `PROFILE::exists` | ✅ |
| `PROFILE::fastL4` | ✅ |
| `PROFILE::fasthttp` | ✅ |
| `PROFILE::ftp` | ✅ |
| `PROFILE::http` | ✅ |
| `PROFILE::httpclass` | ✅ |
| `PROFILE::httpcompression` | ✅ |
| `PROFILE::list` | ✅ |
| `PROFILE::oneconnect` | ✅ |
| `PROFILE::persist` | ✅ |
| `PROFILE::serverssl` | ✅ |
| `PROFILE::stream` | ✅ |
| `PROFILE::tcp` | ✅ |
| `PROFILE::tftp` | ✅ |
| `PROFILE::udp` | ✅ |
| `PROFILE::vdi` | ✅ |
| `PROFILE::webacceleration` | ✅ |
| `PROFILE::xml` | ✅ |
| `PROTOCOL_INSPECTION::disable` | ✅ |
| `PROTOCOL_INSPECTION::id` | ✅ |
| `PSC::aaa_reporting_interval` | ✅ |
| `PSC::attr` | ✅ |
| `PSC::calling_id` | ✅ |
| `PSC::imeisv` | ✅ |
| `PSC::imsi` | ✅ |
| `PSC::ip_address` | ✅ |
| `PSC::lease_time` | ✅ |
| `PSC::policy` | ✅ |
| `PSC::subscriber_id` | ✅ |
| `PSC::tower_id` | ✅ |
| `PSC::user_name` | ✅ |
| `PSM::FTP::disable` | ✅ |
| `PSM::FTP::enable` | ✅ |
| `PSM::HTTP::disable` | ✅ |
| `PSM::HTTP::enable` | ✅ |
| `PSM::SMTP::disable` | ✅ |
| `PSM::SMTP::enable` | ✅ |
| `QOE::disable` | ✅ |
| `QOE::enable` | ✅ |
| `QOE::video` | ✅ |
| `RADIUS::avp` | ✅ |
| `RADIUS::code` | ✅ |
| `RADIUS::id` | ✅ |
| `RADIUS::rtdom` | ✅ |
| `RADIUS::subscriber` | ✅ |
| `RESOLV::lookup` | ✅ |
| `RESOLVER::name_lookup` | ✅ |
| `RESOLVER::summarize` | ✅ |
| `REST::send` | ✅ |
| `REWRITE::disable` | ✅ |
| `REWRITE::enable` | ✅ |
| `REWRITE::payload` | ✅ |
| `REWRITE::post_process` | ✅ |
| `ROUTE::age` | ✅ |
| `ROUTE::bandwidth` | ✅ |
| `ROUTE::clear` | ✅ |
| `ROUTE::cwnd` | ✅ |
| `ROUTE::domain` | ✅ |
| `ROUTE::expiration` | ✅ |
| `ROUTE::mtu` | ✅ |
| `ROUTE::rtt` | ✅ |
| `ROUTE::rttvar` | ✅ |
| `RTSP::collect` | ✅ |
| `RTSP::header` | ✅ |
| `RTSP::method` | ✅ |
| `RTSP::msg_source` | ✅ |
| `RTSP::payload` | ✅ |
| `RTSP::release` | ✅ |
| `RTSP::respond` | ✅ |
| `RTSP::status` | ✅ |
| `RTSP::uri` | ✅ |
| `RTSP::version` | ✅ |
| `SCTP::client_port` | ✅ |
| `SCTP::collect` | ✅ |
| `SCTP::local_port` | ✅ |
| `SCTP::mss` | ✅ |
| `SCTP::payload` | ✅ |
| `SCTP::ppi` | ✅ |
| `SCTP::release` | ✅ |
| `SCTP::remote_port` | ✅ |
| `SCTP::respond` | ✅ |
| `SCTP::rto_initial` | ✅ |
| `SCTP::rto_max` | ✅ |
| `SCTP::rto_min` | ✅ |
| `SCTP::sack_timeout` | ✅ |
| `SCTP::server_port` | ✅ |
| `SDP::field` | ✅ |
| `SDP::media` | ✅ |
| `SDP::session_id` | ✅ |
| `SIP::call_id` | ✅ |
| `SIP::discard` | ✅ |
| `SIP::from` | ✅ |
| `SIP::header` | ✅ |
| `SIP::message` | ✅ |
| `SIP::method` | ✅ |
| `SIP::payload` | ✅ |
| `SIP::persist` | ✅ |
| `SIP::record-route` | ✅ |
| `SIP::respond` | ✅ |
| `SIP::response` | ✅ |
| `SIP::route` | ✅ |
| `SIP::route_status` | ✅ |
| `SIP::to` | ✅ |
| `SIP::uri` | ✅ |
| `SIP::via` | ✅ |
| `SIPALG::hairpin` | ✅ |
| `SIPALG::hairpin_default` | ✅ |
| `SIPALG::nonregister_subscriber_listener` | ✅ |
| `SMTPS::activation_mode` | ✅ |
| `SMTPS::disable` | ✅ |
| `SMTPS::enable` | ✅ |
| `SOCKS::allowed` | ✅ |
| `SOCKS::destination` | ✅ |
| `SOCKS::version` | ✅ |
| `SSE::field` | ✅ |
| `SSL::allow_dynamic_record_sizing` | ✅ |
| `SSL::allow_nonssl` | ✅ |
| `SSL::alpn` | ✅ |
| `SSL::authenticate` | ✅ |
| `SSL::c3d` | ✅ |
| `SSL::cert` | ✅ |
| `SSL::cert_constraint` | ✅ |
| `SSL::cipher` | ✅ |
| `SSL::clientrandom` | ✅ |
| `SSL::collect` | ✅ |
| `SSL::disable` | ✅ |
| `SSL::enable` | ✅ |
| `SSL::extensions` | ✅ |
| `SSL::forward_proxy` | ✅ |
| `SSL::handshake` | ✅ |
| `SSL::is_renegotiation_secure` | ✅ |
| `SSL::maximum_record_size` | ✅ |
| `SSL::mode` | ✅ |
| `SSL::modssl_sessionid_headers` | ✅ |
| `SSL::nextproto` | ✅ |
| `SSL::payload` | ✅ |
| `SSL::profile` | ✅ |
| `SSL::release` | ✅ |
| `SSL::renegotiate` | ✅ |
| `SSL::respond` | ✅ |
| `SSL::secure_renegotiation` | ✅ |
| `SSL::session` | ✅ |
| `SSL::sessionid` | ✅ |
| `SSL::sessionsecret` | ✅ |
| `SSL::sessionticket` | ✅ |
| `SSL::sni` | ✅ |
| `SSL::tls13_secret` | ✅ |
| `SSL::unclean_shutdown` | ✅ |
| `SSL::verify_result` | ✅ |
| `STATS::get` | ✅ |
| `STATS::incr` | ✅ |
| `STATS::set` | ✅ |
| `STATS::setmax` | ✅ |
| `STATS::setmin` | ✅ |
| `STREAM::disable` | ✅ |
| `STREAM::enable` | ✅ |
| `STREAM::encoding` | ✅ |
| `STREAM::expression` | ✅ |
| `STREAM::match` | ✅ |
| `STREAM::max_matchsize` | ✅ |
| `STREAM::replace` | ✅ |
| `TAP::action` | ✅ |
| `TAP::config` | ✅ |
| `TAP::insight` | ✅ |
| `TAP::insight_requested` | ✅ |
| `TAP::score` | ✅ |
| `TCP::abc` | ✅ |
| `TCP::analytics` | ✅ |
| `TCP::autowin` | ✅ |
| `TCP::bandwidth` | ✅ |
| `TCP::client_port` | ✅ |
| `TCP::close` | ✅ |
| `TCP::collect` | ✅ |
| `TCP::congestion` | ✅ |
| `TCP::delayed_ack` | ✅ |
| `TCP::dsack` | ✅ |
| `TCP::earlyrxmit` | ✅ |
| `TCP::ecn` | ✅ |
| `TCP::enhanced_loss_recovery` | ✅ |
| `TCP::idletime` | ✅ |
| `TCP::keepalive` | ✅ |
| `TCP::limxmit` | ✅ |
| `TCP::local_port` | ✅ |
| `TCP::lossfilter` | ✅ |
| `TCP::lossfilterburst` | ✅ |
| `TCP::lossfilterrate` | ✅ |
| `TCP::mss` | ✅ |
| `TCP::nagle` | ✅ |
| `TCP::naglemode` | ✅ |
| `TCP::naglestate` | ✅ |
| `TCP::notify` | ✅ |
| `TCP::offset` | ✅ |
| `TCP::option` | ✅ |
| `TCP::pacing` | ✅ |
| `TCP::payload` | ✅ |
| `TCP::proxybuffer` | ✅ |
| `TCP::proxybufferhigh` | ✅ |
| `TCP::proxybufferlow` | ✅ |
| `TCP::push_flag` | ✅ |
| `TCP::rcv_scale` | ✅ |
| `TCP::rcv_size` | ✅ |
| `TCP::recvwnd` | ✅ |
| `TCP::release` | ✅ |
| `TCP::remote_port` | ✅ |
| `TCP::respond` | ✅ |
| `TCP::rexmt_thresh` | ✅ |
| `TCP::rt_metrics_timeout` | ✅ |
| `TCP::rto` | ✅ |
| `TCP::rtt` | ✅ |
| `TCP::rttvar` | ✅ |
| `TCP::sendbuf` | ✅ |
| `TCP::server_port` | ✅ |
| `TCP::setmss` | ✅ |
| `TCP::snd_cwnd` | ✅ |
| `TCP::snd_scale` | ✅ |
| `TCP::snd_ssthresh` | ✅ |
| `TCP::snd_wnd` | ✅ |
| `TCP::unused_port` | ✅ |
| `TDS::msg` | ✅ |
| `TDS::session` | ✅ |
| `TMM::cmp_count` | ✅ |
| `TMM::cmp_group` | ✅ |
| `TMM::cmp_groups` | ✅ |
| `TMM::cmp_primary_group` | ✅ |
| `TMM::cmp_unit` | ✅ |
| `UDP::client_port` | ✅ |
| `UDP::debug_queue` | ✅ |
| `UDP::drop` | ✅ |
| `UDP::hold` | ✅ |
| `UDP::local_port` | ✅ |
| `UDP::max_buf_pkts` | ✅ |
| `UDP::max_rate` | ✅ |
| `UDP::mss` | ✅ |
| `UDP::payload` | ✅ |
| `UDP::release` | ✅ |
| `UDP::remote_port` | ✅ |
| `UDP::respond` | ✅ |
| `UDP::sendbuffer` | ✅ |
| `UDP::server_port` | ✅ |
| `UDP::unused_port` | ✅ |
| `URI::basename` | ✅ |
| `URI::compare` | ✅ |
| `URI::decode` | ✅ |
| `URI::encode` | ✅ |
| `URI::encode_component` | ✅ |
| `URI::escape` | ✅ |
| `URI::host` | ✅ |
| `URI::path` | ✅ |
| `URI::port` | ✅ |
| `URI::protocol` | ✅ |
| `URI::query` | ✅ |
| `VALIDATE::protocol` | ✅ |
| `VDI::disable` | ✅ |
| `VDI::enable` | ✅ |
| `WAM::disable` | ✅ |
| `WAM::enable` | ✅ |
| `WEBSSO::disable` | ✅ |
| `WEBSSO::enable` | ✅ |
| `WEBSSO::select` | ✅ |
| `WS::collect` | ✅ |
| `WS::disconnect` | ✅ |
| `WS::enabled` | ✅ |
| `WS::frame` | ✅ |
| `WS::masking` | ✅ |
| `WS::message` | ✅ |
| `WS::payload` | ✅ |
| `WS::payload_ivs` | ✅ |
| `WS::payload_processing` | ✅ |
| `WS::release` | ✅ |
| `WS::request` | ✅ |
| `WS::response` | ✅ |
| `X509::cert_fields` | ✅ |
| `X509::extensions` | ✅ |
| `X509::hash` | ✅ |
| `X509::issuer` | ✅ |
| `X509::not_valid_after` | ✅ |
| `X509::not_valid_before` | ✅ |
| `X509::pem2der` | ✅ |
| `X509::serial_number` | ✅ |
| `X509::signature_algorithm` | ✅ |
| `X509::subject` | ✅ |
| `X509::subject_public_key` | ✅ |
| `X509::subject_public_key_RSA_bits` | ✅ |
| `X509::subject_public_key_type` | ✅ |
| `X509::verify_cert_error_string` | ✅ |
| `X509::version` | ✅ |
| `X509::whole` | ✅ |
| `XLAT::listen` | ✅ |
| `XLAT::listen_lifetime` | ✅ |
| `XLAT::src_addr` | ✅ |
| `XLAT::src_config` | ✅ |
| `XLAT::src_endpoint_reservation` | ✅ |
| `XLAT::src_nat_valid_range` | ✅ |
| `XLAT::src_port` | ✅ |
| `XML::address` | ✅ |
| `XML::collect` | ✅ |
| `XML::disable` | ✅ |
| `XML::element` | ✅ |
| `XML::enable` | ✅ |
| `XML::event` | ✅ |
| `XML::eventid` | ✅ |
| `XML::parse` | ✅ |
| `XML::payload` | ✅ |
| `XML::release` | ✅ |
| `XML::soap` | ✅ |
| `XML::subscribe` | ✅ |
| `accumulate` | ✅ |
| `active_members` | ✅ |
| `active_nodes` | ✅ |
| `after` | ✅ |
| `b64decode` | ✅ |
| `b64encode` | ✅ |
| `call` | ✅ |
| `check` | ✅ |
| `class` | ✅ |
| `client_addr` | ✅ |
| `client_port` | ✅ |
| `clientside` | `arity≠` |
| `clone` | ✅ |
| `close` | ✅ |
| `connect` | ✅ |
| `cpu` | ✅ |
| `crc32` | ✅ |
| `decode_uri` | ✅ |
| `discard` | ✅ |
| `domain` | ✅ |
| `drop` | ✅ |
| `event` | ✅ |
| `fasthash` | ✅ |
| `findclass` | ✅ |
| `findstr` | ✅ |
| `forward` | ✅ |
| `getfield` | ✅ |
| `html_encode` | ✅ |
| `html_escape` | ✅ |
| `htmlencode` | ✅ |
| `htonl` | ✅ |
| `htons` | ✅ |
| `http_client_ip` | ✅ |
| `http_content_len_max` | ✅ |
| `http_cookie` | ✅ |
| `http_header` | ✅ |
| `http_host` | ✅ |
| `http_method` | ✅ |
| `http_uri` | ✅ |
| `http_version` | ✅ |
| `ifile` | ✅ |
| `imid` | ✅ |
| `ip_addr` | ✅ |
| `ip_protocol` | ✅ |
| `ip_tos` | ✅ |
| `ip_ttl` | ✅ |
| `lasthop` | ✅ |
| `link_qos` | ✅ |
| `listen` | ✅ |
| `llookup` | ✅ |
| `local_addr` | ✅ |
| `local_port` | ✅ |
| `log` | ✅ |
| `matchclass` | ✅ |
| `md4` | ✅ |
| `md5` | ✅ |
| `members` | ✅ |
| `nexthop` | ✅ |
| `node` | ✅ |
| `nodes` | ✅ |
| `ntohl` | ✅ |
| `ntohs` | ✅ |
| `peer` | `arity≠` |
| `pem_dtos` | ✅ |
| `persist` | ✅ |
| `pool` | ✅ |
| `priority` | ✅ |
| `proc` | ✅ |
| `radius_authenticate` | ✅ |
| `rateclass` | ✅ |
| `recv` | ✅ |
| `redirect` | ✅ |
| `reject` | ✅ |
| `relate_client` | ✅ |
| `relate_server` | ✅ |
| `remote_addr` | ✅ |
| `remote_port` | ✅ |
| `rmd160` | ✅ |
| `send` | ✅ |
| `server_addr` | ✅ |
| `server_port` | ✅ |
| `serverside` | `arity≠` |
| `session` | ✅ |
| `sha1` | ✅ |
| `sha256` | ✅ |
| `sha384` | ✅ |
| `sha512` | ✅ |
| `sharedvar` | ✅ |
| `snat` | ✅ |
| `snatpool` | ✅ |
| `substr` | ✅ |
| `table` | ✅ |
| `tcpdump` | ✅ |
| `timing` | ✅ |
| `traffic_group` | ✅ |
| `translate` | ✅ |
| `uniq_ordered_ip_list` | ✅ |
| `uniq_sorted_ip_list` | ✅ |
| `urlcatblindquery` | ✅ |
| `urlcatquery` | ✅ |
| `use` | ✅ |
| `virtual` | ✅ |
| `vlan_id` | ✅ |
| `when` | ✅ |
| `whereis` | ✅ |
| `xff_list` | ✅ |
| `xff_uniq_ordered_ip_list` | ✅ |
| `xff_uniq_sorted_ip_list` | ✅ |

</details>

<details><summary><b>iapps</b> — 49 entries · 49 ✅ · 0 need work</summary>

| entry | status |
|---|---|
| `iapp::apm_config` | ✅ |
| `iapp::conf` | ✅ |
| `iapp::debug` | ✅ |
| `iapp::destination` | ✅ |
| `iapp::downgrade` | ✅ |
| `iapp::downgrade_template` | ✅ |
| `iapp::get_items` | ✅ |
| `iapp::is` | ✅ |
| `iapp::make_safe_password` | ✅ |
| `iapp::pool_members` | ✅ |
| `iapp::substa` | ✅ |
| `iapp::template` | ✅ |
| `iapp::tmos_version` | ✅ |
| `iapp::upgrade` | ✅ |
| `iapp::upgrade_template` | ✅ |
| `script::help` | ✅ |
| `script::init` | ✅ |
| `script::run` | ✅ |
| `script::tabc` | ✅ |
| `tmsh::add_help` | ✅ |
| `tmsh::add_tabc` | ✅ |
| `tmsh::begin_transaction` | ✅ |
| `tmsh::builtin_help` | ✅ |
| `tmsh::builtin_tabc` | ✅ |
| `tmsh::cancel_transaction` | ✅ |
| `tmsh::cd` | ✅ |
| `tmsh::clear_screen` | ✅ |
| `tmsh::commit_transaction` | ✅ |
| `tmsh::create` | ✅ |
| `tmsh::delete` | ✅ |
| `tmsh::display` | ✅ |
| `tmsh::display_threshold` | ✅ |
| `tmsh::get_config` | ✅ |
| `tmsh::get_field_names` | ✅ |
| `tmsh::get_field_value` | ✅ |
| `tmsh::get_name` | ✅ |
| `tmsh::get_status` | ✅ |
| `tmsh::get_type` | ✅ |
| `tmsh::include` | ✅ |
| `tmsh::list` | ✅ |
| `tmsh::log` | ✅ |
| `tmsh::log_dest` | ✅ |
| `tmsh::log_level` | ✅ |
| `tmsh::modify` | ✅ |
| `tmsh::pwd` | ✅ |
| `tmsh::reset_stats` | ✅ |
| `tmsh::show` | ✅ |
| `tmsh::stateless` | ✅ |
| `tmsh::version` | ✅ |

</details>

<details><summary><b>tk</b> — 55 entries · 55 ✅ · 0 need work</summary>

| entry | status |
|---|---|
| `bell` | ✅ |
| `bind` | ✅ |
| `button` | ✅ |
| `canvas` | ✅ |
| `checkbutton` | ✅ |
| `clipboard` | ✅ |
| `destroy` | ✅ |
| `entry` | ✅ |
| `event` | ✅ |
| `focus` | ✅ |
| `font` | ✅ |
| `frame` | ✅ |
| `grab` | ✅ |
| `grid` | ✅ |
| `image` | ✅ |
| `label` | ✅ |
| `labelframe` | ✅ |
| `listbox` | ✅ |
| `lower` | ✅ |
| `menu` | ✅ |
| `menubutton` | ✅ |
| `message` | ✅ |
| `option` | ✅ |
| `pack` | ✅ |
| `panedwindow` | ✅ |
| `place` | ✅ |
| `radiobutton` | ✅ |
| `raise` | ✅ |
| `scale` | ✅ |
| `scrollbar` | ✅ |
| `selection` | ✅ |
| `spinbox` | ✅ |
| `text` | ✅ |
| `tk` | ✅ |
| `tk_chooseColor` | ✅ |
| `tk_chooseDirectory` | ✅ |
| `tk_getOpenFile` | ✅ |
| `tk_getSaveFile` | ✅ |
| `tk_messageBox` | ✅ |
| `tk_popup` | ✅ |
| `toplevel` | ✅ |
| `ttk::button` | ✅ |
| `ttk::combobox` | ✅ |
| `ttk::entry` | ✅ |
| `ttk::frame` | ✅ |
| `ttk::label` | ✅ |
| `ttk::notebook` | ✅ |
| `ttk::progressbar` | ✅ |
| `ttk::scale` | ✅ |
| `ttk::separator` | ✅ |
| `ttk::sizegrip` | ✅ |
| `ttk::style` | ✅ |
| `ttk::treeview` | ✅ |
| `winfo` | ✅ |
| `wm` | ✅ |

</details>

<details><summary><b>expect</b> — 35 entries · 35 ✅ · 0 need work</summary>

| entry | status |
|---|---|
| `close` | ✅ |
| `debug` | ✅ |
| `disconnect` | ✅ |
| `exit` | ✅ |
| `exp_continue` | ✅ |
| `exp_internal` | ✅ |
| `exp_pid` | ✅ |
| `exp_version` | ✅ |
| `expect` | ✅ |
| `expect_after` | ✅ |
| `expect_background` | ✅ |
| `expect_before` | ✅ |
| `expect_tty` | ✅ |
| `expect_user` | ✅ |
| `fork` | ✅ |
| `interact` | ✅ |
| `log_file` | ✅ |
| `log_user` | ✅ |
| `match_max` | ✅ |
| `overlay` | ✅ |
| `parity` | ✅ |
| `remove_nulls` | ✅ |
| `send` | ✅ |
| `send_error` | ✅ |
| `send_log` | ✅ |
| `send_tty` | ✅ |
| `send_user` | ✅ |
| `sleep` | ✅ |
| `spawn` | ✅ |
| `strace` | ✅ |
| `stty` | ✅ |
| `system` | ✅ |
| `timestamp` | ✅ |
| `trap` | ✅ |
| `wait` | ✅ |

</details>

<details><summary><b>sdc-base</b> — 61 entries · 49 ✅ · 12 need work</summary>

| entry | status |
|---|---|
| `all_clocks` | ✅ |
| `all_fanin` | ✅ |
| `all_fanout` | ✅ |
| `all_inputs` | ✅ |
| `all_outputs` | ✅ |
| `all_registers` | `syn≠` |
| `append_to_collection` | ✅ |
| `check_timing` | ✅ |
| `create_clock` | ✅ |
| `create_generated_clock` | `syn≠` |
| `current_design` | ✅ |
| `define_proc_attributes` | ✅ |
| `filter_collection` | ✅ |
| `foreach_in_collection` | ✅ |
| `get_cells` | ✅ |
| `get_clocks` | ✅ |
| `get_lib_cells` | ✅ |
| `get_lib_pins` | ✅ |
| `get_libs` | ✅ |
| `get_nets` | ✅ |
| `get_object_name` | ✅ |
| `get_pins` | ✅ |
| `get_ports` | ✅ |
| `group_path` | `syn≠` |
| `link_design` | ✅ |
| `remove_from_collection` | ✅ |
| `report_area` | ✅ |
| `report_clock` | ✅ |
| `report_clock_timing` | ✅ |
| `report_constraint` | ✅ |
| `report_power` | ✅ |
| `report_timing` | `syn≠` |
| `set_case_analysis` | ✅ |
| `set_clock_groups` | `syn≠` |
| `set_clock_latency` | ✅ |
| `set_clock_transition` | ✅ |
| `set_clock_uncertainty` | `syn≠` |
| `set_disable_timing` | ✅ |
| `set_dont_touch` | ✅ |
| `set_dont_use` | ✅ |
| `set_driving_cell` | `syn≠` |
| `set_false_path` | `syn≠` |
| `set_ideal_latency` | ✅ |
| `set_ideal_network` | ✅ |
| `set_input_delay` | `syn≠` |
| `set_input_transition` | ✅ |
| `set_load` | ✅ |
| `set_max_area` | ✅ |
| `set_max_capacitance` | ✅ |
| `set_max_delay` | ✅ |
| `set_max_fanout` | ✅ |
| `set_max_transition` | ✅ |
| `set_min_delay` | ✅ |
| `set_multicycle_path` | `syn≠` |
| `set_output_delay` | `syn≠` |
| `set_propagated_clock` | ✅ |
| `set_size_only` | ✅ |
| `set_units` | `syn≠` |
| `set_wire_load_mode` | ✅ |
| `set_wire_load_model` | ✅ |
| `sizeof_collection` | ✅ |

</details>

<details><summary><b>synopsys</b> — 68 entries · 63 ✅ · 5 need work</summary>

| entry | status |
|---|---|
| `analyze` | ✅ |
| `characterize` | ✅ |
| `check_design` | ✅ |
| `check_library` | ✅ |
| `clock_opt` | ✅ |
| `compile` | `syn≠` |
| `compile_ultra` | `syn≠` |
| `connect_net` | ✅ |
| `create_cell` | ✅ |
| `create_floorplan` | `syn≠` |
| `create_net` | ✅ |
| `create_port` | ✅ |
| `current_instance` | ✅ |
| `disconnect_net` | ✅ |
| `elaborate` | ✅ |
| `get_timing_paths` | `syn≠` |
| `group` | ✅ |
| `initialize_floorplan` | ✅ |
| `insert_clock_gating` | ✅ |
| `insert_dft` | ✅ |
| `link` | ✅ |
| `match` | ✅ |
| `optimize_netlist` | ✅ |
| `place_opt` | ✅ |
| `printvar` | ✅ |
| `read_db` | ✅ |
| `read_ddc` | ✅ |
| `read_def` | ✅ |
| `read_file` | ✅ |
| `read_lef` | ✅ |
| `read_sdc` | ✅ |
| `read_verilog` | ✅ |
| `read_vhdl` | ✅ |
| `remove_cell` | ✅ |
| `remove_design` | ✅ |
| `report_analysis_coverage` | ✅ |
| `report_bottleneck` | ✅ |
| `report_cell` | ✅ |
| `report_clock_gating` | ✅ |
| `report_congestion` | ✅ |
| `report_delay_calculation` | ✅ |
| `report_design` | ✅ |
| `report_hierarchy` | ✅ |
| `report_net` | ✅ |
| `report_qor` | ✅ |
| `report_reference` | ✅ |
| `report_status` | ✅ |
| `route_auto` | ✅ |
| `route_opt` | ✅ |
| `set_app_var` | ✅ |
| `set_clock_gating_style` | `syn≠` |
| `set_host_options` | ✅ |
| `set_implementation_design` | ✅ |
| `set_operating_conditions` | ✅ |
| `set_reference_design` | ✅ |
| `set_scan_configuration` | ✅ |
| `set_technology` | ✅ |
| `size_cell` | ✅ |
| `swap_cell` | ✅ |
| `ungroup` | ✅ |
| `uniquify` | ✅ |
| `update_timing` | ✅ |
| `verify` | ✅ |
| `write` | ✅ |
| `write_def` | ✅ |
| `write_file` | ✅ |
| `write_gds` | ✅ |
| `write_sdc` | ✅ |

</details>

<details><summary><b>cadence</b> — 56 entries · 55 ✅ · 1 need work</summary>

| entry | status |
|---|---|
| `add_endcap` | ✅ |
| `add_filler` | ✅ |
| `add_well_tap` | ✅ |
| `ccopt_design` | ✅ |
| `check_design` | ✅ |
| `check_timing_intent` | ✅ |
| `create_analysis_view` | ✅ |
| `create_constraint_mode` | ✅ |
| `create_delay_corner` | ✅ |
| `create_floorplan` | `syn≠` |
| `create_route_rule` | ✅ |
| `dbGet` | ✅ |
| `dbQuery` | ✅ |
| `dbSet` | ✅ |
| `dbShape` | ✅ |
| `edit_pin` | ✅ |
| `elaborate` | ✅ |
| `get_db` | ✅ |
| `init_design` | ✅ |
| `opt_design` | ✅ |
| `place_opt_design` | ✅ |
| `read_hdl` | ✅ |
| `read_library` | ✅ |
| `read_mmmc` | ✅ |
| `read_netlist` | ✅ |
| `read_physical` | ✅ |
| `report_analysis_coverage` | ✅ |
| `report_area` | ✅ |
| `report_constraint` | ✅ |
| `report_dp` | ✅ |
| `report_gates` | ✅ |
| `report_power` | ✅ |
| `report_qor` | ✅ |
| `report_timing` | ✅ |
| `route_design` | ✅ |
| `set_analysis_view` | ✅ |
| `set_db` | ✅ |
| `stream_out` | ✅ |
| `syn_generic` | ✅ |
| `syn_map` | ✅ |
| `syn_opt` | ✅ |
| `time_design` | ✅ |
| `update_timing` | ✅ |
| `verify_connectivity` | ✅ |
| `verify_drc` | ✅ |
| `verify_geometry` | ✅ |
| `write_def` | ✅ |
| `write_design` | ✅ |
| `write_do_lec` | ✅ |
| `write_gds` | ✅ |
| `write_hdl` | ✅ |
| `write_netlist` | ✅ |
| `write_sdc` | ✅ |
| `xelab` | ✅ |
| `xrun` | ✅ |
| `xsim` | ✅ |

</details>

<details><summary><b>xilinx</b> — 64 entries · 59 ✅ · 5 need work</summary>

| entry | status |
|---|---|
| `apply_bd_automation` | ✅ |
| `close_hw_manager` | ✅ |
| `close_project` | ✅ |
| `close_sim` | ✅ |
| `config_ip_cache` | ✅ |
| `connect_bd_intf_net` | ✅ |
| `connect_bd_net` | ✅ |
| `connect_hw_server` | ✅ |
| `create_bd_cell` | ✅ |
| `create_bd_design` | ✅ |
| `create_bd_intf_port` | ✅ |
| `create_bd_port` | ✅ |
| `create_ip` | ✅ |
| `create_project` | ✅ |
| `create_run` | ✅ |
| `current_project` | ✅ |
| `generate_target` | ✅ |
| `get_ips` | ✅ |
| `get_projects` | ✅ |
| `get_property` | ✅ |
| `get_runs` | ✅ |
| `import_ip` | ✅ |
| `ipx::add_bus_interface` | ✅ |
| `ipx::package_project` | ✅ |
| `launch_runs` | ✅ |
| `launch_simulation` | ✅ |
| `open_bd_design` | ✅ |
| `open_checkpoint` | ✅ |
| `open_hw_manager` | ✅ |
| `open_hw_target` | ✅ |
| `open_project` | ✅ |
| `open_run` | `arity≠` |
| `opt_design` | ✅ |
| `phys_opt_design` | `syn≠` |
| `place_design` | ✅ |
| `program_hw_devices` | ✅ |
| `read_checkpoint` | ✅ |
| `read_edif` | ✅ |
| `read_ip` | ✅ |
| `read_verilog` | ✅ |
| `read_vhdl` | ✅ |
| `read_xdc` | `arity≠` |
| `report_clock_networks` | ✅ |
| `report_clock_utilization` | ✅ |
| `report_design_analysis` | ✅ |
| `report_drc` | ✅ |
| `report_io` | ✅ |
| `report_methodology` | ✅ |
| `report_power` | ✅ |
| `report_route_status` | ✅ |
| `report_timing` | `syn≠` |
| `report_timing_summary` | ✅ |
| `report_utilization` | ✅ |
| `reset_run` | ✅ |
| `route_design` | ✅ |
| `save_bd_design` | ✅ |
| `save_project_as` | ✅ |
| `set_property` | ✅ |
| `synth_design` | `syn≠` |
| `upgrade_ip` | ✅ |
| `validate_bd_design` | ✅ |
| `wait_on_run` | ✅ |
| `write_bitstream` | ✅ |
| `write_checkpoint` | ✅ |

</details>

<details><summary><b>quartus</b> — 48 entries · 46 ✅ · 2 need work</summary>

| entry | status |
|---|---|
| `check_timing` | ✅ |
| `close_device` | ✅ |
| `create_timing_netlist` | ✅ |
| `delete_timing_netlist` | ✅ |
| `derive_clocks` | ✅ |
| `derive_pll_clocks` | ✅ |
| `device_lock` | ✅ |
| `device_unlock` | ✅ |
| `execute_flow` | `syn≠` |
| `execute_module` | ✅ |
| `export_assignments` | ✅ |
| `get_all_assignments` | ✅ |
| `get_global_assignment` | ✅ |
| `get_instance_assignment` | ✅ |
| `get_io_assignment` | ✅ |
| `get_name_info` | ✅ |
| `get_names` | ✅ |
| `get_number_of_columns` | ✅ |
| `get_number_of_rows` | ✅ |
| `get_part_info` | ✅ |
| `get_part_list` | ✅ |
| `get_report_panel_data` | ✅ |
| `get_report_panel_id` | ✅ |
| `get_report_panel_row_index` | ✅ |
| `load_package` | ✅ |
| `load_report` | ✅ |
| `make_connection` | ✅ |
| `open_device` | ✅ |
| `project_close` | ✅ |
| `project_exists` | ✅ |
| `project_new` | ✅ |
| `project_open` | ✅ |
| `read_sdc` | ✅ |
| `remove_all_assignments` | ✅ |
| `remove_connection` | ✅ |
| `rename_node` | ✅ |
| `report_clock_fmax_summary` | ✅ |
| `report_datasheet` | ✅ |
| `report_min_pulse_width` | ✅ |
| `report_timing` | `syn≠` |
| `report_ucp` | ✅ |
| `save_report` | ✅ |
| `set_global_assignment` | ✅ |
| `set_instance_assignment` | ✅ |
| `set_io_assignment` | ✅ |
| `set_location_assignment` | ✅ |
| `set_parameter` | ✅ |
| `update_timing_netlist` | ✅ |

</details>

<details><summary><b>mentor</b> — 49 entries · 45 ✅ · 4 need work</summary>

| entry | status |
|---|---|
| `add_list` | ✅ |
| `add_log` | ✅ |
| `add_wave` | `syn≠` |
| `bc` | ✅ |
| `bd` | ✅ |
| `be` | ✅ |
| `bl` | ✅ |
| `bp` | ✅ |
| `calibre` | ✅ |
| `calibre_drc` | ✅ |
| `calibre_lvs` | ✅ |
| `calibre_pex` | ✅ |
| `change` | ✅ |
| `coverage` | ✅ |
| `describe` | ✅ |
| `drivers` | ✅ |
| `examine` | ✅ |
| `find` | ✅ |
| `force` | ✅ |
| `formal_analyze` | ✅ |
| `formal_compile` | ✅ |
| `formal_verify` | ✅ |
| `init_signal_driver` | ✅ |
| `init_signal_spy` | ✅ |
| `onbreak` | ✅ |
| `qrun` | ✅ |
| `qverilog` | ✅ |
| `qvhdl` | ✅ |
| `qwave` | ✅ |
| `readers` | ✅ |
| `release` | ✅ |
| `restart` | ✅ |
| `resume` | ✅ |
| `run` | ✅ |
| `signal_force` | ✅ |
| `signal_release` | ✅ |
| `toggle` | ✅ |
| `transcript` | ✅ |
| `vcom` | `syn≠` |
| `vcover` | ✅ |
| `vdel` | ✅ |
| `virtual` | ✅ |
| `vlib` | ✅ |
| `vlog` | `syn≠` |
| `vmap` | ✅ |
| `vopt` | ✅ |
| `vsim` | `syn≠` |
| `wave` | ✅ |
| `when` | ✅ |

</details>

### Meta / data registries

<details><summary><b>bigip object registry</b> — 992 entries · 992 ✅ · 0 unported</summary>

Every kind is ported to the Rust `bigip` registry (`rust/tcl-registry/src/bigip/`); property-level parity is verified by `scripts/registry-audit/audit_bigip.py`.

| object kind | status |
|---|---|
| `analytics_afm_sweeper_scheduled_report` | ✅ |
| `analytics_application_security_anomalies_scheduled_report` | ✅ |
| `analytics_application_security_network_scheduled_report` | ✅ |
| `analytics_application_security_scheduled_report` | ✅ |
| `analytics_asm_bypass_scheduled_report` | ✅ |
| `analytics_asm_cpu_scheduled_report` | ✅ |
| `analytics_asm_memory_scheduled_report` | ✅ |
| `analytics_asm_violation_scheduled_report` | ✅ |
| `analytics_cpu_scheduled_report` | ✅ |
| `analytics_device_traffic_scheduled_report` | ✅ |
| `analytics_disk_info_scheduled_report` | ✅ |
| `analytics_dns_protocol_scheduled_report` | ✅ |
| `analytics_dns_scheduled_report` | ✅ |
| `analytics_dos_l3_scheduled_report` | ✅ |
| `analytics_fw_nat_scheduled_report` | ✅ |
| `analytics_global_settings` | ✅ |
| `analytics_http_scheduled_report` | ✅ |
| `analytics_ip_intelligence_scheduled_report` | ✅ |
| `analytics_ip_layer_scheduled_report` | ✅ |
| `analytics_lsn_pool_scheduled_report` | ✅ |
| `analytics_memory_scheduled_report` | ✅ |
| `analytics_network_scheduled_report` | ✅ |
| `analytics_pem_scheduled_report` | ✅ |
| `analytics_pool_traffic_scheduled_report` | ✅ |
| `analytics_proc_cpu_scheduled_report` | ✅ |
| `analytics_protocol_security_http_scheduled_report` | ✅ |
| `analytics_protocol_security_scheduled_report` | ✅ |
| `analytics_sip_dos_scheduled_report` | ✅ |
| `analytics_sip_scheduled_report` | ✅ |
| `analytics_ssl_orchestrator_scheduled_report` | ✅ |
| `analytics_ssl_orchestrator_service_virtual_scheduled_report` | ✅ |
| `analytics_swg_blocked_scheduled_report` | ✅ |
| `analytics_swg_scheduled_report` | ✅ |
| `analytics_tcp_analytics_scheduled_report` | ✅ |
| `analytics_tcp_scheduled_report` | ✅ |
| `analytics_traffic_classification_scheduled_report` | ✅ |
| `analytics_udp_scheduled_report` | ✅ |
| `analytics_uri_type` | ✅ |
| `analytics_vcmp_scheduled_report` | ✅ |
| `analytics_virtual_scheduled_report` | ✅ |
| `api_protection_profile_apiprotection` | ✅ |
| `api_protection_response` | ✅ |
| `api_protection_server` | ✅ |
| `apm_aaa_active_directory` | ✅ |
| `apm_aaa_active_directory_trusted_domains` | ✅ |
| `apm_aaa_crldp` | ✅ |
| `apm_aaa_endpoint_management_system` | ✅ |
| `apm_aaa_f5_mfa_configuration` | ✅ |
| `apm_aaa_f5_service_connector` | ✅ |
| `apm_aaa_http` | ✅ |
| `apm_aaa_http_connector_request` | ✅ |
| `apm_aaa_kerberos` | ✅ |
| `apm_aaa_kerberos_keytab_file` | ✅ |
| `apm_aaa_ldap` | ✅ |
| `apm_aaa_oam` | ✅ |
| `apm_aaa_oauth_provider` | ✅ |
| `apm_aaa_oauth_request` | ✅ |
| `apm_aaa_oauth_server` | ✅ |
| `apm_aaa_ocsp` | ✅ |
| `apm_aaa_okta_connector` | ✅ |
| `apm_aaa_radius` | ✅ |
| `apm_aaa_saml` | ✅ |
| `apm_aaa_saml_idp_automation` | ✅ |
| `apm_aaa_saml_idp_connector` | ✅ |
| `apm_aaa_securid` | ✅ |
| `apm_aaa_tacacsplus` | ✅ |
| `apm_acl` | ✅ |
| `apm_apm_avr_config` | ✅ |
| `apm_client_image` | ✅ |
| `apm_configuration_captcha` | ✅ |
| `apm_epsec_epsec_package` | ✅ |
| `apm_log_setting` | ✅ |
| `apm_ntlm_machine_account` | ✅ |
| `apm_ntlm_ntlm_auth` | ✅ |
| `apm_oauth_db_instance` | ✅ |
| `apm_oauth_jwk_config` | ✅ |
| `apm_oauth_jwt_config` | ✅ |
| `apm_oauth_jwt_provider_list` | ✅ |
| `apm_oauth_oauth_claim` | ✅ |
| `apm_oauth_oauth_client_app` | ✅ |
| `apm_oauth_oauth_resource_server` | ✅ |
| `apm_oauth_oauth_scope` | ✅ |
| `apm_policy_agent_aaa_active_directory` | ✅ |
| `apm_policy_agent_aaa_client_cert` | ✅ |
| `apm_policy_agent_aaa_crldp` | ✅ |
| `apm_policy_agent_aaa_http` | ✅ |
| `apm_policy_agent_aaa_ldap` | ✅ |
| `apm_policy_agent_aaa_oauth` | ✅ |
| `apm_policy_agent_aaa_radius` | ✅ |
| `apm_policy_agent_aaa_saml` | ✅ |
| `apm_policy_agent_aaa_securid` | ✅ |
| `apm_policy_agent_acct_radius` | ✅ |
| `apm_policy_agent_acct_tacacsplus` | ✅ |
| `apm_policy_agent_api_authentication` | ✅ |
| `apm_policy_agent_api_server_selection` | ✅ |
| `apm_policy_agent_decision_box` | ✅ |
| `apm_policy_agent_dynamic_acl` | ✅ |
| `apm_policy_agent_ending_allow` | ✅ |
| `apm_policy_agent_ending_deny` | ✅ |
| `apm_policy_agent_ending_redirect` | ✅ |
| `apm_policy_agent_endpoint_check_machine_cert` | ✅ |
| `apm_policy_agent_endpoint_check_software` | ✅ |
| `apm_policy_agent_endpoint_linux_check_file` | ✅ |
| `apm_policy_agent_endpoint_linux_check_process` | ✅ |
| `apm_policy_agent_endpoint_mac_check_file` | ✅ |
| `apm_policy_agent_endpoint_mac_check_process` | ✅ |
| `apm_policy_agent_endpoint_machine_info` | ✅ |
| `apm_policy_agent_endpoint_windows_browser_cache_cleaner` | ✅ |
| `apm_policy_agent_endpoint_windows_check_file` | ✅ |
| `apm_policy_agent_endpoint_windows_check_process` | ✅ |
| `apm_policy_agent_endpoint_windows_check_registry` | ✅ |
| `apm_policy_agent_endpoint_windows_group_policy` | ✅ |
| `apm_policy_agent_endpoint_windows_info_os` | ✅ |
| `apm_policy_agent_endpoint_windows_protected_workspace` | ✅ |
| `apm_policy_agent_external_logon_page` | ✅ |
| `apm_policy_agent_http_header_modify` | ✅ |
| `apm_policy_agent_ip_geolocation_lookup` | ✅ |
| `apm_policy_agent_ip_reputation_lookup` | ✅ |
| `apm_policy_agent_irule_event` | ✅ |
| `apm_policy_agent_kerberos` | ✅ |
| `apm_policy_agent_l7_protocol_lookup` | ✅ |
| `apm_policy_agent_logging` | ✅ |
| `apm_policy_agent_logon_page` | ✅ |
| `apm_policy_agent_message_box` | ✅ |
| `apm_policy_agent_oam` | ✅ |
| `apm_policy_agent_oauth_authz` | ✅ |
| `apm_policy_agent_request_classification` | ✅ |
| `apm_policy_agent_resource_assign` | ✅ |
| `apm_policy_agent_response_selection` | ✅ |
| `apm_policy_agent_route_domain_selection` | ✅ |
| `apm_policy_agent_server_cert_response_control` | ✅ |
| `apm_policy_agent_server_cert_status` | ✅ |
| `apm_policy_agent_session_check` | ✅ |
| `apm_policy_agent_ssl_check` | ✅ |
| `apm_policy_agent_tacacsplus` | ✅ |
| `apm_policy_agent_variable_assign` | ✅ |
| `apm_profile_access` | ✅ |
| `apm_profile_connectivity` | ✅ |
| `apm_profile_exchange` | ✅ |
| `apm_profile_oauth` | ✅ |
| `apm_profile_vdi` | ✅ |
| `apm_report_custom_report_field` | ✅ |
| `apm_resource_address_space` | ✅ |
| `apm_resource_app_tunnel` | ✅ |
| `apm_resource_client_rate_class` | ✅ |
| `apm_resource_client_traffic_classifier` | ✅ |
| `apm_resource_ipv6_leasepool` | ✅ |
| `apm_resource_leasepool` | ✅ |
| `apm_resource_network_access` | ✅ |
| `apm_resource_portal_access` | ✅ |
| `apm_resource_remote_desktop_citrix` | ✅ |
| `apm_resource_remote_desktop_citrix_client_bundle` | ✅ |
| `apm_resource_remote_desktop_citrix_client_package_file` | ✅ |
| `apm_resource_remote_desktop_quest` | ✅ |
| `apm_resource_remote_desktop_rdp` | ✅ |
| `apm_resource_remote_desktop_vmware_view` | ✅ |
| `apm_resource_sandbox` | ✅ |
| `apm_resource_webtop` | ✅ |
| `apm_resource_webtop_link` | ✅ |
| `apm_saml_artifact_resolution_service` | ✅ |
| `apm_saml_attribute_consuming_service` | ✅ |
| `apm_saml_auth_context_class_list` | ✅ |
| `apm_session` | ✅ |
| `apm_sso_basic` | ✅ |
| `apm_sso_form_based` | ✅ |
| `apm_sso_form_basedv2` | ✅ |
| `apm_sso_kerberos` | ✅ |
| `apm_sso_ntlmv1` | ✅ |
| `apm_sso_ntlmv2` | ✅ |
| `apm_sso_oauth_bearer` | ✅ |
| `apm_sso_saml` | ✅ |
| `apm_sso_saml_resource` | ✅ |
| `apm_sso_saml_sp_automation` | ✅ |
| `apm_sso_saml_sp_connector` | ✅ |
| `apm_swg_scheme` | ✅ |
| `apm_url_filter` | ✅ |
| `asm_httpclass_asm` | ✅ |
| `asm_policy` | ✅ |
| `auth_apm_auth` | ✅ |
| `auth_cert_ldap` | ✅ |
| `auth_ldap` | ✅ |
| `auth_login_failures` | ✅ |
| `auth_partition` | ✅ |
| `auth_password` | ✅ |
| `auth_password_policy` | ✅ |
| `auth_radius` | ✅ |
| `auth_radius_server` | ✅ |
| `auth_remote_role` | ✅ |
| `auth_remote_user` | ✅ |
| `auth_source` | ✅ |
| `auth_tacacs` | ✅ |
| `auth_user` | ✅ |
| `cli_admin_partitions` | ✅ |
| `cli_alias_private` | ✅ |
| `cli_alias_shared` | ✅ |
| `cli_global_settings` | ✅ |
| `cli_preference` | ✅ |
| `cli_script` | ✅ |
| `cli_transaction` | ✅ |
| `cli_version` | ✅ |
| `cm_add_to_trust` | ✅ |
| `cm_cert` | ✅ |
| `cm_config_sync` | ✅ |
| `cm_device` | ✅ |
| `cm_device_group` | ✅ |
| `cm_failover_status` | ✅ |
| `cm_ha_group` | ✅ |
| `cm_key` | ✅ |
| `cm_remove_from_trust` | ✅ |
| `cm_sha1_fingerprint` | ✅ |
| `cm_sniff_updates` | ✅ |
| `cm_sync_status` | ✅ |
| `cm_traffic_group` | ✅ |
| `cm_trust_domain` | ✅ |
| `cm_watch_devicegroup_device` | ✅ |
| `cm_watch_sys_device` | ✅ |
| `cm_watch_trafficgroup_device` | ✅ |
| `data_group` | ✅ |
| `gtm_add` | ✅ |
| `gtm_datacenter` | ✅ |
| `gtm_distributed_app` | ✅ |
| `gtm_global_settings_general` | ✅ |
| `gtm_global_settings_load_balancing` | ✅ |
| `gtm_global_settings_metrics` | ✅ |
| `gtm_global_settings_metrics_exclusions` | ✅ |
| `gtm_iquery` | ✅ |
| `gtm_ldns` | ✅ |
| `gtm_link` | ✅ |
| `gtm_listener` | ✅ |
| `gtm_listener_doh_proxy` | ✅ |
| `gtm_listener_doh_server` | ✅ |
| `gtm_monitor_bigip` | ✅ |
| `gtm_monitor_bigip_link` | ✅ |
| `gtm_monitor_external` | ✅ |
| `gtm_monitor_firepass` | ✅ |
| `gtm_monitor_ftp` | ✅ |
| `gtm_monitor_gateway_icmp` | ✅ |
| `gtm_monitor_gtp` | ✅ |
| `gtm_monitor_http` | ✅ |
| `gtm_monitor_https` | ✅ |
| `gtm_monitor_imap` | ✅ |
| `gtm_monitor_ldap` | ✅ |
| `gtm_monitor_mssql` | ✅ |
| `gtm_monitor_mysql` | ✅ |
| `gtm_monitor_nntp` | ✅ |
| `gtm_monitor_none` | ✅ |
| `gtm_monitor_oracle` | ✅ |
| `gtm_monitor_pop3` | ✅ |
| `gtm_monitor_postgresql` | ✅ |
| `gtm_monitor_radius` | ✅ |
| `gtm_monitor_radius_accounting` | ✅ |
| `gtm_monitor_real_server` | ✅ |
| `gtm_monitor_scripted` | ✅ |
| `gtm_monitor_sip` | ✅ |
| `gtm_monitor_smtp` | ✅ |
| `gtm_monitor_snmp` | ✅ |
| `gtm_monitor_snmp_link` | ✅ |
| `gtm_monitor_soap` | ✅ |
| `gtm_monitor_tcp` | ✅ |
| `gtm_monitor_tcp_half_open` | ✅ |
| `gtm_monitor_udp` | ✅ |
| `gtm_monitor_wap` | ✅ |
| `gtm_monitor_wmi` | ✅ |
| `gtm_path` | ✅ |
| `gtm_persist` | ✅ |
| `gtm_pool` | ✅ |
| `gtm_pool_a` | ✅ |
| `gtm_pool_aaaa` | ✅ |
| `gtm_pool_cname` | ✅ |
| `gtm_pool_https` | ✅ |
| `gtm_pool_mx` | ✅ |
| `gtm_pool_naptr` | ✅ |
| `gtm_pool_srv` | ✅ |
| `gtm_pool_svcb` | ✅ |
| `gtm_prober_pool` | ✅ |
| `gtm_region` | ✅ |
| `gtm_rule` | ✅ |
| `gtm_server` | ✅ |
| `gtm_topology` | ✅ |
| `gtm_traffic` | ✅ |
| `gtm_wideip_a` | ✅ |
| `gtm_wideip_aaaa` | ✅ |
| `gtm_wideip_cname` | ✅ |
| `gtm_wideip_https` | ✅ |
| `gtm_wideip_mx` | ✅ |
| `gtm_wideip_naptr` | ✅ |
| `gtm_wideip_srv` | ✅ |
| `gtm_wideip_svcb` | ✅ |
| `ilx_global_settings` | ✅ |
| `ilx_plugin` | ✅ |
| `ilx_workspace` | ✅ |
| `ltm_alg_log_profile` | ✅ |
| `ltm_auth_crldp_server` | ✅ |
| `ltm_auth_kerberos_delegation` | ✅ |
| `ltm_auth_ldap` | ✅ |
| `ltm_auth_ocsp_responder` | ✅ |
| `ltm_auth_profile` | ✅ |
| `ltm_auth_radius` | ✅ |
| `ltm_auth_radius_server` | ✅ |
| `ltm_auth_ssl_cc_ldap` | ✅ |
| `ltm_auth_ssl_crldp` | ✅ |
| `ltm_auth_ssl_ocsp` | ✅ |
| `ltm_auth_tacacs` | ✅ |
| `ltm_cipher_group` | ✅ |
| `ltm_cipher_rule` | ✅ |
| `ltm_classification_application` | ✅ |
| `ltm_classification_auto_update_settings` | ✅ |
| `ltm_classification_auto_update_status` | ✅ |
| `ltm_classification_category` | ✅ |
| `ltm_classification_ce` | ✅ |
| `ltm_classification_signature_definition` | ✅ |
| `ltm_classification_signature_update_schedule` | ✅ |
| `ltm_classification_signature_version` | ✅ |
| `ltm_classification_signatures` | ✅ |
| `ltm_classification_stats_application` | ✅ |
| `ltm_classification_stats_url_category` | ✅ |
| `ltm_classification_stats_urlcat_cloud` | ✅ |
| `ltm_classification_update_signatures` | ✅ |
| `ltm_classification_updates` | ✅ |
| `ltm_classification_url_cat_policy` | ✅ |
| `ltm_classification_url_category` | ✅ |
| `ltm_classification_urldb_feed_list` | ✅ |
| `ltm_classification_urldb_file` | ✅ |
| `ltm_clientssl_ocsp_stapling_responses` | ✅ |
| `ltm_clientssl_proxy_cached_certs` | ✅ |
| `ltm_data_group_external` | ✅ |
| `ltm_data_group_internal` | ✅ |
| `ltm_default_node_monitor` | ✅ |
| `ltm_dns_analytics_global_settings` | ✅ |
| `ltm_dns_cache_global_settings` | ✅ |
| `ltm_dns_cache_records_all` | ✅ |
| `ltm_dns_cache_records_key` | ✅ |
| `ltm_dns_cache_records_msg` | ✅ |
| `ltm_dns_cache_records_nameserver` | ✅ |
| `ltm_dns_cache_records_rrset` | ✅ |
| `ltm_dns_cache_resolver` | ✅ |
| `ltm_dns_cache_transparent` | ✅ |
| `ltm_dns_cache_validating_resolver` | ✅ |
| `ltm_dns_dns_express_db` | ✅ |
| `ltm_dns_dnssec_key` | ✅ |
| `ltm_dns_dnssec_zone` | ✅ |
| `ltm_dns_hpke_key` | ✅ |
| `ltm_dns_hpke_profile` | ✅ |
| `ltm_dns_nameserver` | ✅ |
| `ltm_dns_tsig_key` | ✅ |
| `ltm_dns_zone` | ✅ |
| `ltm_eviction_policy` | ✅ |
| `ltm_global_settings_connection` | ✅ |
| `ltm_global_settings_general` | ✅ |
| `ltm_global_settings_rule` | ✅ |
| `ltm_global_settings_traffic_control` | ✅ |
| `ltm_ifile` | ✅ |
| `ltm_lsn_log_profile` | ✅ |
| `ltm_lsn_pool` | ✅ |
| `ltm_message_routing_diameter_peer` | ✅ |
| `ltm_message_routing_diameter_profile_router` | ✅ |
| `ltm_message_routing_diameter_profile_session` | ✅ |
| `ltm_message_routing_diameter_route` | ✅ |
| `ltm_message_routing_diameter_transport_config` | ✅ |
| `ltm_message_routing_generic_peer` | ✅ |
| `ltm_message_routing_generic_protocol` | ✅ |
| `ltm_message_routing_generic_route` | ✅ |
| `ltm_message_routing_generic_router` | ✅ |
| `ltm_message_routing_generic_transport_config` | ✅ |
| `ltm_message_routing_mqtt_peer` | ✅ |
| `ltm_message_routing_mqtt_profile_router` | ✅ |
| `ltm_message_routing_mqtt_profile_session` | ✅ |
| `ltm_message_routing_mqtt_route` | ✅ |
| `ltm_message_routing_mqtt_transport_config` | ✅ |
| `ltm_message_routing_sip_peer` | ✅ |
| `ltm_message_routing_sip_profile_router` | ✅ |
| `ltm_message_routing_sip_profile_session` | ✅ |
| `ltm_message_routing_sip_route` | ✅ |
| `ltm_message_routing_sip_transport_config` | ✅ |
| `ltm_monitor_diameter` | ✅ |
| `ltm_monitor_dns` | ✅ |
| `ltm_monitor_external` | ✅ |
| `ltm_monitor_firepass` | ✅ |
| `ltm_monitor_ftp` | ✅ |
| `ltm_monitor_gateway_icmp` | ✅ |
| `ltm_monitor_http` | ✅ |
| `ltm_monitor_http2` | ✅ |
| `ltm_monitor_https` | ✅ |
| `ltm_monitor_icmp` | ✅ |
| `ltm_monitor_imap` | ✅ |
| `ltm_monitor_inband` | ✅ |
| `ltm_monitor_ldap` | ✅ |
| `ltm_monitor_module_score` | ✅ |
| `ltm_monitor_mqtt` | ✅ |
| `ltm_monitor_mssql` | ✅ |
| `ltm_monitor_mysql` | ✅ |
| `ltm_monitor_nntp` | ✅ |
| `ltm_monitor_none` | ✅ |
| `ltm_monitor_oracle` | ✅ |
| `ltm_monitor_pop3` | ✅ |
| `ltm_monitor_postgresql` | ✅ |
| `ltm_monitor_radius` | ✅ |
| `ltm_monitor_radius_accounting` | ✅ |
| `ltm_monitor_real_server` | ✅ |
| `ltm_monitor_rpc` | ✅ |
| `ltm_monitor_sasp` | ✅ |
| `ltm_monitor_scripted` | ✅ |
| `ltm_monitor_sip` | ✅ |
| `ltm_monitor_smb` | ✅ |
| `ltm_monitor_smtp` | ✅ |
| `ltm_monitor_snmp_dca` | ✅ |
| `ltm_monitor_snmp_dca_base` | ✅ |
| `ltm_monitor_soap` | ✅ |
| `ltm_monitor_tcp` | ✅ |
| `ltm_monitor_tcp_echo` | ✅ |
| `ltm_monitor_tcp_half_open` | ✅ |
| `ltm_monitor_udp` | ✅ |
| `ltm_monitor_virtual_location` | ✅ |
| `ltm_monitor_wap` | ✅ |
| `ltm_monitor_wmi` | ✅ |
| `ltm_nat` | ✅ |
| `ltm_nat_stats` | ✅ |
| `ltm_node` | ✅ |
| `ltm_persistence_cookie` | ✅ |
| `ltm_persistence_dest_addr` | ✅ |
| `ltm_persistence_global_settings` | ✅ |
| `ltm_persistence_hash` | ✅ |
| `ltm_persistence_host` | ✅ |
| `ltm_persistence_msrdp` | ✅ |
| `ltm_persistence_persist_records` | ✅ |
| `ltm_persistence_sip` | ✅ |
| `ltm_persistence_source_addr` | ✅ |
| `ltm_persistence_ssl` | ✅ |
| `ltm_persistence_universal` | ✅ |
| `ltm_policy` | ✅ |
| `ltm_policy_strategy` | ✅ |
| `ltm_pool` | ✅ |
| `ltm_profile_analytics` | ✅ |
| `ltm_profile_certificate_authority` | ✅ |
| `ltm_profile_classification` | ✅ |
| `ltm_profile_client_ldap` | ✅ |
| `ltm_profile_client_ssl` | ✅ |
| `ltm_profile_connector` | ✅ |
| `ltm_profile_dhcpv4` | ✅ |
| `ltm_profile_dhcpv6` | ✅ |
| `ltm_profile_diameter` | ✅ |
| `ltm_profile_dns` | ✅ |
| `ltm_profile_dns_logging` | ✅ |
| `ltm_profile_doh_proxy` | ✅ |
| `ltm_profile_doh_server` | ✅ |
| `ltm_profile_fasthttp` | ✅ |
| `ltm_profile_fastl4` | ✅ |
| `ltm_profile_fix` | ✅ |
| `ltm_profile_ftp` | ✅ |
| `ltm_profile_georedundancy` | ✅ |
| `ltm_profile_gtp` | ✅ |
| `ltm_profile_html` | ✅ |
| `ltm_profile_http` | ✅ |
| `ltm_profile_http2` | ✅ |
| `ltm_profile_http3` | ✅ |
| `ltm_profile_http_compression` | ✅ |
| `ltm_profile_httprouter` | ✅ |
| `ltm_profile_icap` | ✅ |
| `ltm_profile_iiop` | ✅ |
| `ltm_profile_ilx` | ✅ |
| `ltm_profile_imap` | ✅ |
| `ltm_profile_ipother` | ✅ |
| `ltm_profile_ipsecalg` | ✅ |
| `ltm_profile_json` | ✅ |
| `ltm_profile_mapt` | ✅ |
| `ltm_profile_mblb` | ✅ |
| `ltm_profile_mqtt` | ✅ |
| `ltm_profile_mr_ratelimit` | ✅ |
| `ltm_profile_mr_ratelimit_action` | ✅ |
| `ltm_profile_mssql` | ✅ |
| `ltm_profile_netflow` | ✅ |
| `ltm_profile_ntlm` | ✅ |
| `ltm_profile_ocsp` | ✅ |
| `ltm_profile_ocsp_stapling_params` | ✅ |
| `ltm_profile_one_connect` | ✅ |
| `ltm_profile_pcp` | ✅ |
| `ltm_profile_pop3` | ✅ |
| `ltm_profile_pptp` | ✅ |
| `ltm_profile_qoe` | ✅ |
| `ltm_profile_quic` | ✅ |
| `ltm_profile_radius` | ✅ |
| `ltm_profile_ramcache` | ✅ |
| `ltm_profile_request_adapt` | ✅ |
| `ltm_profile_request_log` | ✅ |
| `ltm_profile_response_adapt` | ✅ |
| `ltm_profile_rewrite` | ✅ |
| `ltm_profile_rtsp` | ✅ |
| `ltm_profile_sctp` | ✅ |
| `ltm_profile_server_ldap` | ✅ |
| `ltm_profile_server_ssl` | ✅ |
| `ltm_profile_sip` | ✅ |
| `ltm_profile_smtp` | ✅ |
| `ltm_profile_smtps` | ✅ |
| `ltm_profile_socks` | ✅ |
| `ltm_profile_splitsessionclient` | ✅ |
| `ltm_profile_splitsessionserver` | ✅ |
| `ltm_profile_sse` | ✅ |
| `ltm_profile_statistics` | ✅ |
| `ltm_profile_stream` | ✅ |
| `ltm_profile_tcp` | ✅ |
| `ltm_profile_tcp_analytics` | ✅ |
| `ltm_profile_tdr` | ✅ |
| `ltm_profile_tftp` | ✅ |
| `ltm_profile_traffic_acceleration` | ✅ |
| `ltm_profile_udp` | ✅ |
| `ltm_profile_wa_cache` | ✅ |
| `ltm_profile_web_acceleration` | ✅ |
| `ltm_profile_web_security` | ✅ |
| `ltm_profile_websocket` | ✅ |
| `ltm_profile_xml` | ✅ |
| `ltm_rule` | ✅ |
| `ltm_rule_profiler` | ✅ |
| `ltm_snat` | ✅ |
| `ltm_snat_translation` | ✅ |
| `ltm_snatpool` | ✅ |
| `ltm_tacdb_customdb` | ✅ |
| `ltm_tacdb_customdb_file` | ✅ |
| `ltm_tacdb_licenseddb` | ✅ |
| `ltm_tacdb_query` | ✅ |
| `ltm_traffic_class` | ✅ |
| `ltm_traffic_matching_criteria` | ✅ |
| `ltm_urlcat_cloud_cache` | ✅ |
| `ltm_urlcat_query` | ✅ |
| `ltm_virtual` | ✅ |
| `ltm_virtual_address` | ✅ |
| `mgmt_shared_settings_api_status_availability` | ✅ |
| `mgmt_shared_settings_api_status_log_resource` | ✅ |
| `mgmt_shared_settings_api_status_log_resource_property` | ✅ |
| `net_address_list` | ✅ |
| `net_arp` | ✅ |
| `net_bwc_policy` | ✅ |
| `net_bwc_priority_group` | ✅ |
| `net_bwc_traffic_group` | ✅ |
| `net_clone_stats` | ✅ |
| `net_cmetrics` | ✅ |
| `net_cos_global_settings` | ✅ |
| `net_cos_map_8021p` | ✅ |
| `net_cos_map_dscp` | ✅ |
| `net_cos_traffic_priority` | ✅ |
| `net_dag_globals` | ✅ |
| `net_dns_resolver` | ✅ |
| `net_f5optics` | ✅ |
| `net_fdb_tunnel` | ✅ |
| `net_fdb_vlan` | ✅ |
| `net_ike_evt_stat` | ✅ |
| `net_ike_msg_stat` | ✅ |
| `net_interface` | ✅ |
| `net_interface_cos` | ✅ |
| `net_interface_ddm` | ✅ |
| `net_ipsec_ike_daemon` | ✅ |
| `net_ipsec_ike_peer` | ✅ |
| `net_ipsec_ike_sa` | ✅ |
| `net_ipsec_ipsec_policy` | ✅ |
| `net_ipsec_ipsec_sa` | ✅ |
| `net_ipsec_manual_security_association` | ✅ |
| `net_ipsec_stat` | ✅ |
| `net_ipsec_traffic_selector` | ✅ |
| `net_ipv6_subscriber_prefix_length` | ✅ |
| `net_lacp_globals` | ✅ |
| `net_lldp_globals` | ✅ |
| `net_lldp_neighbors` | ✅ |
| `net_mroute` | ✅ |
| `net_multicast_globals` | ✅ |
| `net_ndp` | ✅ |
| `net_packet_filter` | ✅ |
| `net_packet_filter_trusted` | ✅ |
| `net_packet_tester_security` | ✅ |
| `net_port_list` | ✅ |
| `net_port_mirror` | ✅ |
| `net_rate_shaping_class` | ✅ |
| `net_rate_shaping_color_policer` | ✅ |
| `net_rate_shaping_drop_policy` | ✅ |
| `net_rate_shaping_queue` | ✅ |
| `net_rate_shaping_shaping_policy` | ✅ |
| `net_route` | ✅ |
| `net_route_domain` | ✅ |
| `net_router_advertisement` | ✅ |
| `net_routing_access_list` | ✅ |
| `net_routing_bfd` | ✅ |
| `net_routing_bgp` | ✅ |
| `net_routing_community_list` | ✅ |
| `net_routing_debug` | ✅ |
| `net_routing_extcommunity_list` | ✅ |
| `net_routing_prefix_list` | ✅ |
| `net_routing_profile_bgp` | ✅ |
| `net_routing_route_map` | ✅ |
| `net_rst_cause` | ✅ |
| `net_self` | ✅ |
| `net_self_allow` | ✅ |
| `net_service_policy` | ✅ |
| `net_sfc_chain` | ✅ |
| `net_sfc_hop` | ✅ |
| `net_sfc_sf` | ✅ |
| `net_sfc_stats` | ✅ |
| `net_stp` | ✅ |
| `net_stp_globals` | ✅ |
| `net_timer_policy` | ✅ |
| `net_trunk` | ✅ |
| `net_tunnels_endpoint` | ✅ |
| `net_tunnels_etherip` | ✅ |
| `net_tunnels_fec` | ✅ |
| `net_tunnels_fec_stat` | ✅ |
| `net_tunnels_geneve` | ✅ |
| `net_tunnels_gre` | ✅ |
| `net_tunnels_ipip` | ✅ |
| `net_tunnels_ipsec` | ✅ |
| `net_tunnels_lw4o6` | ✅ |
| `net_tunnels_map` | ✅ |
| `net_tunnels_ppp` | ✅ |
| `net_tunnels_tcp_forward` | ✅ |
| `net_tunnels_tunnel` | ✅ |
| `net_tunnels_v6rd` | ✅ |
| `net_tunnels_vxlan` | ✅ |
| `net_tunnels_wccp` | ✅ |
| `net_vlan` | ✅ |
| `net_vlan_allowed` | ✅ |
| `net_vlan_group` | ✅ |
| `net_wccp` | ✅ |
| `pem_forwarding_endpoint` | ✅ |
| `pem_global_settings_analytics` | ✅ |
| `pem_global_settings_gx` | ✅ |
| `pem_global_settings_hsl_flow` | ✅ |
| `pem_global_settings_hsl_report` | ✅ |
| `pem_global_settings_insert_content` | ✅ |
| `pem_global_settings_policy` | ✅ |
| `pem_global_settings_quota_mgmt` | ✅ |
| `pem_global_settings_session_mgmt_attributes` | ✅ |
| `pem_global_settings_subscriber_activity_log` | ✅ |
| `pem_interception_endpoint` | ✅ |
| `pem_irule` | ✅ |
| `pem_listener` | ✅ |
| `pem_policy` | ✅ |
| `pem_profile_diameter_endpoint` | ✅ |
| `pem_profile_radius_aaa` | ✅ |
| `pem_profile_spm` | ✅ |
| `pem_profile_subscriber_mgmt` | ✅ |
| `pem_protocol_diameter_avp` | ✅ |
| `pem_protocol_profile_gx` | ✅ |
| `pem_protocol_profile_radius` | ✅ |
| `pem_protocol_radius_avp` | ✅ |
| `pem_quota_mgmt_rating_group` | ✅ |
| `pem_reporting_format_script` | ✅ |
| `pem_service_chain_endpoint` | ✅ |
| `pem_subscriber` | ✅ |
| `pem_subscriber_attribute` | ✅ |
| `profile` | ✅ |
| `profile` | ✅ |
| `saas_ap_ai_profile` | ✅ |
| `saas_ati_profile` | ✅ |
| `saas_bd_profile` | ✅ |
| `saas_csd_profile` | ✅ |
| `security_analytics_settings` | ✅ |
| `security_anti_fraud_engine_update` | ✅ |
| `security_anti_fraud_profile` | ✅ |
| `security_anti_fraud_signatures_update` | ✅ |
| `security_blacklist_publisher_all_blacklist_publisher` | ✅ |
| `security_blacklist_publisher_blacklist_publisher_stats` | ✅ |
| `security_blacklist_publisher_by_addr` | ✅ |
| `security_blacklist_publisher_by_category` | ✅ |
| `security_blacklist_publisher_category` | ✅ |
| `security_blacklist_publisher_profile` | ✅ |
| `security_bot_defense_anomaly` | ✅ |
| `security_bot_defense_anomaly_category` | ✅ |
| `security_bot_defense_class` | ✅ |
| `security_bot_defense_micro_service` | ✅ |
| `security_bot_defense_profile` | ✅ |
| `security_bot_defense_signature` | ✅ |
| `security_bot_defense_signature_category` | ✅ |
| `security_bot_defense_template` | ✅ |
| `security_cloud_services_cmd` | ✅ |
| `security_cloud_services_connector` | ✅ |
| `security_datasync_background_tasks` | ✅ |
| `security_datasync_device_stats` | ✅ |
| `security_datasync_global_profile` | ✅ |
| `security_datasync_local_profile` | ✅ |
| `security_debug_drop_redirect_stats` | ✅ |
| `security_debug_matcher` | ✅ |
| `security_debug_register` | ✅ |
| `security_device_device_context` | ✅ |
| `security_device_id_attribute` | ✅ |
| `security_dos_auto_thresholds_heavy_urls` | ✅ |
| `security_dos_auto_thresholds_stress_based` | ✅ |
| `security_dos_auto_thresholds_top_device_ids` | ✅ |
| `security_dos_auto_thresholds_top_geolocations` | ✅ |
| `security_dos_auto_thresholds_top_source_ips` | ✅ |
| `security_dos_auto_thresholds_top_urls` | ✅ |
| `security_dos_auto_thresholds_tps_based` | ✅ |
| `security_dos_autodos_file_object` | ✅ |
| `security_dos_behavioral_signature` | ✅ |
| `security_dos_bot_signature` | ✅ |
| `security_dos_bot_signature_category` | ✅ |
| `security_dos_device_config` | ✅ |
| `security_dos_dns_nxdomain_stat` | ✅ |
| `security_dos_dos_signature` | ✅ |
| `security_dos_dynamic_signatures` | ✅ |
| `security_dos_ip_uncommon_protolist` | ✅ |
| `security_dos_l4bdos_file_object` | ✅ |
| `security_dos_network_whitelist` | ✅ |
| `security_dos_profile` | ✅ |
| `security_dos_spva_stats` | ✅ |
| `security_dos_stress_stats` | ✅ |
| `security_dos_udp_portlist` | ✅ |
| `security_dos_virtual` | ✅ |
| `security_firewall_address_list` | ✅ |
| `security_firewall_config_change_log` | ✅ |
| `security_firewall_container_stat` | ✅ |
| `security_firewall_context_stat` | ✅ |
| `security_firewall_current_state` | ✅ |
| `security_firewall_fqdn_entity` | ✅ |
| `security_firewall_fqdn_info` | ✅ |
| `security_firewall_global_fqdn_policy` | ✅ |
| `security_firewall_global_rules` | ✅ |
| `security_firewall_ipi_category_info` | ✅ |
| `security_firewall_management_ip_rules` | ✅ |
| `security_firewall_matching_rule` | ✅ |
| `security_firewall_on_demand_compilation` | ✅ |
| `security_firewall_on_demand_rule_deploy` | ✅ |
| `security_firewall_policy` | ✅ |
| `security_firewall_port_list` | ✅ |
| `security_firewall_port_misuse_policy` | ✅ |
| `security_firewall_rule_list` | ✅ |
| `security_firewall_rule_stat` | ✅ |
| `security_firewall_schedule` | ✅ |
| `security_firewall_user_domain` | ✅ |
| `security_firewall_user_list` | ✅ |
| `security_firewall_uuid_default_autogenerate` | ✅ |
| `security_flowspec_route_injector_flowspec_advertised_route_info` | ✅ |
| `security_flowspec_route_injector_profile` | ✅ |
| `security_http_file_type` | ✅ |
| `security_http_mandatory_header` | ✅ |
| `security_http_profile` | ✅ |
| `security_ip_intelligence_blacklist_category` | ✅ |
| `security_ip_intelligence_feed_list` | ✅ |
| `security_ip_intelligence_global_policy` | ✅ |
| `security_ip_intelligence_info` | ✅ |
| `security_ip_intelligence_policy` | ✅ |
| `security_log_antifraud_storage_field` | ✅ |
| `security_log_network_storage_field` | ✅ |
| `security_log_profile` | ✅ |
| `security_log_protocol_dns_storage_field` | ✅ |
| `security_log_protocol_sip_storage_field` | ✅ |
| `security_log_remote_format` | ✅ |
| `security_log_storage_field` | ✅ |
| `security_malicious_sources_device_ids` | ✅ |
| `security_malicious_sources_ip_addresses` | ✅ |
| `security_nat_destination_translation` | ✅ |
| `security_nat_policy` | ✅ |
| `security_nat_source_translation` | ✅ |
| `security_packet_filter_default_rules` | ✅ |
| `security_packet_filter_policy` | ✅ |
| `security_packet_filter_rule_stat` | ✅ |
| `security_presentation_tmui_netflow_details` | ✅ |
| `security_presentation_tmui_netflow_list` | ✅ |
| `security_presentation_tmui_signature_details` | ✅ |
| `security_presentation_tmui_signature_list` | ✅ |
| `security_protected_servers_netflow_tmc_stat` | ✅ |
| `security_protected_zone` | ✅ |
| `security_protocol_inspection_auto_update_settings` | ✅ |
| `security_protocol_inspection_auto_update_status` | ✅ |
| `security_protocol_inspection_common_config` | ✅ |
| `security_protocol_inspection_compliance` | ✅ |
| `security_protocol_inspection_compliance_enums` | ✅ |
| `security_protocol_inspection_learning_stats` | ✅ |
| `security_protocol_inspection_learning_suggestions` | ✅ |
| `security_protocol_inspection_profile` | ✅ |
| `security_protocol_inspection_profile_status` | ✅ |
| `security_protocol_inspection_service` | ✅ |
| `security_protocol_inspection_signature` | ✅ |
| `security_protocol_inspection_staging` | ✅ |
| `security_protocol_inspection_system` | ✅ |
| `security_protocol_inspection_updates` | ✅ |
| `security_protocol_inspection_virtual_servers` | ✅ |
| `security_scrubber_dwbl_scrubber_category_stats` | ✅ |
| `security_scrubber_dwbl_scrubber_stat` | ✅ |
| `security_scrubber_profile` | ✅ |
| `security_scrubber_unredirect` | ✅ |
| `security_ssh_ciphers` | ✅ |
| `security_ssh_profile` | ✅ |
| `security_zone` | ✅ |
| `sys_air_filter_reset` | ✅ |
| `sys_alert_lcd` | ✅ |
| `sys_aom` | ✅ |
| `sys_appiq_config` | ✅ |
| `sys_application_apl_script` | ✅ |
| `sys_application_custom_stat` | ✅ |
| `sys_application_service` | ✅ |
| `sys_application_template` | ✅ |
| `sys_autoscale_group` | ✅ |
| `sys_availability` | ✅ |
| `sys_clock` | ✅ |
| `sys_cluster` | ✅ |
| `sys_config` | ✅ |
| `sys_config_diff` | ✅ |
| `sys_connection` | ✅ |
| `sys_console` | ✅ |
| `sys_core` | ✅ |
| `sys_cpu` | ✅ |
| `sys_crypto_acceleration_strategy` | ✅ |
| `sys_crypto_allow_key_export` | ✅ |
| `sys_crypto_ca_bundle_manager` | ✅ |
| `sys_crypto_cert` | ✅ |
| `sys_crypto_cert_order_manager` | ✅ |
| `sys_crypto_cert_validation_response_ocsp` | ✅ |
| `sys_crypto_cert_validator_crl` | ✅ |
| `sys_crypto_cert_validator_ocsp` | ✅ |
| `sys_crypto_check_cert` | ✅ |
| `sys_crypto_client` | ✅ |
| `sys_crypto_crl` | ✅ |
| `sys_crypto_csr` | ✅ |
| `sys_crypto_encrypted_attributes` | ✅ |
| `sys_crypto_fips_by_handle` | ✅ |
| `sys_crypto_fips_external_hsm` | ✅ |
| `sys_crypto_fips_key` | ✅ |
| `sys_crypto_key` | ✅ |
| `sys_crypto_master_key` | ✅ |
| `sys_crypto_pkcs12` | ✅ |
| `sys_crypto_server` | ✅ |
| `sys_daemon_ha` | ✅ |
| `sys_daemon_log_settings_clusterd` | ✅ |
| `sys_daemon_log_settings_csyncd` | ✅ |
| `sys_daemon_log_settings_icr_eventd` | ✅ |
| `sys_daemon_log_settings_icrd` | ✅ |
| `sys_daemon_log_settings_lind` | ✅ |
| `sys_daemon_log_settings_mcpd` | ✅ |
| `sys_daemon_log_settings_tmm` | ✅ |
| `sys_datastor` | ✅ |
| `sys_db` | ✅ |
| `sys_default_config` | ✅ |
| `sys_diags_ihealth` | ✅ |
| `sys_diags_ihealth_request` | ✅ |
| `sys_diags_ihealth_result` | ✅ |
| `sys_disk_application_volume` | ✅ |
| `sys_disk_directory` | ✅ |
| `sys_disk_logical_disk` | ✅ |
| `sys_dns` | ✅ |
| `sys_dynad_instrumentation` | ✅ |
| `sys_dynad_key` | ✅ |
| `sys_dynad_rpm` | ✅ |
| `sys_dynad_settings` | ✅ |
| `sys_dynad_status` | ✅ |
| `sys_ecm_config` | ✅ |
| `sys_ecm_register` | ✅ |
| `sys_failover` | ✅ |
| `sys_feature_module` | ✅ |
| `sys_file_apache_ssl_cert` | ✅ |
| `sys_file_browser_capabilities_db` | ✅ |
| `sys_file_data_group` | ✅ |
| `sys_file_device_capabilities_db` | ✅ |
| `sys_file_external_monitor` | ✅ |
| `sys_file_ifile` | ✅ |
| `sys_file_lwtunneltbl` | ✅ |
| `sys_file_rewrite_rule` | ✅ |
| `sys_file_ssl_cert` | ✅ |
| `sys_file_ssl_crl` | ✅ |
| `sys_file_ssl_key` | ✅ |
| `sys_fix_connection` | ✅ |
| `sys_folder` | ✅ |
| `sys_fpga_firmware_config` | ✅ |
| `sys_fpga_info` | ✅ |
| `sys_fpga_turboflex_profile` | ✅ |
| `sys_geoip` | ✅ |
| `sys_global_settings` | ✅ |
| `sys_ha_group` | ✅ |
| `sys_ha_status` | ✅ |
| `sys_hardware` | ✅ |
| `sys_host_info` | ✅ |
| `sys_httpd` | ✅ |
| `sys_hypervisor_info` | ✅ |
| `sys_iapp_restricted_key` | ✅ |
| `sys_iapprestricted_key` | ✅ |
| `sys_icall_event` | ✅ |
| `sys_icall_handler_periodic` | ✅ |
| `sys_icall_handler_perpetual` | ✅ |
| `sys_icall_handler_triggered` | ✅ |
| `sys_icall_istats_trigger` | ✅ |
| `sys_icall_publisher` | ✅ |
| `sys_icall_script` | ✅ |
| `sys_icmp_stat` | ✅ |
| `sys_icontrol_soap` | ✅ |
| `sys_integrity_status_check` | ✅ |
| `sys_internal_proxy` | ✅ |
| `sys_ip_address` | ✅ |
| `sys_ip_stat` | ✅ |
| `sys_ipfix_destination` | ✅ |
| `sys_ipfix_element` | ✅ |
| `sys_ipfix_irules` | ✅ |
| `sys_iprep_status` | ✅ |
| `sys_license` | ✅ |
| `sys_log` | ✅ |
| `sys_log_config_destination_alertd` | ✅ |
| `sys_log_config_destination_arcsight` | ✅ |
| `sys_log_config_destination_ipfix` | ✅ |
| `sys_log_config_destination_local_database` | ✅ |
| `sys_log_config_destination_local_syslog` | ✅ |
| `sys_log_config_destination_management_port` | ✅ |
| `sys_log_config_destination_remote_high_speed_log` | ✅ |
| `sys_log_config_destination_remote_syslog` | ✅ |
| `sys_log_config_destination_splunk` | ✅ |
| `sys_log_config_filter` | ✅ |
| `sys_log_config_publisher` | ✅ |
| `sys_log_rotate` | ✅ |
| `sys_mac_address` | ✅ |
| `sys_management_dhcp` | ✅ |
| `sys_management_ip` | ✅ |
| `sys_management_ovsdb` | ✅ |
| `sys_management_proxy_config` | ✅ |
| `sys_management_route` | ✅ |
| `sys_mcp_state` | ✅ |
| `sys_memory` | ✅ |
| `sys_nethsm_async_queue_stat` | ✅ |
| `sys_nethsm_pkcs11d_stat` | ✅ |
| `sys_nethsm_sync_queue_stat` | ✅ |
| `sys_ntp` | ✅ |
| `sys_outbound_smtp` | ✅ |
| `sys_performance_all_stats` | ✅ |
| `sys_performance_connections` | ✅ |
| `sys_performance_dnsexpress` | ✅ |
| `sys_performance_dnssec` | ✅ |
| `sys_performance_gtm` | ✅ |
| `sys_performance_ramcache` | ✅ |
| `sys_performance_system` | ✅ |
| `sys_performance_throughput` | ✅ |
| `sys_pfman_consumer` | ✅ |
| `sys_pfman_device` | ✅ |
| `sys_proc_info` | ✅ |
| `sys_provision` | ✅ |
| `sys_pva_traffic` | ✅ |
| `sys_raid_array` | ✅ |
| `sys_raid_bay` | ✅ |
| `sys_raid_disk` | ✅ |
| `sys_ready` | ✅ |
| `sys_scriptd` | ✅ |
| `sys_service` | ✅ |
| `sys_sflow_data_source_http` | ✅ |
| `sys_sflow_data_source_interface` | ✅ |
| `sys_sflow_data_source_system` | ✅ |
| `sys_sflow_data_source_vlan` | ✅ |
| `sys_sflow_global_settings_http` | ✅ |
| `sys_sflow_global_settings_interface` | ✅ |
| `sys_sflow_global_settings_system` | ✅ |
| `sys_sflow_global_settings_vlan` | ✅ |
| `sys_sflow_receiver` | ✅ |
| `sys_smtp_server` | ✅ |
| `sys_snmp` | ✅ |
| `sys_software_block_device_hotfix` | ✅ |
| `sys_software_block_device_image` | ✅ |
| `sys_software_hotfix` | ✅ |
| `sys_software_image` | ✅ |
| `sys_software_signature` | ✅ |
| `sys_software_status` | ✅ |
| `sys_software_update` | ✅ |
| `sys_software_update_status` | ✅ |
| `sys_software_volume` | ✅ |
| `sys_sshd` | ✅ |
| `sys_state_mirroring` | ✅ |
| `sys_sync_sys_files` | ✅ |
| `sys_syslog` | ✅ |
| `sys_tmm_info` | ✅ |
| `sys_tmm_traffic` | ✅ |
| `sys_traffic` | ✅ |
| `sys_turboflex_features` | ✅ |
| `sys_turboflex_profile_all` | ✅ |
| `sys_turboflex_profile_config` | ✅ |
| `sys_turboflex_profile_feature` | ✅ |
| `sys_turboflex_warning` | ✅ |
| `sys_ucs` | ✅ |
| `sys_url_db_download_result` | ✅ |
| `sys_url_db_download_schedule` | ✅ |
| `sys_url_db_url_category` | ✅ |
| `sys_version` | ✅ |
| `util_ipsecalgdb` | ✅ |
| `vcmp_guest` | ✅ |
| `vcmp_traffic_profile` | ✅ |
| `vcmp_virtual_disk` | ✅ |
| `vcmp_virtual_disk_template` | ✅ |
| `wam_ad_policy` | ✅ |
| `wam_application` | ✅ |
| `wam_domain_list` | ✅ |
| `wam_object_type` | ✅ |
| `wam_policy` | ✅ |
| `wam_resource_concat_set` | ✅ |
| `wam_resource_domain_list` | ✅ |
| `wam_resource_url` | ✅ |
| `wom_advertised_route` | ✅ |
| `wom_deduplication` | ✅ |
| `wom_endpoint_discovery` | ✅ |
| `wom_local_endpoint` | ✅ |
| `wom_profile_cifs` | ✅ |
| `wom_profile_isession` | ✅ |
| `wom_profile_mapi` | ✅ |
| `wom_remote_endpoint` | ✅ |
| `wom_server_discovery` | ✅ |

</details>

<details><summary><b>bigip — Python copy ↔ main divergence</b> — 799 shared · 0 main-only · 194 core-only (spec file stems)</summary>

The rust-branch `core/bigip/registry/specs` and `origin/main` `dialects/f5/bigip/registry/specs` have **diverged in both directions** (sampled main-only kinds confirmed absent from core's `OBJECT_SPECS`). Reconcile before/with the Rust port.

**main-only (0)** — present on main, missing from rust-branch core:



**core-only (194)** — present on rust-branch core, not on main (verify renamed vs genuinely extra):

`auth_login_failures`, `cm_add_to_trust`, `cm_cert`, `cm_config_sync`, `cm_failover_status`, `cm_remove_from_trust`, `cm_sha1_fingerprint`, `cm_sniff_updates`, `cm_sync_status`, `cm_watch_devicegroup_device`, `cm_watch_sys_device`, `cm_watch_trafficgroup_device`, `gtm_add`, `gtm_iquery`, `gtm_ldns`, `gtm_monitor_none`, `gtm_path`, `gtm_persist`, `gtm_pool`, `gtm_traffic`, `ltm_classification_auto_update_status`, `ltm_classification_signature_definition`, `ltm_classification_signature_version`, `ltm_classification_signatures`, `ltm_classification_stats_application`, `ltm_classification_stats_url_category`, `ltm_classification_stats_urlcat_cloud`, `ltm_classification_update_signatures`, `ltm_classification_updates`, `ltm_data_group`, `ltm_dns_dns_express_db`, `ltm_monitor_none`, `ltm_nat_stats`, `ltm_policy_strategy`, `ltm_profile_clientssl`, `ltm_profile_ocsp_stapling_params`, `ltm_profile_serverssl`, `ltm_tacdb_query`, `ltm_urlcat_query`, `net_clone_stats`, `net_f5optics`, `net_ike_evt_stat`, `net_ike_msg_stat`, `net_interface_cos`, `net_interface_ddm`, `net_ipsec_ike_sa`, `net_ipsec_ipsec_sa`, `net_ipsec_stat`, `net_lldp_neighbors`, `net_mroute`, `net_packet_tester_security`, `net_rst_cause`, `net_sfc_hop`, `net_sfc_stats`, `net_tunnels_endpoint`, `net_tunnels_fec_stat`, `net_vlan_allowed`, `security_anti_fraud_engine_update`, `security_blacklist_publisher_all_blacklist_publisher`, `security_blacklist_publisher_blacklist_publisher_stats`, `security_blacklist_publisher_by_addr`, `security_blacklist_publisher_by_category`, `security_bot_defense_anomaly`, `security_bot_defense_anomaly_category`, `security_bot_defense_class`, `security_bot_defense_micro_service`, `security_bot_defense_template`, `security_cloud_services_cmd`, `security_datasync_device_stats`, `security_debug_drop_redirect_stats`, `security_dos_auto_thresholds_heavy_urls`, `security_dos_auto_thresholds_stress_based`, `security_dos_auto_thresholds_top_device_ids`, `security_dos_auto_thresholds_top_geolocations`, `security_dos_auto_thresholds_top_source_ips`, `security_dos_auto_thresholds_top_urls`, `security_dos_auto_thresholds_tps_based`, `security_dos_dns_nxdomain_stat`, `security_dos_spva_stats`, `security_dos_stress_stats`, `security_dos_virtual`, `security_firewall_container_stat`, `security_firewall_context_stat`, `security_firewall_current_state`, `security_firewall_fqdn_entity`, `security_firewall_fqdn_info`, `security_firewall_ipi_category_info`, `security_firewall_matching_rule`, `security_firewall_rule_stat`, `security_flowspec_route_injector_flowspec_advertised_route_info`, `security_http_file_type`, `security_http_mandatory_header`, `security_ip_intelligence_info`, `security_log_antifraud_storage_field`, `security_log_network_storage_field`, `security_log_protocol_dns_storage_field`, `security_log_protocol_sip_storage_field`, `security_log_remote_format`, `security_log_storage_field`, `security_malicious_sources_device_ids`, `security_malicious_sources_ip_addresses`, `security_packet_filter_rule_stat`, `security_presentation_tmui_netflow_details`, `security_presentation_tmui_netflow_list`, `security_presentation_tmui_signature_details`, `security_presentation_tmui_signature_list`, `security_protected_servers_netflow_tmc_stat`, `security_protocol_inspection_auto_update_settings`, `security_protocol_inspection_auto_update_status`, `security_protocol_inspection_compliance`, `security_protocol_inspection_compliance_enums`, `security_protocol_inspection_learning_suggestions`, `security_protocol_inspection_profile_status`, `security_protocol_inspection_service`, `security_protocol_inspection_staging`, `security_protocol_inspection_system`, `security_protocol_inspection_updates`, `security_protocol_inspection_virtual_servers`, `security_scrubber_dwbl_scrubber_category_stats`, `security_scrubber_dwbl_scrubber_stat`, `security_scrubber_unredirect`, `sys_air_filter_reset`, `sys_alert_lcd`, `sys_aom`, `sys_availability`, `sys_config_diff`, `sys_cpu`, `sys_crypto_acceleration_strategy`, `sys_crypto_check_cert`, `sys_crypto_crl`, `sys_crypto_encrypted_attributes`, `sys_crypto_fips_by_handle`, `sys_crypto_pkcs12`, `sys_default_config`, `sys_diags_ihealth_request`, `sys_diags_ihealth_result`, `sys_dynad_status`, `sys_ecm_register`, `sys_failover`, `sys_fix_connection`, `sys_fpga_info`, `sys_fpga_turboflex_profile`, `sys_geoip`, `sys_ha_status`, `sys_hardware`, `sys_host_info`, `sys_hypervisor_info`, `sys_icall_event`, `sys_icall_publisher`, `sys_icmp_stat`, `sys_integrity_status_check`, `sys_ip_address`, `sys_ip_stat`, `sys_ipfix_destination`, `sys_ipfix_irules`, `sys_iprep_status`, `sys_license`, `sys_log`, `sys_mac_address`, `sys_mcp_state`, `sys_memory`, `sys_nethsm_async_queue_stat`, `sys_nethsm_pkcs11d_stat`, `sys_nethsm_sync_queue_stat`, `sys_performance_all_stats`, `sys_performance_connections`, `sys_performance_dnsexpress`, `sys_performance_dnssec`, `sys_performance_gtm`, `sys_performance_ramcache`, `sys_performance_system`, `sys_performance_throughput`, `sys_proc_info`, `sys_pva_traffic`, `sys_raid_disk`, `sys_ready`, `sys_sflow_data_source_http`, `sys_sflow_data_source_interface`, `sys_sflow_data_source_system`, `sys_sflow_data_source_vlan`, `sys_software_block_device_hotfix`, `sys_software_block_device_image`, `sys_software_status`, `sys_software_update_status`, `sys_sync_sys_files`, `sys_tmm_info`, `sys_tmm_traffic`, `sys_traffic`, `sys_turboflex_features`, `sys_turboflex_profile_all`, `sys_turboflex_profile_feature`, `sys_turboflex_warning`, `sys_url_db_download_result`, `sys_version`

</details>

<details><summary><b>iRule events</b> — 176 entries · 176 ✅ · 0 need work</summary>

Names **and** all 9 `EventProps` fields compared Python↔Rust.

| event | status |
|---|---|
| `ACCESS2_POLICY_EXPRESSION_EVAL` | ✅ |
| `ACCESS_ACL_ALLOWED` | ✅ |
| `ACCESS_ACL_DENIED` | ✅ |
| `ACCESS_PER_REQUEST_AGENT_EVENT` | ✅ |
| `ACCESS_POLICY_AGENT_EVENT` | ✅ |
| `ACCESS_POLICY_COMPLETED` | ✅ |
| `ACCESS_SAML_ASSERTION` | ✅ |
| `ACCESS_SAML_AUTHN` | ✅ |
| `ACCESS_SAML_SLO_REQ` | ✅ |
| `ACCESS_SAML_SLO_RESP` | ✅ |
| `ACCESS_SESSION_CLOSED` | ✅ |
| `ACCESS_SESSION_STARTED` | ✅ |
| `ADAPT_REQUEST_HEADERS` | ✅ |
| `ADAPT_REQUEST_RESULT` | ✅ |
| `ADAPT_RESPONSE_HEADERS` | ✅ |
| `ADAPT_RESPONSE_RESULT` | ✅ |
| `ANTIFRAUD_ALERT` | ✅ |
| `ANTIFRAUD_LOGIN` | ✅ |
| `ASM_REQUEST_BLOCKING` | ✅ |
| `ASM_REQUEST_DONE` | ✅ |
| `ASM_REQUEST_VIOLATION` | ✅ |
| `ASM_RESPONSE_LOGIN` | ✅ |
| `ASM_RESPONSE_VIOLATION` | ✅ |
| `AUTH_ERROR` | ✅ |
| `AUTH_FAILURE` | ✅ |
| `AUTH_RESULT` | ✅ |
| `AUTH_SUCCESS` | ✅ |
| `AUTH_WANTCREDENTIAL` | ✅ |
| `AVR_CSPM_INJECTION` | ✅ |
| `BOTDEFENSE_ACTION` | ✅ |
| `BOTDEFENSE_REQUEST` | ✅ |
| `CACHE_REQUEST` | ✅ |
| `CACHE_RESPONSE` | ✅ |
| `CACHE_UPDATE` | ✅ |
| `CATEGORY_MATCHED` | ✅ |
| `CLASSIFICATION_DETECTED` | ✅ |
| `CLIENTSSL_CLIENTCERT` | ✅ |
| `CLIENTSSL_CLIENTHELLO` | ✅ |
| `CLIENTSSL_DATA` | ✅ |
| `CLIENTSSL_HANDSHAKE` | ✅ |
| `CLIENTSSL_PASSTHROUGH` | ✅ |
| `CLIENTSSL_SERVERHELLO_SEND` | ✅ |
| `CLIENT_ACCEPTED` | ✅ |
| `CLIENT_CLOSED` | ✅ |
| `CLIENT_DATA` | ✅ |
| `CONNECTOR_OPEN` | ✅ |
| `DIAMETER_EGRESS` | ✅ |
| `DIAMETER_INGRESS` | ✅ |
| `DIAMETER_RETRANSMISSION` | ✅ |
| `DNS_REQUEST` | ✅ |
| `DNS_RESPONSE` | ✅ |
| `ECA_REQUEST_ALLOWED` | ✅ |
| `ECA_REQUEST_DENIED` | ✅ |
| `EPI_NA_CHECK_HTTP_REQUEST` | ✅ |
| `FIX_HEADER` | ✅ |
| `FIX_MESSAGE` | ✅ |
| `FLOW_INIT` | ✅ |
| `GENERICMESSAGE_EGRESS` | ✅ |
| `GENERICMESSAGE_INGRESS` | ✅ |
| `GTP_GPDU_EGRESS` | ✅ |
| `GTP_GPDU_INGRESS` | ✅ |
| `GTP_PRIME_EGRESS` | ✅ |
| `GTP_PRIME_INGRESS` | ✅ |
| `GTP_SIGNALLING_EGRESS` | ✅ |
| `GTP_SIGNALLING_INGRESS` | ✅ |
| `HTML_COMMENT_MATCHED` | ✅ |
| `HTML_TAG_MATCHED` | ✅ |
| `HTTP_CLASS_FAILED` | ✅ |
| `HTTP_CLASS_SELECTED` | ✅ |
| `HTTP_DISABLED` | ✅ |
| `HTTP_PROXY_CONNECT` | ✅ |
| `HTTP_PROXY_REQUEST` | ✅ |
| `HTTP_PROXY_RESPONSE` | ✅ |
| `HTTP_REJECT` | ✅ |
| `HTTP_REQUEST` | ✅ |
| `HTTP_REQUEST_DATA` | ✅ |
| `HTTP_REQUEST_RELEASE` | ✅ |
| `HTTP_REQUEST_SEND` | ✅ |
| `HTTP_RESPONSE` | ✅ |
| `HTTP_RESPONSE_CONTINUE` | ✅ |
| `HTTP_RESPONSE_DATA` | ✅ |
| `HTTP_RESPONSE_RELEASE` | ✅ |
| `ICAP_REQUEST` | ✅ |
| `ICAP_RESPONSE` | ✅ |
| `IN_DOSL7_ATTACK` | ✅ |
| `IP_GTM` | ✅ |
| `IVS_ENTRY_REQUEST` | ✅ |
| `IVS_ENTRY_RESPONSE` | ✅ |
| `JSON_REQUEST` | ✅ |
| `JSON_REQUEST_ERROR` | ✅ |
| `JSON_REQUEST_MISSING` | ✅ |
| `JSON_RESPONSE` | ✅ |
| `JSON_RESPONSE_ERROR` | ✅ |
| `JSON_RESPONSE_MISSING` | ✅ |
| `L7CHECK_CLIENT_DATA` | ✅ |
| `L7CHECK_SERVER_DATA` | ✅ |
| `LB_FAILED` | ✅ |
| `LB_QUEUED` | ✅ |
| `LB_SELECTED` | ✅ |
| `MQTT_CLIENT_DATA` | ✅ |
| `MQTT_CLIENT_EGRESS` | ✅ |
| `MQTT_CLIENT_INGRESS` | ✅ |
| `MQTT_CLIENT_SHUTDOWN` | ✅ |
| `MQTT_SERVER_DATA` | ✅ |
| `MQTT_SERVER_EGRESS` | ✅ |
| `MQTT_SERVER_INGRESS` | ✅ |
| `MR_DATA` | ✅ |
| `MR_EGRESS` | ✅ |
| `MR_FAILED` | ✅ |
| `MR_INGRESS` | ✅ |
| `NAME_RESOLVED` | ✅ |
| `PCP_REQUEST` | ✅ |
| `PCP_RESPONSE` | ✅ |
| `PEM_POLICY` | ✅ |
| `PEM_SUBS_SESS_CREATED` | ✅ |
| `PEM_SUBS_SESS_DELETED` | ✅ |
| `PEM_SUBS_SESS_UPDATED` | ✅ |
| `PERSIST_DOWN` | ✅ |
| `PING_REQUEST_READY` | ✅ |
| `PING_RESPONSE_READY` | ✅ |
| `PROTOCOL_INSPECTION_MATCH` | ✅ |
| `QOE_PARSE_DONE` | ✅ |
| `RADIUS_AAA_ACCT_REQUEST` | ✅ |
| `RADIUS_AAA_ACCT_RESPONSE` | ✅ |
| `RADIUS_AAA_AUTH_REQUEST` | ✅ |
| `RADIUS_AAA_AUTH_RESPONSE` | ✅ |
| `REWRITE_REQUEST` | ✅ |
| `REWRITE_REQUEST_DONE` | ✅ |
| `REWRITE_RESPONSE` | ✅ |
| `REWRITE_RESPONSE_DONE` | ✅ |
| `RTSP_REQUEST` | ✅ |
| `RTSP_REQUEST_DATA` | ✅ |
| `RTSP_RESPONSE` | ✅ |
| `RTSP_RESPONSE_DATA` | ✅ |
| `RULE_INIT` | ✅ |
| `SA_PICKED` | ✅ |
| `SERVERSSL_CLIENTHELLO_SEND` | ✅ |
| `SERVERSSL_DATA` | ✅ |
| `SERVERSSL_HANDSHAKE` | ✅ |
| `SERVERSSL_SERVERCERT` | ✅ |
| `SERVERSSL_SERVERHELLO` | ✅ |
| `SERVER_CLOSED` | ✅ |
| `SERVER_CONNECTED` | ✅ |
| `SERVER_DATA` | ✅ |
| `SERVER_INIT` | ✅ |
| `SIP_REQUEST` | ✅ |
| `SIP_REQUEST_DONE` | ✅ |
| `SIP_REQUEST_SEND` | ✅ |
| `SIP_RESPONSE` | ✅ |
| `SIP_RESPONSE_DONE` | ✅ |
| `SIP_RESPONSE_SEND` | ✅ |
| `SOCKS_REQUEST` | ✅ |
| `SSE_RESPONSE` | ✅ |
| `STREAM_MATCHED` | ✅ |
| `TAP_REQUEST` | ✅ |
| `TCP_GTM` | ✅ |
| `TDS_REQUEST` | ✅ |
| `TDS_RESPONSE` | ✅ |
| `UDP_GTM` | ✅ |
| `USER_REQUEST` | ✅ |
| `USER_RESPONSE` | ✅ |
| `WS_CLIENT_DATA` | ✅ |
| `WS_CLIENT_FRAME` | ✅ |
| `WS_CLIENT_FRAME_DONE` | ✅ |
| `WS_REQUEST` | ✅ |
| `WS_RESPONSE` | ✅ |
| `WS_SERVER_DATA` | ✅ |
| `WS_SERVER_FRAME` | ✅ |
| `WS_SERVER_FRAME_DONE` | ✅ |
| `XML_BEGIN_DOCUMENT` | ✅ |
| `XML_BEGIN_ELEMENT` | ✅ |
| `XML_CDATA` | ✅ |
| `XML_CONTENT_BASED_ROUTING` | ✅ |
| `XML_END_DOCUMENT` | ✅ |
| `XML_END_ELEMENT` | ✅ |
| `XML_EVENT` | ✅ |

</details>

<details><summary><b>F5 profiles</b> — 65 entries · 65 ✅ · 0 need work</summary>

Names compared Python↔Rust (prop-level diff is a follow-up).

| entry | status |
|---|---|
| `ACCESS` | ✅ |
| `ANTIFRAUD` | ✅ |
| `ASM` | ✅ |
| `AUTH` | ✅ |
| `AVR` | ✅ |
| `BOTDEFENSE` | ✅ |
| `CACHE` | ✅ |
| `CATEGORY` | ✅ |
| `CLASSIFICATION` | ✅ |
| `CLIENTSSL` | ✅ |
| `CONNECTOR` | ✅ |
| `DATAGRAM` | ✅ |
| `DIAMETER` | ✅ |
| `DIAMETERSESSION` | ✅ |
| `DIAMETER_ENDPOINT` | ✅ |
| `DNS` | ✅ |
| `DOSL7` | ✅ |
| `ECA` | ✅ |
| `FASTHTTP` | ✅ |
| `FASTL4` | ✅ |
| `FIX` | ✅ |
| `FLOW` | ✅ |
| `GENERICMSG` | ✅ |
| `GTP` | ✅ |
| `HTML` | ✅ |
| `HTTP` | ✅ |
| `HTTP2` | ✅ |
| `HTTP_PROXY_CONNECT` | ✅ |
| `ICAP` | ✅ |
| `IPS` | ✅ |
| `IVS_ENTRY` | ✅ |
| `JSON` | ✅ |
| `L7CHECK` | ✅ |
| `LSN` | ✅ |
| `MQTT` | ✅ |
| `MR` | ✅ |
| `MSSQL` | ✅ |
| `NAME` | ✅ |
| `PCP` | ✅ |
| `PEM` | ✅ |
| `PERSIST` | ✅ |
| `PROTOCOL_INSPECTION` | ✅ |
| `QOE` | ✅ |
| `RADIUS` | ✅ |
| `RADIUS_AAA` | ✅ |
| `REQUESTADAPT` | ✅ |
| `RESPONSEADAPT` | ✅ |
| `REWRITE` | ✅ |
| `RTSP` | ✅ |
| `SCTP` | ✅ |
| `SERVERSSL` | ✅ |
| `SIP` | ✅ |
| `SIPROUTER` | ✅ |
| `SIPSESSION` | ✅ |
| `SOCKS` | ✅ |
| `SSE` | ✅ |
| `SSL_PERSISTENCE` | ✅ |
| `STREAM` | ✅ |
| `TAP` | ✅ |
| `TCP` | ✅ |
| `TDS` | ✅ |
| `UDP` | ✅ |
| `WEBACCELERATION` | ✅ |
| `WS` | ✅ |
| `XML` | ✅ |

</details>

<details><summary><b>Protocol namespaces</b> — 113 entries · 113 ✅ · 0 need work</summary>

Names compared Python↔Rust (prop-level diff is a follow-up).

| entry | status |
|---|---|
| `AAA` | ✅ |
| `ACCESS` | ✅ |
| `ACCESS2` | ✅ |
| `ACL` | ✅ |
| `ADAPT` | ✅ |
| `AES` | ✅ |
| `AM` | ✅ |
| `ANTIFRAUD` | ✅ |
| `ASM` | ✅ |
| `ASN1` | ✅ |
| `AUTH` | ✅ |
| `AVR` | ✅ |
| `BIGPROTO` | ✅ |
| `BIGTCP` | ✅ |
| `BOTDEFENSE` | ✅ |
| `BWC` | ✅ |
| `CACHE` | ✅ |
| `CATEGORY` | ✅ |
| `CLASSIFICATION` | ✅ |
| `CLASSIFY` | ✅ |
| `COMPRESS` | ✅ |
| `CONNECTOR` | ✅ |
| `CRYPTO` | ✅ |
| `DATAGRAM` | ✅ |
| `DECOMPRESS` | ✅ |
| `DEMANGLE` | ✅ |
| `DHCP` | ✅ |
| `DHCPv4` | ✅ |
| `DHCPv6` | ✅ |
| `DIAG` | ✅ |
| `DIAMETER` | ✅ |
| `DNS` | ✅ |
| `DNSMSG` | ✅ |
| `DOSL7` | ✅ |
| `DSLITE` | ✅ |
| `ECA` | ✅ |
| `FIX` | ✅ |
| `FLOW` | ✅ |
| `FLOWTABLE` | ✅ |
| `FTP` | ✅ |
| `GENERICMESSAGE` | ✅ |
| `GTP` | ✅ |
| `HA` | ✅ |
| `HSL` | ✅ |
| `HTML` | ✅ |
| `HTTP` | ✅ |
| `HTTP2` | ✅ |
| `HTTPLOG` | ✅ |
| `ICAP` | ✅ |
| `IKE` | ✅ |
| `ILX` | ✅ |
| `IMAP` | ✅ |
| `IP` | ✅ |
| `IPFIX` | ✅ |
| `ISESSION` | ✅ |
| `ISTATS` | ✅ |
| `IVS_ENTRY` | ✅ |
| `JSON` | ✅ |
| `L7CHECK` | ✅ |
| `LB` | ✅ |
| `LDAP` | ✅ |
| `LINE` | ✅ |
| `LINK` | ✅ |
| `LSN` | ✅ |
| `MESSAGE` | ✅ |
| `MQTT` | ✅ |
| `MR` | ✅ |
| `NAME` | ✅ |
| `NSH` | ✅ |
| `NTLM` | ✅ |
| `OFFBOX` | ✅ |
| `ONECONNECT` | ✅ |
| `PCP` | ✅ |
| `PEM` | ✅ |
| `PLUGIN` | ✅ |
| `POLICY` | ✅ |
| `POP3` | ✅ |
| `PROFILE` | ✅ |
| `PROTOCOL_INSPECTION` | ✅ |
| `PSC` | ✅ |
| `PSM` | ✅ |
| `QOE` | ✅ |
| `RADIUS` | ✅ |
| `RESOLV` | ✅ |
| `RESOLVER` | ✅ |
| `REST` | ✅ |
| `REWRITE` | ✅ |
| `ROUTE` | ✅ |
| `RTSP` | ✅ |
| `SCTP` | ✅ |
| `SDP` | ✅ |
| `SIP` | ✅ |
| `SIPALG` | ✅ |
| `SMTPS` | ✅ |
| `SOCKS` | ✅ |
| `SSE` | ✅ |
| `SSL` | ✅ |
| `STATS` | ✅ |
| `STREAM` | ✅ |
| `TAP` | ✅ |
| `TCP` | ✅ |
| `TDS` | ✅ |
| `TMM` | ✅ |
| `UDP` | ✅ |
| `URI` | ✅ |
| `VALIDATE` | ✅ |
| `VDI` | ✅ |
| `WAM` | ✅ |
| `WEBSSO` | ✅ |
| `WS` | ✅ |
| `X509` | ✅ |
| `XLAT` | ✅ |
| `XML` | ✅ |

</details>

