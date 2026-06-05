<!-- markdownlint-disable MD013 MD033 -->
# Rust-rewrite registry audit

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

- [x] **tcl** (1e3a71b7 / 1abc0d35) — **NAMES DIFFER** (104 missing, 16 extra) + thin hover/forms. See §1.
- [x] **stdlib** (1e3a71b7 / 1abc0d35) — **DATA GAPS** (`side_effects` 12→0; `required_package` 110→110 ✅ restored).
- [x] **tcllib** (1e3a71b7 / 1abc0d35) — **DATA GAPS** (`forms`, `side_effects`, `return_value`, `examples` dropped).
- [x] **irules** (1e3a71b7 / 1abc0d35) — **DATA GAPS (severe)** — see §2. Names 1015=1015, but almost all rich data dropped.
- [x] **iapps** (1e3a71b7 / 1abc0d35) — **minor** (only `forms` 49→0).
- [x] **tk** (1e3a71b7 / 1abc0d35) — **DATA GAPS** (`options`, `side_effects`, `subcommands` dropped; `required_package` 55→55 ✅ restored).
- [x] **expect** (1e3a71b7 / 1abc0d35) — **DATA GAPS** (`forms`, `options` dropped).
- [x] **sdc-base** (1e3a71b7 / 1abc0d35) — **minor** (`forms`, `arg_roles`).
- [x] **synopsys** (1e3a71b7 / 1abc0d35) — **minor** (`forms` only).
- [x] **cadence** (1e3a71b7 / 1abc0d35) — **minor** (`forms` only).
- [x] **xilinx** (1e3a71b7 / 1abc0d35) — **minor** (`forms` only).
- [x] **quartus** (1e3a71b7 / 1abc0d35) — **minor** (`forms` only).
- [x] **mentor** (1e3a71b7 / 1abc0d35) — **minor** (`forms` only).

### Meta / data registries

- [x] **iRule events** (1e3a71b7 / 1abc0d35) — **OK** ✅ `events.rs` vs `namespace_data.EVENT_PROPS`: 176=176 names, all 9 prop fields match for every event. See §3.
- [x] **F5 profiles** (1e3a71b7 / 1abc0d35) — **OK** ✅ `profiles.rs` vs `PROFILE_SPECS`: 65=65 names. See §3.
- [x] **Protocol namespaces** (1e3a71b7 / 1abc0d35) — **OK** ✅ `profiles.rs` vs `PROTOCOL_NAMESPACE_SPECS`: 113=113 names. See §3.
- [x] **BigIP object registry** (1e3a71b7 / 1abc0d35) — **UNPORTED** ❌ Rust has **no** BigIP registry (0 files). Python copy also drifted behind main. See §4.
- [x] **Codegen / lowering hooks** (1e3a71b7 / 1abc0d35) — **modelling diff** — Rust stamps hook IDs on specs; Python dispatches via `core/compiler/codegen/`. See §5.
- [x] **Secondary infra** (taint, type-hints, operators, stub-overlay, runtime) (1e3a71b7 / 1abc0d35) — present both sides, **light-touch** verified. See §6.

---

## Cross-cutting Rust-port gaps (systematic)

These patterns repeat across *every* command registry and are the bulk of the
data loss. They are almost certainly mechanical port omissions, not deliberate:

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

## §4 — BigIP object registry (UNPORTED ❌)

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
| `tcl` | 214 | 126 | 104 | 16 | NAMES DIFFER |
| `stdlib` | 225 | 225 | 0 | 0 | DATA GAPS |
| `tcllib` | 206 | 206 | 0 | 0 | DATA GAPS |
| `irules` | 1015 | 1015 | 0 | 0 | DATA GAPS |
| `iapps` | 49 | 49 | 0 | 0 | DATA GAPS |
| `tk` | 55 | 55 | 0 | 0 | DATA GAPS |
| `expect` | 35 | 35 | 0 | 0 | DATA GAPS |
| `sdc-base` | 61 | 61 | 0 | 0 | DATA GAPS |
| `synopsys` | 68 | 68 | 0 | 0 | DATA GAPS |
| `cadence` | 56 | 56 | 0 | 0 | DATA GAPS |
| `xilinx` | 64 | 64 | 0 | 0 | DATA GAPS |
| `quartus` | 48 | 48 | 0 | 0 | DATA GAPS |
| `mentor` | 49 | 49 | 0 | 0 | DATA GAPS |

### Command registries — data completeness (commands carrying each dimension)

Only dimensions where Python and Rust differ are shown. `py→rust`.

- **`tcl`** — `forms` 199→1, `hover` 211→125, `hover_synopsis` 196→125, `arity_bounded` 160→119, `side_effects` 84→46, `hover_return_value` 9→0, `options` 19→12, `arg_types` 19→13, `hover_examples` 1→0, `const_fold` 10→12, `required_package` 0→2, `subcommands` 13→18, `arg_roles` 30→36, `codegen_hook` 0→7, `return_type` 92→110, `lowering_hook` 0→23, `traits` 42→104
- **`stdlib`** — `side_effects` 12→0, `forms` 2→0, `subcommands` 2→0, `arg_roles` 1→0, `options` 1→0, `traits` 23→24
- **`tcllib`** — `forms` 206→0, `hover_return_value` 68→0, `side_effects` 62→0, `hover_examples` 25→0, `arg_roles` 17→3, `subcommands` 3→0
- **`irules`** — `side_effects` 1002→0, `subcommands` 47→0, `arg_roles` 4→5, `options` 54→55, `lowering_hook` 0→2, `arity_bounded` 31→34, `traits` 62→69
- **`iapps`** — `forms` 49→0
- **`tk`** — `forms` 55→0, `side_effects` 55→0, `options` 44→0, `subcommands` 17→0
- **`expect`** — `forms` 35→0, `options` 26→0, `arg_roles` 1→0
- **`sdc-base`** — `forms` 61→0, `arg_roles` 3→0
- **`synopsys`** — `forms` 68→0
- **`cadence`** — `forms` 56→0
- **`xilinx`** — `forms` 64→0, `arity_bounded` 18→19
- **`quartus`** — `forms` 48→0
- **`mentor`** — `forms` 49→0

### Command registries — value mismatches on common commands

| Registry | field | # mismatched | examples |
|---|---|--:|---|
| `tcl` | summary | 94 | `after`, `append`, `apply` |
| `tcl` | synopsis | 37 | `apply`, `binary`, `classvariable` |
| `tcl` | body_kind | 3 | `oo::abstract`, `oo::configurable`, `oo::singleton` |
| `tcl` | return_type | 96 | `after`, `append`, `apply` |
| `tcl` | arity_min | 8 | `flush`, `oo::abstract`, `oo::class` |
| `tcl` | arity_max | 3 | `fcopy`, `oo::copy`, `source` |
| `stdlib` | summary | 3 | `tcltest::limitConstraints`, `tcltest::matchDirectories`, `tcltest::skipDirectories` |
| `stdlib` | synopsis | 7 | `history`, `http::config`, `http::cookiejar` |
| `stdlib` | body_kind | 1 | `tcltest::test` |
| `stdlib` | return_type | 3 | `http::requestHeaders`, `http::responseHeaders`, `http::responseInfo` |
| `tcllib` | synopsis | 5 | `csv::split`, `mime::initialize`, `smtp::sendmessage` |
| `tcllib` | body_kind | 5 | `snit::compile`, `snit::macro`, `snit::type` |
| `tcllib` | return_type | 6 | `ip::collapse`, `ip::is`, `ip::subtract` |
| `irules` | body_kind | 1 | `after` |
| `irules` | return_type | 1 | `HSL::open` |
| `irules` | arity_min | 1 | `peer` |
| `irules` | arity_max | 3 | `clientside`, `peer`, `serverside` |
| `tk` | synopsis | 15 | `bind`, `clipboard`, `event` |
| `expect` | summary | 1 | `fork` |
| `expect` | synopsis | 8 | `exit`, `expect`, `interact` |
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

<details><summary><b>tcl</b> — 230 entries · 0 ✅ · 230 need work</summary>

| entry | status |
|---|---|
| `!` | ✗ missing in rust |
| `!=` | ✗ missing in rust |
| `%` | ✗ missing in rust |
| `&` | ✗ missing in rust |
| `&&` | ✗ missing in rust |
| `*` | ✗ missing in rust |
| `**` | ✗ missing in rust |
| `+` | ✗ missing in rust |
| `-` | ✗ missing in rust |
| `/` | ✗ missing in rust |
| `::tcl::build-info` | ✗ missing in rust |
| `::tcl::idna` | ➕ rust-only |
| `::tcl::mathop::!` | ✗ missing in rust |
| `::tcl::mathop::!=` | ✗ missing in rust |
| `::tcl::mathop::%` | ✗ missing in rust |
| `::tcl::mathop::&` | ✗ missing in rust |
| `::tcl::mathop::&&` | ✗ missing in rust |
| `::tcl::mathop::*` | ✗ missing in rust |
| `::tcl::mathop::**` | ✗ missing in rust |
| `::tcl::mathop::+` | ✗ missing in rust |
| `::tcl::mathop::-` | ✗ missing in rust |
| `::tcl::mathop::/` | ✗ missing in rust |
| `::tcl::mathop::<` | ✗ missing in rust |
| `::tcl::mathop::<<` | ✗ missing in rust |
| `::tcl::mathop::<=` | ✗ missing in rust |
| `::tcl::mathop::==` | ✗ missing in rust |
| `::tcl::mathop::>` | ✗ missing in rust |
| `::tcl::mathop::>=` | ✗ missing in rust |
| `::tcl::mathop::>>` | ✗ missing in rust |
| `::tcl::mathop::@` | ✗ missing in rust |
| `::tcl::mathop::^` | ✗ missing in rust |
| `::tcl::mathop::eq` | ✗ missing in rust |
| `::tcl::mathop::in` | ✗ missing in rust |
| `::tcl::mathop::max` | ✗ missing in rust |
| `::tcl::mathop::min` | ✗ missing in rust |
| `::tcl::mathop::ne` | ✗ missing in rust |
| `::tcl::mathop::ni` | ✗ missing in rust |
| `::tcl::mathop::|` | ✗ missing in rust |
| `::tcl::mathop::||` | ✗ missing in rust |
| `::tcl::mathop::~` | ✗ missing in rust |
| `::tcl::process` | ➕ rust-only |
| `::tcl::unsupported::corotype` | ➕ rust-only |
| `<` | ✗ missing in rust |
| `<<` | ✗ missing in rust |
| `<=` | ✗ missing in rust |
| `==` | ✗ missing in rust |
| `>` | ✗ missing in rust |
| `>=` | ✗ missing in rust |
| `>>` | ✗ missing in rust |
| `@` | ✗ missing in rust |
| `^` | ✗ missing in rust |
| `after` | `forms` `sum≠` |
| `append` | `forms` `sum≠` |
| `apply` | `forms` `sfx` `sum≠` `syn≠` |
| `array` | `forms` `sum≠` |
| `auto_execok` | ✗ missing in rust |
| `auto_import` | ✗ missing in rust |
| `auto_load` | ✗ missing in rust |
| `auto_mkindex` | ✗ missing in rust |
| `auto_mkindex_old` | ✗ missing in rust |
| `auto_qualify` | ✗ missing in rust |
| `auto_reset` | ✗ missing in rust |
| `bgerror` | ✗ missing in rust |
| `binary` | `forms` `sfx` `sum≠` `syn≠` |
| `break` | `forms` `sum≠` |
| `catch` | `forms` `sfx` `sum≠` |
| `cd` | `forms` `sum≠` |
| `chan` | `forms` `opt` `sfx` `sum≠` |
| `classvariable` | `forms` `sum≠` `syn≠` |
| `clock` | `forms` `opt` `sum≠` `syn≠` |
| `close` | `forms` `sum≠` |
| `concat` | `forms` `sum≠` `syn≠` |
| `const` | ➕ rust-only |
| `continue` | `forms` `sum≠` |
| `coroinject` | ➕ rust-only |
| `coroprobe` | ➕ rust-only |
| `coroutine` | `forms` `sfx` `sum≠` `syn≠` |
| `dict` | `forms` `sfx` `sum≠` `syn≠` |
| `disabled_in_irules` | ➕ rust-only |
| `encoding` | `forms` `opt` `sfx` `sum≠` `syn≠` |
| `eof` | `forms` `sum≠` |
| `eq` | ✗ missing in rust |
| `error` | `forms` `sum≠` |
| `eval` | `forms` `sfx` |
| `exec` | `forms` `sum≠` `syn≠` |
| `exit` | `forms` `sum≠` |
| `expr` | `forms` `sfx` `ret` `sum≠` |
| `fblocked` | `forms` `sum≠` |
| `fconfigure` | `forms` |
| `fcopy` | `forms` `sum≠` `arity≠` |
| `file` | `forms` `sum≠` |
| `fileevent` | `forms` `sum≠` |
| `filename` | ✗ missing in rust |
| `flush` | `forms` `sum≠` `arity≠` |
| `for` | `forms` `sfx` |
| `foreach` | `forms` `sfx` `sum≠` `syn≠` |
| `foreachLine` | ➕ rust-only |
| `format` | `forms` `sum≠` `syn≠` |
| `gets` | `forms` `sum≠` |
| `glob` | `forms` `ret` |
| `global` | `forms` `sum≠` |
| `http` | ✗ missing in rust |
| `if` | `forms` `sfx` `sum≠` `syn≠` |
| `in` | ✗ missing in rust |
| `incr` | `forms` `sfx` `sum≠` |
| `info` | `forms` `sum≠` |
| `interp` | `forms` `sfx` `sum≠` `syn≠` |
| `join` | `forms` `sum≠` |
| `lappend` | `forms` `sum≠` |
| `lassign` | `forms` `sfx` `sum≠` |
| `lindex` | `forms` `sum≠` |
| `linsert` | `forms` `sum≠` `syn≠` |
| `list` | `forms` `sum≠` `syn≠` |
| `llength` | `forms` `sum≠` |
| `lmap` | `forms` `sfx` `sum≠` |
| `load` | `forms` `sfx` `sum≠` `syn≠` |
| `lpop` | ➕ rust-only |
| `lrange` | `forms` `sum≠` |
| `lremove` | `forms` `sum≠` |
| `lrepeat` | `forms` `sum≠` |
| `lreplace` | `forms` `sum≠` `syn≠` |
| `lreverse` | `forms` `sum≠` |
| `lsearch` | `forms` `sum≠` `syn≠` |
| `lseq` | `forms` |
| `lset` | `forms` `sfx` `sum≠` |
| `lsort` | `sum≠` `syn≠` |
| `max` | ✗ missing in rust |
| `memory` | ✗ missing in rust |
| `min` | ✗ missing in rust |
| `my` | `forms` `sum≠` |
| `namespace` | `forms` `sum≠` |
| `ne` | ✗ missing in rust |
| `next` | `forms` `sum≠` |
| `nextto` | ✗ missing in rust |
| `ni` | ✗ missing in rust |
| `oo::abstract` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `oo::class` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `oo::configurable` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `oo::copy` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `oo::define` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `oo::objdefine` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `oo::object` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `oo::singleton` | `forms` `sfx` `sum≠` `syn≠` `arity≠` |
| `open` | `forms` |
| `package` | `forms` `sum≠` `syn≠` |
| `parray` | `forms` `sum≠` |
| `pid` | `forms` `sfx` `sum≠` |
| `pkg::create` | ✗ missing in rust |
| `pkg_mkindex` | ✗ missing in rust |
| `proc` | `forms` `sfx` `sum≠` |
| `puts` | `forms` `sum≠` |
| `pwd` | ✗ missing in rust |
| `re_quote` | `forms` `ret` `sum≠` `syn≠` |
| `read` | `forms` `sum≠` |
| `readFile` | ➕ rust-only |
| `regex::quote` | ✗ missing in rust |
| `regex_quote` | `forms` `ret` `sum≠` `syn≠` |
| `regexp` | `forms` `ret` |
| `regexp::quote` | ✗ missing in rust |
| `regexp_quote` | ➕ rust-only |
| `registry` | `forms` |
| `regsub` | `forms` `ret` |
| `rename` | `forms` `sfx` `sum≠` |
| `return` | `forms` |
| `scan` | `forms` `sfx` `sum≠` `syn≠` |
| `seek` | `forms` |
| `self` | `forms` `sum≠` |
| `set` | `forms` `sfx` `sum≠` |
| `socket` | `forms` `opt` `sum≠` `syn≠` |
| `source` | `forms` `arity≠` |
| `split` | `forms` `sum≠` |
| `string` | `forms` |
| `subst` | `forms` `opt` `sfx` `ret` `syn≠` |
| `switch` | `forms` `sfx` |
| `tailcall` | `forms` `sum≠` |
| `tcl::build-info` | ✗ missing in rust |
| `tcl::idna` | ➕ rust-only |
| `tcl::mathop` | ➕ rust-only |
| `tcl::mathop::!` | ✗ missing in rust |
| `tcl::mathop::!=` | ✗ missing in rust |
| `tcl::mathop::%` | ✗ missing in rust |
| `tcl::mathop::&` | ✗ missing in rust |
| `tcl::mathop::&&` | ✗ missing in rust |
| `tcl::mathop::*` | ✗ missing in rust |
| `tcl::mathop::**` | ✗ missing in rust |
| `tcl::mathop::+` | ✗ missing in rust |
| `tcl::mathop::-` | ✗ missing in rust |
| `tcl::mathop::/` | ✗ missing in rust |
| `tcl::mathop::<` | ✗ missing in rust |
| `tcl::mathop::<<` | ✗ missing in rust |
| `tcl::mathop::<=` | ✗ missing in rust |
| `tcl::mathop::==` | ✗ missing in rust |
| `tcl::mathop::>` | ✗ missing in rust |
| `tcl::mathop::>=` | ✗ missing in rust |
| `tcl::mathop::>>` | ✗ missing in rust |
| `tcl::mathop::@` | ✗ missing in rust |
| `tcl::mathop::^` | ✗ missing in rust |
| `tcl::mathop::eq` | ✗ missing in rust |
| `tcl::mathop::in` | ✗ missing in rust |
| `tcl::mathop::max` | ✗ missing in rust |
| `tcl::mathop::min` | ✗ missing in rust |
| `tcl::mathop::ne` | ✗ missing in rust |
| `tcl::mathop::ni` | ✗ missing in rust |
| `tcl::mathop::|` | ✗ missing in rust |
| `tcl::mathop::||` | ✗ missing in rust |
| `tcl::mathop::~` | ✗ missing in rust |
| `tcl::process` | ➕ rust-only |
| `tcl_findLibrary` | ✗ missing in rust |
| `tell` | `forms` `sum≠` |
| `throw` | `forms` `sum≠` |
| `time` | `forms` `sfx` `sum≠` |
| `timerate` | ➕ rust-only |
| `trace` | `forms` `sfx` `sum≠` |
| `try` | `forms` `sfx` `sum≠` |
| `unknown` | `forms` `sfx` `sum≠` `syn≠` |
| `unload` | `forms` `opt` `sfx` `sum≠` `syn≠` |
| `unset` | `forms` `sum≠` |
| `update` | `forms` `sfx` `sum≠` |
| `uplevel` | `forms` `sfx` `sum≠` |
| `upvar` | `forms` `sum≠` |
| `variable` | `forms` `sum≠` |
| `vwait` | `forms` `opt` `sfx` `sum≠` `syn≠` |
| `while` | `forms` `sfx` `sum≠` |
| `writeFile` | ➕ rust-only |
| `yield` | `forms` `sfx` `sum≠` |
| `yieldto` | `forms` `sfx` `sum≠` `syn≠` |
| `zlib` | `forms` |
| `|` | ✗ missing in rust |
| `||` | ✗ missing in rust |
| `~` | ✗ missing in rust |

</details>

<details><summary><b>stdlib</b> — 225 entries · 111 ✅ · 114 need work</summary>

| entry | status |
|---|---|
| `gettimes` | ✅ |
| `history` | `forms` `sub` `sfx` `syn≠` |
| `http::cleanup` | `pkg` |
| `http::code` | `pkg` |
| `http::config` | `pkg` `syn≠` |
| `http::cookiejar` | `sfx` `pkg` `syn≠` |
| `http::data` | `pkg` |
| `http::error` | `pkg` |
| `http::formatQuery` | `pkg` |
| `http::geturl` | `sfx` `pkg` |
| `http::meta` | `pkg` |
| `http::ncode` | `pkg` |
| `http::postError` | `pkg` |
| `http::quoteString` | `pkg` |
| `http::reasonPhrase` | `pkg` |
| `http::register` | `pkg` |
| `http::registerError` | `sfx` `pkg` |
| `http::requestHeaderValue` | `pkg` |
| `http::requestHeaders` | `pkg` `syn≠` |
| `http::requestLine` | `pkg` |
| `http::reset` | `pkg` |
| `http::responseBody` | `pkg` |
| `http::responseCode` | `pkg` |
| `http::responseHeaderValue` | `pkg` |
| `http::responseHeaders` | `pkg` `syn≠` |
| `http::responseInfo` | `pkg` |
| `http::responseLine` | `pkg` |
| `http::size` | `pkg` |
| `http::status` | `pkg` |
| `http::unregister` | `pkg` |
| `http::wait` | `pkg` |
| `lgen` | ✅ |
| `lstring` | ✅ |
| `msgcat::mc` | `sfx` `pkg` |
| `msgcat::mcexists` | `pkg` |
| `msgcat::mcflmset` | `pkg` |
| `msgcat::mcflset` | `pkg` |
| `msgcat::mcforgetpackage` | `pkg` |
| `msgcat::mcload` | `pkg` |
| `msgcat::mcloadedlocales` | `pkg` |
| `msgcat::mclocale` | `pkg` |
| `msgcat::mcmax` | `pkg` |
| `msgcat::mcmset` | `pkg` |
| `msgcat::mcn` | `pkg` |
| `msgcat::mcpackageconfig` | `pkg` |
| `msgcat::mcpackagelocale` | `pkg` |
| `msgcat::mcpackagenamespaceget` | `pkg` |
| `msgcat::mcpreferences` | `pkg` |
| `msgcat::mcset` | `pkg` |
| `msgcat::mcunknown` | `pkg` |
| `msgcat::mcutil` | `pkg` |
| `noop` | ✅ |
| `pkg::create` | ✅ |
| `pkg_mkIndex` | `sfx` |
| `platform::generic` | `pkg` |
| `platform::identify` | `sfx` `pkg` |
| `platform::patterns` | `pkg` |
| `platform::shell::generic` | `pkg` |
| `platform::shell::identify` | `pkg` |
| `safe::interpAddToAccessPath` | `pkg` |
| `safe::interpConfigure` | `pkg` |
| `safe::interpCreate` | `sfx` `pkg` |
| `safe::interpDelete` | `pkg` |
| `safe::interpFindInAccessPath` | `pkg` |
| `safe::interpInit` | `pkg` |
| `safe::setLogCmd` | `pkg` |
| `safe::setSyncMode` | `pkg` |
| `tcl::OptKeyDelete` | `pkg` |
| `tcl::OptKeyError` | `pkg` |
| `tcl::OptKeyParse` | `pkg` |
| `tcl::OptKeyRegister` | `pkg` |
| `tcl::OptParse` | `pkg` |
| `tcl::OptProc` | `sfx` `pkg` |
| `tcl::OptProcArgGiven` | `pkg` |
| `tcl::idna::decode` | `pkg` |
| `tcl::idna::encode` | `pkg` |
| `tcl::tm::path` | `sub` `sfx` `syn≠` |
| `tcl::tm::roots` | ✅ |
| `tcl_endOfWord` | ✅ |
| `tcl_startOfNextWord` | ✅ |
| `tcl_startOfPreviousWord` | ✅ |
| `tcl_wordBreakAfter` | `sfx` |
| `tcl_wordBreakBefore` | ✅ |
| `tcltest::bytestring` | `pkg` |
| `tcltest::cleanupTests` | `pkg` |
| `tcltest::configure` | `pkg` |
| `tcltest::customMatch` | `pkg` |
| `tcltest::debug` | `pkg` |
| `tcltest::errorChannel` | `pkg` |
| `tcltest::errorFile` | `pkg` |
| `tcltest::getMatchingFiles` | `pkg` |
| `tcltest::interpreter` | `pkg` |
| `tcltest::limitConstraints` | `pkg` `sum≠` |
| `tcltest::loadFile` | `pkg` |
| `tcltest::loadScript` | `pkg` |
| `tcltest::loadTestedCommands` | `pkg` |
| `tcltest::mainThread` | `pkg` |
| `tcltest::makeDirectory` | `pkg` |
| `tcltest::makeFile` | `pkg` |
| `tcltest::match` | `pkg` |
| `tcltest::matchDirectories` | `pkg` `sum≠` |
| `tcltest::matchFiles` | `pkg` |
| `tcltest::normalizeMsg` | `pkg` |
| `tcltest::normalizePath` | `pkg` |
| `tcltest::outputChannel` | `pkg` |
| `tcltest::outputFile` | `pkg` |
| `tcltest::preserveCore` | `pkg` |
| `tcltest::removeDirectory` | `pkg` |
| `tcltest::removeFile` | `pkg` |
| `tcltest::restoreState` | `pkg` |
| `tcltest::runAllTests` | `pkg` |
| `tcltest::saveState` | `pkg` |
| `tcltest::singleProcess` | `pkg` |
| `tcltest::skip` | `pkg` |
| `tcltest::skipDirectories` | `pkg` `sum≠` |
| `tcltest::skipFiles` | `pkg` |
| `tcltest::temporaryDirectory` | `pkg` |
| `tcltest::test` | `forms` `opt` `sfx` `pkg` `syn≠` |
| `tcltest::testConstraint` | `pkg` |
| `tcltest::testsDirectory` | `pkg` |
| `tcltest::threadReap` | `pkg` |
| `tcltest::verbose` | `pkg` |
| `tcltest::viewFile` | `pkg` |
| `tcltest::workingDirectory` | `pkg` |
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

<details><summary><b>tcllib</b> — 206 entries · 0 ✅ · 206 need work</summary>

| entry | status |
|---|---|
| `base64::decode` | `forms` `ex` `ret` |
| `base64::encode` | `forms` `sfx` `ex` `ret` |
| `cmdline::getArgv0` | `forms` `ret` |
| `cmdline::getKnownOpt` | `forms` `ret` |
| `cmdline::getKnownOptions` | `forms` `ret` |
| `cmdline::getfiles` | `forms` `sfx` `ret` |
| `cmdline::getopt` | `forms` `sfx` `ret` |
| `cmdline::getoptions` | `forms` `ex` `ret` |
| `cmdline::typedGetopt` | `forms` `ret` |
| `cmdline::typedGetoptions` | `forms` `ret` |
| `cmdline::typedUsage` | `forms` `ret` |
| `cmdline::usage` | `forms` `ret` |
| `csv::iscomplete` | `forms` |
| `csv::join` | `forms` `ex` `ret` |
| `csv::joinlist` | `forms` |
| `csv::joinmatrix` | `forms` |
| `csv::read2matrix` | `forms` `ret` |
| `csv::read2queue` | `forms` `sfx` |
| `csv::report` | `forms` |
| `csv::split` | `forms` `sfx` `ex` `ret` `syn≠` |
| `csv::split2matrix` | `forms` |
| `csv::split2queue` | `forms` |
| `csv::writematrix` | `forms` `sfx` |
| `csv::writequeue` | `forms` `sfx` |
| `dns::address` | `forms` `ret` |
| `dns::cleanup` | `forms` |
| `dns::cname` | `forms` |
| `dns::configure` | `forms` `sfx` |
| `dns::dump` | `forms` |
| `dns::error` | `forms` |
| `dns::errorcode` | `forms` |
| `dns::name` | `forms` `ret` |
| `dns::reset` | `forms` `sfx` |
| `dns::resolve` | `forms` `sfx` `ex` `ret` |
| `dns::result` | `forms` |
| `dns::status` | `forms` |
| `dns::wait` | `forms` `sfx` |
| `fileutil::appendToFile` | `forms` `sfx` |
| `fileutil::cat` | `forms` `sfx` `ex` `ret` |
| `fileutil::fileType` | `forms` `sfx` |
| `fileutil::find` | `forms` `sfx` |
| `fileutil::findByPattern` | `forms` `sfx` |
| `fileutil::foreachLine` | `forms` `sfx` |
| `fileutil::fullnormalize` | `forms` `sfx` |
| `fileutil::grep` | `forms` `sfx` |
| `fileutil::insertIntoFile` | `forms` `sfx` |
| `fileutil::install` | `forms` `sfx` |
| `fileutil::jail` | `forms` |
| `fileutil::lexnormalize` | `forms` |
| `fileutil::maketempdir` | `forms` `sfx` |
| `fileutil::relative` | `forms` |
| `fileutil::relativeUrl` | `forms` |
| `fileutil::removeFromFile` | `forms` `sfx` |
| `fileutil::replaceInFile` | `forms` `sfx` |
| `fileutil::stripN` | `forms` |
| `fileutil::stripPwd` | `forms` |
| `fileutil::tempdir` | `forms` `ret` |
| `fileutil::tempdirReset` | `forms` `sfx` |
| `fileutil::tempfile` | `forms` `ret` |
| `fileutil::test` | `forms` |
| `fileutil::touch` | `forms` `sfx` |
| `fileutil::updateInPlace` | `forms` `sfx` |
| `fileutil::writeFile` | `forms` `ex` |
| `html::html_entities` | `forms` `sfx` `ex` `ret` |
| `html::tagstrip` | `forms` `ret` |
| `ip::collapse` | `forms` `ret` |
| `ip::contract` | `forms` `ret` |
| `ip::equal` | `forms` `ret` |
| `ip::is` | `forms` `ret` |
| `ip::mask` | `forms` `ret` |
| `ip::normalize` | `forms` `sfx` `ex` `ret` |
| `ip::prefix` | `forms` `ex` `ret` |
| `ip::subtract` | `forms` `ret` |
| `ip::type` | `forms` `ret` |
| `ip::version` | `forms` `ret` |
| `json::dict2json` | `forms` `ex` `ret` |
| `json::json2dict` | `forms` `sfx` `ex` `ret` |
| `json::list2json` | `forms` |
| `json::many-json2dict` | `forms` |
| `json::string2json` | `forms` |
| `json::validate` | `forms` |
| `logger::disable` | `forms` `sfx` |
| `logger::enable` | `forms` `sfx` |
| `logger::import` | `forms` `sfx` |
| `logger::init` | `forms` `sfx` `ex` `ret` |
| `logger::initNamespace` | `forms` `sfx` |
| `logger::levels` | `forms` `ret` |
| `logger::servicecmd` | `forms` `ret` |
| `logger::services` | `forms` `ret` |
| `logger::setlevel` | `forms` `sfx` |
| `logger::walk` | `forms` `sfx` |
| `math::statistics::analyse-Kruskal-Wallis` | `forms` |
| `math::statistics::autocorr` | `forms` |
| `math::statistics::basic-stats` | `forms` `ret` |
| `math::statistics::control-Rchart` | `forms` |
| `math::statistics::control-xbar` | `forms` |
| `math::statistics::corr` | `forms` |
| `math::statistics::crosscorr` | `forms` |
| `math::statistics::filter` | `forms` |
| `math::statistics::group-rank` | `forms` |
| `math::statistics::histogram` | `forms` |
| `math::statistics::histogram-alt` | `forms` |
| `math::statistics::interval-mean-stdev` | `forms` |
| `math::statistics::lillieforsFit` | `forms` |
| `math::statistics::linear-model` | `forms` |
| `math::statistics::linear-residuals` | `forms` |
| `math::statistics::map` | `forms` |
| `math::statistics::max` | `forms` |
| `math::statistics::mean` | `forms` `sfx` `ret` |
| `math::statistics::mean-histogram-limits` | `forms` |
| `math::statistics::median` | `forms` `ret` |
| `math::statistics::min` | `forms` |
| `math::statistics::minmax-histogram-limits` | `forms` |
| `math::statistics::number` | `forms` |
| `math::statistics::print-2x2` | `forms` |
| `math::statistics::pstdev` | `forms` |
| `math::statistics::pvar` | `forms` |
| `math::statistics::quantiles` | `forms` `ret` |
| `math::statistics::samplescount` | `forms` |
| `math::statistics::spearman-rank` | `forms` |
| `math::statistics::spearman-rank-extended` | `forms` |
| `math::statistics::stdev` | `forms` `ret` |
| `math::statistics::t-test-mean` | `forms` |
| `math::statistics::test-2x2` | `forms` |
| `math::statistics::test-Duckworth` | `forms` |
| `math::statistics::test-Dunnett` | `forms` |
| `math::statistics::test-Kruskal-Wallis` | `forms` |
| `math::statistics::test-Rchart` | `forms` |
| `math::statistics::test-Tukey-range` | `forms` |
| `math::statistics::test-Wilcoxon` | `forms` |
| `math::statistics::test-anova-F` | `forms` |
| `math::statistics::test-normal` | `forms` |
| `math::statistics::test-xbar` | `forms` |
| `math::statistics::var` | `forms` `ret` |
| `md5::md5` | `forms` `sfx` `ret` |
| `mime::buildmessage` | `forms` `sfx` |
| `mime::copymessage` | `forms` `sfx` |
| `mime::field_decode` | `forms` |
| `mime::finalize` | `forms` |
| `mime::getContentType` | `forms` |
| `mime::getTransferEncoding` | `forms` |
| `mime::getbody` | `forms` `ret` |
| `mime::getheader` | `forms` |
| `mime::getproperty` | `forms` `ret` |
| `mime::getsize` | `forms` |
| `mime::initialize` | `forms` `sfx` `ret` `syn≠` |
| `mime::mapencoding` | `forms` |
| `mime::parseaddress` | `forms` |
| `mime::parsedatetime` | `forms` |
| `mime::reversemapencoding` | `forms` |
| `mime::setheader` | `forms` `sfx` |
| `mime::uniqueID` | `forms` `sfx` |
| `mime::word_decode` | `forms` |
| `mime::word_encode` | `forms` |
| `sha1::sha1` | `forms` `sfx` `ret` |
| `sha2::sha256` | `forms` `ret` |
| `smtp::sendmessage` | `forms` `sfx` `syn≠` |
| `snit::compile` | `forms` `sfx` |
| `snit::macro` | `forms` `sfx` |
| `snit::method` | `forms` |
| `snit::type` | `forms` `sfx` `ex` |
| `snit::typemethod` | `forms` |
| `snit::widget` | `forms` |
| `snit::widgetadaptor` | `forms` |
| `struct::list` | `forms` `sub` `sfx` |
| `struct::queue` | `forms` `sfx` |
| `struct::set` | `forms` `sub` `sfx` |
| `struct::stack` | `forms` `sfx` |
| `textutil::adjust` | `forms` `ex` `ret` `syn≠` |
| `textutil::blank` | `forms` |
| `textutil::cap` | `forms` |
| `textutil::capEachWord` | `forms` |
| `textutil::chop` | `forms` |
| `textutil::indent` | `forms` `ex` `ret` |
| `textutil::longestCommonPrefix` | `forms` |
| `textutil::longestCommonPrefixList` | `forms` |
| `textutil::splitn` | `forms` |
| `textutil::splitx` | `forms` `ex` `ret` |
| `textutil::strRepeat` | `forms` |
| `textutil::tabify` | `forms` |
| `textutil::tabify2` | `forms` |
| `textutil::tail` | `forms` |
| `textutil::trim` | `forms` `sfx` `ex` `ret` |
| `textutil::trimEmptyHeading` | `forms` |
| `textutil::trimPrefix` | `forms` |
| `textutil::trimleft` | `forms` |
| `textutil::trimright` | `forms` |
| `textutil::uncap` | `forms` |
| `textutil::undent` | `forms` `ex` `ret` |
| `textutil::untabify` | `forms` |
| `textutil::untabify2` | `forms` |
| `uri::canonicalize` | `forms` `ret` |
| `uri::geturl` | `forms` `sfx` `ret` |
| `uri::isrelative` | `forms` `ret` |
| `uri::join` | `forms` `ex` `ret` |
| `uri::register` | `forms` `sfx` |
| `uri::resolve` | `forms` `ex` `ret` |
| `uri::setQuirkOption` | `forms` `sfx` |
| `uri::split` | `forms` `sfx` `ex` `ret` |
| `uuid::uuid` | `forms` `sub` `sfx` `ex` `ret` `syn≠` |
| `yaml::dict2yaml` | `forms` `ret` |
| `yaml::huddle2yaml` | `forms` `ret` |
| `yaml::list2yaml` | `forms` `ret` |
| `yaml::setOptions` | `forms` `sfx` |
| `yaml::yaml2dict` | `forms` `sfx` `ex` `ret` |
| `yaml::yaml2huddle` | `forms` `sfx` `ret` |

</details>

<details><summary><b>irules</b> — 1015 entries · 0 ✅ · 1015 need work</summary>

| entry | status |
|---|---|
| `AAA::acct_result` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `AAA::acct_send` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `AAA::auth_result` | `forms` `sfx` `ex` `ret` `srcurl` |
| `AAA::auth_send` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `ACCESS2::access2_proc` | `forms` `sfx` `srcurl` `sum≠` |
| `ACCESS::acl` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ACCESS::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ACCESS::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ACCESS::ephemeral-auth` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `ACCESS::flowid` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ACCESS::log` | `forms` `sfx` `ex` `srcurl` |
| `ACCESS::oauth` | `forms` `opt` `sfx` `ex` `ret` `srcurl` |
| `ACCESS::perflow` | `forms` `sub` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `ACCESS::policy` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `ACCESS::respond` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` `syn≠` |
| `ACCESS::restrict_irule_events` | `forms` `sfx` `evtreq` `ex` `srcurl` `sum≠` |
| `ACCESS::saml` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `ACCESS::session` | `forms` `opt` `sub` `sfx` `ex` `srcurl` `syn≠` |
| `ACCESS::user` | `forms` `sub` `sfx` `ex` `srcurl` `syn≠` |
| `ACCESS::uuid` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `ACL::action` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ACL::eval` | `forms` `opt` `sfx` `evtreq` `ex` `srcurl` |
| `ADAPT::allow` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::context_create` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::context_current` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::context_delete_all` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ADAPT::context_name` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::context_static` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::preview_size` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::result` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::select` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::service_down_action` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ADAPT::timeout` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `AES::decrypt` | `forms` `sfx` `ex` `ret` `srcurl` |
| `AES::encrypt` | `forms` `sfx` `ex` `ret` `srcurl` |
| `AES::key` | `forms` `sfx` `ex` `ret` `srcurl` |
| `AM::age` | `forms` `sfx` `srcurl` |
| `AM::application` | `forms` `sfx` `srcurl` |
| `AM::cache` | `forms` `sfx` `srcurl` |
| `AM::disable` | `forms` `sfx` `srcurl` |
| `AM::expires` | `forms` `sfx` `srcurl` |
| `AM::media_playlist` | `forms` `sfx` `srcurl` |
| `AM::policy_node` | `forms` `sfx` `srcurl` |
| `ANTIFRAUD::alert_additional_info` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `ANTIFRAUD::alert_bait_signatures` | `forms` `sfx` `evtreq` `evtprof` `srcurl` `sum≠` |
| `ANTIFRAUD::alert_component` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_defined_value` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_details` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_device_id` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `ANTIFRAUD::alert_expected_value` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `ANTIFRAUD::alert_fingerprint` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_forbidden_added_element` | `forms` `sfx` `evtreq` `evtprof` `srcurl` `sum≠` |
| `ANTIFRAUD::alert_guid` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `ANTIFRAUD::alert_html` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_http_referrer` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_license_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_min` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_origin` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_resolved_value` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_score` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_transaction_data` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_transaction_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_type` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_username` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::alert_view_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::client_id` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ANTIFRAUD::device_id` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ANTIFRAUD::disable` | `forms` `sfx` `ex` `srcurl` |
| `ANTIFRAUD::disable_alert` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::disable_app_layer_encryption` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::disable_auto_transactions` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::disable_injection` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::disable_malware` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::disable_phishing` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::enable` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ANTIFRAUD::enable_log` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ANTIFRAUD::fingerprint` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::geo` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ANTIFRAUD::guid` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::result` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ANTIFRAUD::username` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::captcha` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::captcha_age` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::captcha_status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::client_ip` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::conviction` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::deception` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASM::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASM::fingerprint` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::is_authenticated` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::login_status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `ASM::microservice` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `ASM::policy` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ASM::raise` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASM::severity` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `ASM::signature` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASM::status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::support_id` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `ASM::threat_campaign` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASM::unblock` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASM::uncaptcha` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASM::username` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ASM::violation` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `ASM::violation_data` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ASN1::decode` | `forms` `sfx` `ex` `srcurl` |
| `ASN1::element` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `ASN1::encode` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `AUTH::abort` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `AUTH::authenticate` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `AUTH::authenticate_continue` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::cert_credential` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `AUTH::cert_issuer_credential` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `AUTH::last_event_session_id` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::password_credential` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `AUTH::response_data` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::ssl_cc_ldap_status` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::ssl_cc_ldap_username` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::start` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::status` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::subscribe` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::unsubscribe` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::username_credential` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::wantcredential_prompt` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `AUTH::wantcredential_prompt_style` | `forms` `sfx` `ex` `srcurl` |
| `AUTH::wantcredential_type` | `forms` `sfx` `ex` `srcurl` |
| `AVR::disable` | `forms` `sfx` `srcurl` |
| `AVR::disable_cspm_injection` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `AVR::enable` | `forms` `sfx` `srcurl` |
| `AVR::log` | `forms` `sfx` `srcurl` |
| `BIGPROTO::enable_fix_reset` | `forms` `sfx` `ex` `ret` `srcurl` |
| `BIGTCP::release_flow` | `forms` `sfx` `ex` `srcurl` |
| `BOTDEFENSE::action` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::bot_anomalies` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `BOTDEFENSE::bot_categories` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::bot_name` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::bot_signature` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::bot_signature_category` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::captcha_age` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::captcha_status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::client_class` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `BOTDEFENSE::client_type` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::cookie_age` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::cookie_status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::cs_allowed` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `BOTDEFENSE::cs_attribute` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::cs_possible` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::device_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::disable` | `forms` `sfx` `ex` `srcurl` |
| `BOTDEFENSE::enable` | `forms` `sfx` `ex` `srcurl` |
| `BOTDEFENSE::intent` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::micro_service` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::previous_action` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::previous_request_age` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `BOTDEFENSE::previous_support_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::reason` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BOTDEFENSE::support_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `BWC::color` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `BWC::debug` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `BWC::mark` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `BWC::measure` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `BWC::policy` | `forms` `sfx` `ex` `srcurl` |
| `BWC::pps` | `forms` `sfx` `ex` `srcurl` |
| `BWC::priority` | `forms` `sfx` `ex` `srcurl` |
| `BWC::rate` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `CACHE::accept_encoding` | `forms` `sfx` `srcurl` `sum≠` |
| `CACHE::age` | `forms` `sfx` `ex` `ret` `srcurl` |
| `CACHE::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::disabled` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `CACHE::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::expire` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::fresh` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `CACHE::header` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` `syn≠` |
| `CACHE::headers` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `CACHE::hits` | `forms` `sfx` `ex` `ret` `srcurl` |
| `CACHE::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::priority` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::statskey` | `forms` `sfx` `srcurl` |
| `CACHE::trace` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::uri` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::useragent` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CACHE::userkey` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `CATEGORY::analytics` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `CATEGORY::filetype` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `CATEGORY::lookup` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `CATEGORY::matchtype` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `CATEGORY::result` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `CATEGORY::safesearch` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `CLASSIFICATION::app` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CLASSIFICATION::category` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CLASSIFICATION::disable` | `forms` `sfx` `srcurl` |
| `CLASSIFICATION::enable` | `forms` `sfx` `srcurl` |
| `CLASSIFICATION::protocol` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CLASSIFICATION::result` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CLASSIFICATION::urlcat` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `CLASSIFICATION::username` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CLASSIFY::application` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `CLASSIFY::category` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `CLASSIFY::defer` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `CLASSIFY::disable` | `forms` `sfx` `srcurl` |
| `CLASSIFY::urlcat` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `CLASSIFY::username` | `forms` `sfx` `ex` `srcurl` |
| `COMPRESS::buffer_size` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `COMPRESS::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `COMPRESS::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `COMPRESS::gzip` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `COMPRESS::method` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `COMPRESS::nodelay` | `forms` `sfx` `srcurl` |
| `CONNECTOR::disable` | `forms` `sfx` `ex` `srcurl` |
| `CONNECTOR::enable` | `forms` `sfx` `ex` `srcurl` |
| `CONNECTOR::profile` | `forms` `sfx` `ex` `ret` `srcurl` |
| `CONNECTOR::remap` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `CRYPTO::decrypt` | `forms` `opt` `sfx` `srcurl` |
| `CRYPTO::encrypt` | `forms` `opt` `sfx` `ex` `srcurl` |
| `CRYPTO::hash` | `forms` `opt` `sfx` `ex` `srcurl` |
| `CRYPTO::keygen` | `forms` `opt` `sfx` `srcurl` |
| `CRYPTO::sign` | `forms` `opt` `sfx` `ex` `srcurl` |
| `CRYPTO::verify` | `forms` `opt` `sfx` `ex` `srcurl` |
| `DATAGRAM::dns` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DATAGRAM::ip` | `forms` `sfx` `evtreq` `evtprof` `srcurl` `syn≠` |
| `DATAGRAM::ip6` | `forms` `sfx` `evtreq` `evtprof` `srcurl` `syn≠` |
| `DATAGRAM::l2` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DATAGRAM::tcp` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DATAGRAM::udp` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DECOMPRESS::disable` | `forms` `sfx` `srcurl` |
| `DECOMPRESS::enable` | `forms` `sfx` `srcurl` |
| `DEMANGLE::disable` | `forms` `sfx` `srcurl` |
| `DEMANGLE::enable` | `forms` `sfx` `srcurl` |
| `DHCP::version` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::chaddr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::ciaddr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::drop` | `forms` `sfx` `ex` `srcurl` |
| `DHCPv4::giaddr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::hlen` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::hops` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::htype` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::len` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::opcode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::option` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::reject` | `forms` `sfx` `ex` `srcurl` |
| `DHCPv4::secs` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::siaddr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::type` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::xid` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv4::yiaddr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv6::drop` | `forms` `sfx` `ex` `srcurl` |
| `DHCPv6::hop_count` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv6::len` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv6::link_address` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv6::msg_type` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv6::option` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv6::peer_address` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DHCPv6::reject` | `forms` `sfx` `ex` `srcurl` |
| `DHCPv6::transaction_id` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DIAG::test` | `forms` `sfx` `srcurl` |
| `DIAMETER::avp` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DIAMETER::command` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DIAMETER::disconnect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::drop` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::dynamic_route_insertion` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `DIAMETER::dynamic_route_lookup` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::header` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DIAMETER::host` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::is_request` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DIAMETER::is_response` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DIAMETER::is_retransmission` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DIAMETER::length` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::message` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::persist` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DIAMETER::realm` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::respond` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::result` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::retransmission` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::retransmission_default` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `DIAMETER::retransmission_reason` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DIAMETER::retransmit` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `DIAMETER::retry` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `DIAMETER::route_status` | `forms` `sfx` `evtreq` `evtprof` `ret` `srcurl` |
| `DIAMETER::session` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DIAMETER::skip_capabilities_exchange` | `forms` `sfx` `evtreq` `ex` `srcurl` `sum≠` |
| `DIAMETER::state` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::additional` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::answer` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::authority` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::class` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::drop` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::edns0` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DNS::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::header` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DNS::is_wideip` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::last_act` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::len` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::log` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::name` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::origin` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DNS::ptype` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::query` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `DNS::question` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `DNS::rdata` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::return` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `DNS::rpz_policy` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `DNS::rr` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` `syn≠` |
| `DNS::scrape` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `DNS::tsig` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `DNS::ttl` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNS::type` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `DNSMSG::header` | `forms` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `DNSMSG::record` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DNSMSG::section` | `forms` `sfx` `ex` `ret` `srcurl` |
| `DOSL7::disable` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `DOSL7::enable` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `DOSL7::health` | `forms` `sfx` `ret` `srcurl` |
| `DOSL7::is_ip_slowdown` | `forms` `sfx` `srcurl` |
| `DOSL7::is_mitigated` | `forms` `sfx` `ret` `srcurl` |
| `DOSL7::profile` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `DOSL7::slowdown` | `forms` `sfx` `ex` `srcurl` |
| `DSLITE::remote_addr` | `forms` `sfx` `srcurl` |
| `ECA::client_machine_name` | `forms` `sfx` `srcurl` |
| `ECA::disable` | `forms` `sfx` `srcurl` |
| `ECA::domainname` | `forms` `sfx` `srcurl` |
| `ECA::enable` | `forms` `sfx` `srcurl` |
| `ECA::select` | `forms` `sfx` `srcurl` `sum≠` |
| `ECA::status` | `forms` `sfx` `srcurl` |
| `ECA::username` | `forms` `sfx` `srcurl` |
| `FIX::tag` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `FLOW::create_related` | `forms` `opt` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `FLOW::idle_duration` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `FLOW::idle_timeout` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `FLOW::peer` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `FLOW::priority` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `FLOW::refresh` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `FLOW::this` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `FLOWTABLE::count` | `forms` `sfx` `srcurl` `syn≠` |
| `FLOWTABLE::limit` | `forms` `sfx` `srcurl` `syn≠` |
| `FTP::allow_active_mode` | `forms` `sfx` `ex` `srcurl` |
| `FTP::disable` | `forms` `sfx` `ex` `srcurl` |
| `FTP::enable` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `FTP::enforce_tls_session_reuse` | `forms` `sfx` `ex` `srcurl` |
| `FTP::ftps_mode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `FTP::port` | `forms` `sfx` `ex` `srcurl` |
| `GENERICMESSAGE::message` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `GENERICMESSAGE::peer` | `forms` `sfx` `ex` `ret` `srcurl` |
| `GENERICMESSAGE::route` | `forms` `sfx` `ex` `srcurl` |
| `GTP::clone` | `forms` `sfx` `ex` `ret` `srcurl` |
| `GTP::discard` | `forms` `sfx` `ex` `srcurl` |
| `GTP::forward` | `forms` `sfx` `ex` `srcurl` |
| `GTP::header` | `forms` `opt` `sub` `sfx` `ex` `srcurl` `syn≠` |
| `GTP::ie` | `forms` `opt` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `GTP::length` | `forms` `opt` `sfx` `ex` `srcurl` |
| `GTP::message` | `forms` `opt` `sfx` `ex` `srcurl` |
| `GTP::new` | `forms` `sfx` `ex` `ret` `srcurl` |
| `GTP::parse` | `forms` `sfx` `ex` `ret` `srcurl` |
| `GTP::payload` | `forms` `opt` `sfx` `ex` `srcurl` `syn≠` |
| `GTP::respond` | `forms` `sfx` `ex` `srcurl` |
| `GTP::tunnel` | `forms` `opt` `sfx` `ex` `srcurl` `sum≠` |
| `HA::status` | `forms` `sfx` `ex` `ret` `srcurl` `exev` `sum≠` |
| `HSL::open` | `forms` `opt` `sfx` `ex` `srcurl` `syn≠` |
| `HSL::send` | `forms` `sfx` `ex` `srcurl` |
| `HTML::comment` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTML::disable` | `forms` `sfx` `ex` `ret` `srcurl` |
| `HTML::enable` | `forms` `sfx` `ex` `ret` `srcurl` |
| `HTML::encode` | `forms` `ex` `ret` `srcurl` |
| `HTML::tag` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `HTTP2::active` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `HTTP2::concurrency` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `HTTP2::disable` | `forms` `sfx` `ex` `srcurl` |
| `HTTP2::disconnect` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP2::enable` | `forms` `sfx` `ex` `srcurl` |
| `HTTP2::header` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `HTTP2::push` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` `syn≠` |
| `HTTP2::requests` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `HTTP2::stream` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP2::version` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::class` | `forms` `sub` `sfx` `evtreq` `evtprof` `srcurl` `syn≠` |
| `HTTP::close` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::collect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::cookie` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::enable` | `forms` `sfx` `ex` `srcurl` |
| `HTTP::fallback` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::has_responded` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `HTTP::header` | `forms` `sub` `sfx` `evtreq` `evtprof` `srcurl` |
| `HTTP::host` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::hsts` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `HTTP::is_keepalive` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::is_redirect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::method` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::passthrough_reason` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `HTTP::password` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::path` | `forms` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `HTTP::proxy` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `HTTP::query` | `forms` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::redirect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::reject_reason` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::release` | `forms` `sfx` `ex` `srcurl` |
| `HTTP::request` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::request_num` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `HTTP::respond` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `HTTP::response` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::retry` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `HTTP::status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::uri` | `forms` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::username` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `HTTP::version` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `HTTPLOG::disable` | `forms` `sfx` `srcurl` |
| `HTTPLOG::enable` | `forms` `sfx` `srcurl` |
| `ICAP::header` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `ICAP::method` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ICAP::status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `ICAP::uri` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `IKE::auth_success` | `forms` `sfx` `srcurl` |
| `IKE::cert` | `forms` `sfx` `srcurl` |
| `IKE::san_dirname` | `forms` `sfx` `srcurl` |
| `IKE::san_dns` | `forms` `sfx` `srcurl` |
| `IKE::san_ediparty` | `forms` `sfx` `srcurl` |
| `IKE::san_email` | `forms` `sfx` `srcurl` |
| `IKE::san_ipadd` | `forms` `sfx` `srcurl` |
| `IKE::san_othername` | `forms` `sfx` `srcurl` |
| `IKE::san_rid` | `forms` `sfx` `srcurl` |
| `IKE::san_uri` | `forms` `sfx` `srcurl` |
| `IKE::san_x400` | `forms` `sfx` `srcurl` |
| `IKE::subjectAltName` | `forms` `sfx` `srcurl` |
| `ILX::call` | `forms` `opt` `sfx` `ex` `ret` `srcurl` |
| `ILX::init` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ILX::notify` | `forms` `sfx` `ex` `ret` `srcurl` |
| `IMAP::activation_mode` | `forms` `sfx` `ex` `srcurl` |
| `IMAP::disable` | `forms` `sfx` `ex` `srcurl` |
| `IMAP::enable` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `IP::addr` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `IP::client_addr` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `IP::hops` | `forms` `sfx` `ex` `ret` `srcurl` |
| `IP::idle_timeout` | `forms` `sfx` `ex` `ret` `srcurl` |
| `IP::ingress_drop_rate` | `forms` `sfx` `evtreq` `srcurl` |
| `IP::ingress_rate_limit` | `forms` `sfx` `evtreq` `srcurl` |
| `IP::intelligence` | `forms` `sfx` `ex` `ret` `srcurl` |
| `IP::local_addr` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` `sum≠` |
| `IP::protocol` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `IP::remote_addr` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `IP::reputation` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `IP::server_addr` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `IP::stats` | `forms` `sub` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `IP::tos` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `IP::ttl` | `forms` `sfx` `evtreq` `ret` `srcurl` |
| `IP::version` | `forms` `sfx` `ex` `ret` `srcurl` |
| `IPFIX::destination` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `IPFIX::msg` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `IPFIX::template` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `ISESSION::deduplication` | `forms` `sfx` `srcurl` |
| `ISTATS::get` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ISTATS::incr` | `forms` `sfx` `ex` `srcurl` |
| `ISTATS::remove` | `forms` `sfx` `ex` `srcurl` |
| `ISTATS::set` | `forms` `sfx` `ex` `srcurl` |
| `IVS_ENTRY::result` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `JSON::array` | `forms` `sfx` `ex` `ret` `srcurl` |
| `JSON::create` | `forms` `sfx` `ex` `ret` `srcurl` |
| `JSON::get` | `forms` `sfx` `ex` `ret` `srcurl` |
| `JSON::object` | `forms` `sfx` `ex` `ret` `srcurl` |
| `JSON::parse` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `JSON::render` | `forms` `sfx` `ex` `ret` `srcurl` |
| `JSON::root` | `forms` `sfx` `ex` `ret` `srcurl` |
| `JSON::set` | `forms` `sfx` `ex` `ret` `srcurl` |
| `JSON::type` | `forms` `sfx` `ex` `ret` `srcurl` |
| `L7CHECK::protocol` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `LB::bias` | `forms` `sfx` `srcurl` |
| `LB::class` | `forms` `sfx` `ret` `srcurl` |
| `LB::command` | `forms` `sfx` `srcurl` |
| `LB::connect` | `forms` `sfx` `srcurl` |
| `LB::connlimit` | `forms` `sub` `sfx` `srcurl` |
| `LB::context_id` | `forms` `sfx` `srcurl` |
| `LB::detach` | `forms` `sfx` `ex` `srcurl` |
| `LB::down` | `forms` `sub` `sfx` `ex` `srcurl` `syn≠` |
| `LB::dst_tag` | `forms` `sfx` `srcurl` |
| `LB::enable_decisionlog` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `LB::mode` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `LB::persist` | `forms` `sub` `sfx` `srcurl` `syn≠` |
| `LB::prime` | `forms` `sfx` `srcurl` |
| `LB::queue` | `forms` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `LB::reselect` | `forms` `sfx` `evtreq` `ex` `srcurl` `syn≠` |
| `LB::select` | `forms` `sfx` `srcurl` |
| `LB::server` | `forms` `sub` `sfx` `ex` `ret` `srcurl` |
| `LB::snat` | `forms` `sfx` `ex` `ret` `srcurl` |
| `LB::src_tag` | `forms` `sfx` `srcurl` |
| `LB::status` | `forms` `sub` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `LB::up` | `forms` `sub` `sfx` `srcurl` `syn≠` |
| `LDAP::activation_mode` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `LDAP::disable` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `LDAP::enable` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `LINE::get` | `forms` `sfx` `srcurl` |
| `LINE::set` | `forms` `sfx` `srcurl` |
| `LINK::lasthop` | `forms` `sfx` `ex` `ret` `srcurl` |
| `LINK::nexthop` | `forms` `sfx` `ex` `ret` `srcurl` |
| `LINK::qos` | `forms` `sfx` `ex` `ret` `srcurl` |
| `LINK::vlan_id` | `forms` `sfx` `ex` `ret` `srcurl` |
| `LSN::address` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `LSN::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `LSN::inbound` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `LSN::inbound-entry` | `forms` `opt` `sfx` `ret` `srcurl` `sum≠` `syn≠` |
| `LSN::persistence` | `forms` `sfx` `evtreq` `evtprof` `ret` `srcurl` `sum≠` `syn≠` |
| `LSN::persistence-entry` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `LSN::pool` | `forms` `sfx` `evtreq` `evtprof` `ret` `srcurl` |
| `LSN::port` | `forms` `sfx` `evtreq` `evtprof` `ret` `srcurl` |
| `MESSAGE::field` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MESSAGE::proto` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MESSAGE::type` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::clean_session` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::client_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::collect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `MQTT::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::disconnect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `MQTT::drop` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::dup` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::enable` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `MQTT::insert` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::keep_alive` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::length` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::message` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::packet_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::password` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::payload` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `MQTT::protocol_name` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::protocol_version` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::qos` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::release` | `forms` `sfx` `ex` `srcurl` |
| `MQTT::replace` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::respond` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::retain` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::return_code` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::return_code_list` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::session_present` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::topic` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `MQTT::type` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::username` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MQTT::will` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `MR::always_match_port` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::available_for_routing` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::collect` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `MR::connect_back_port` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::connection_instance` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::connection_mode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::equivalent_transport` | `forms` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `MR::flow_id` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::ignore_peer_port` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::instance` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::max_retries` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::message` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `MR::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `MR::peer` | `forms` `sfx` `ex` `srcurl` |
| `MR::prime` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `MR::protocol` | `forms` `sfx` `ex` `ret` `srcurl` |
| `MR::release` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `MR::restore` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `MR::retry` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `MR::return` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `MR::store` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `MR::stream` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `MR::transport` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `NAME::lookup` | `forms` `sfx` `srcurl` `sum≠` |
| `NAME::response` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `NSH::chain` | `forms` `sfx` `ex` `srcurl` |
| `NSH::context` | `forms` `sfx` `ex` `srcurl` |
| `NSH::md1` | `forms` `sfx` `ex` `srcurl` |
| `NSH::mocksf` | `forms` `sfx` `ex` `srcurl` |
| `NSH::path_id` | `forms` `sfx` `ex` `ret` `srcurl` |
| `NSH::service_index` | `forms` `sfx` `ex` `ret` `srcurl` |
| `NTLM::disable` | `forms` `sfx` `ex` `srcurl` |
| `NTLM::enable` | `forms` `sfx` `srcurl` |
| `OFFBOX::request` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `ONECONNECT::detach` | `forms` `sfx` `ex` `srcurl` |
| `ONECONNECT::label` | `forms` `sfx` `ex` `srcurl` |
| `ONECONNECT::reuse` | `forms` `sfx` `ex` `srcurl` |
| `ONECONNECT::select` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `PCP::reject` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `PCP::request` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `PCP::response` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `PEM::disable` | `forms` `sfx` `ex` `srcurl` |
| `PEM::enable` | `forms` `sfx` `ex` `srcurl` |
| `PEM::flow` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `PEM::session` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `PEM::subscriber` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `PLUGIN::disable` | `sfx` `syn≠` |
| `PLUGIN::enable` | `sfx` `syn≠` |
| `POLICY::controls` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `POLICY::names` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `POLICY::rules` | `forms` `sfx` `ex` `srcurl` |
| `POLICY::targets` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `POP3::activation_mode` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `POP3::disable` | `forms` `sfx` `ex` `srcurl` |
| `POP3::enable` | `forms` `sfx` `ex` `srcurl` |
| `PROFILE::access` | `forms` `sfx` `srcurl` |
| `PROFILE::antifraud` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::auth` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::avr` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::clientssl` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::diameter` | `forms` `sfx` `ret` `srcurl` `sum≠` |
| `PROFILE::exchange` | `forms` `sfx` `srcurl` |
| `PROFILE::exists` | `forms` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `PROFILE::fastL4` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::fasthttp` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::ftp` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::http` | `forms` `sfx` `ex` `ret` `srcurl` |
| `PROFILE::httpclass` | `sfx` `syn≠` |
| `PROFILE::httpcompression` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::list` | `forms` `sfx` `ret` `srcurl` `sum≠` |
| `PROFILE::oneconnect` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::persist` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::serverssl` | `forms` `sfx` `ex` `ret` `srcurl` |
| `PROFILE::stream` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::tcp` | `forms` `sfx` `ex` `ret` `srcurl` |
| `PROFILE::tftp` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::udp` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::vdi` | `forms` `sfx` `ex` `ret` `srcurl` |
| `PROFILE::webacceleration` | `forms` `sfx` `ret` `srcurl` |
| `PROFILE::xml` | `forms` `sfx` `ret` `srcurl` |
| `PROTOCOL_INSPECTION::disable` | `forms` `sfx` `srcurl` |
| `PROTOCOL_INSPECTION::id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `PSC::aaa_reporting_interval` | `forms` `sfx` `ret` `srcurl` |
| `PSC::attr` | `forms` `sfx` `ret` `srcurl` `syn≠` |
| `PSC::calling_id` | `forms` `sfx` `ret` `srcurl` |
| `PSC::imeisv` | `forms` `sfx` `ret` `srcurl` |
| `PSC::imsi` | `forms` `sfx` `ret` `srcurl` |
| `PSC::ip_address` | `forms` `sfx` `ret` `srcurl` `syn≠` |
| `PSC::lease_time` | `forms` `sfx` `ret` `srcurl` |
| `PSC::policy` | `forms` `sfx` `ret` `srcurl` `syn≠` |
| `PSC::subscriber_id` | `forms` `sfx` `ret` `srcurl` |
| `PSC::tower_id` | `forms` `sfx` `ret` `srcurl` |
| `PSC::user_name` | `forms` `sfx` `ret` `srcurl` |
| `PSM::FTP::disable` | `forms` `sfx` `srcurl` |
| `PSM::FTP::enable` | `forms` `sfx` `srcurl` |
| `PSM::HTTP::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `PSM::HTTP::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `PSM::SMTP::disable` | `forms` `sfx` `srcurl` |
| `PSM::SMTP::enable` | `forms` `sfx` `srcurl` |
| `QOE::disable` | `forms` `sfx` `evtreq` `evtprof` `srcurl` `sum≠` |
| `QOE::enable` | `forms` `sfx` `evtreq` `evtprof` `srcurl` `sum≠` |
| `QOE::video` | `forms` `sfx` `evtreq` `evtprof` `srcurl` `sum≠` |
| `RADIUS::avp` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` `syn≠` |
| `RADIUS::code` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `RADIUS::id` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `RADIUS::rtdom` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `RADIUS::subscriber` | `forms` `sfx` `evtreq` `srcurl` |
| `RESOLV::lookup` | `forms` `opt` `sfx` `srcurl` |
| `RESOLVER::name_lookup` | `forms` `sfx` `ex` `ret` `srcurl` |
| `RESOLVER::summarize` | `forms` `sfx` `ex` `ret` `srcurl` |
| `REST::send` | `forms` `opt` `sfx` `srcurl` |
| `REWRITE::disable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `REWRITE::enable` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `REWRITE::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `REWRITE::post_process` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `ROUTE::age` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ROUTE::bandwidth` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `ROUTE::clear` | `forms` `sfx` `ex` `srcurl` |
| `ROUTE::cwnd` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ROUTE::domain` | `forms` `sfx` `ex` `srcurl` |
| `ROUTE::expiration` | `forms` `sfx` `ex` `srcurl` |
| `ROUTE::mtu` | `forms` `sfx` `ex` `srcurl` |
| `ROUTE::rtt` | `forms` `sfx` `ex` `ret` `srcurl` |
| `ROUTE::rttvar` | `forms` `sfx` `ex` `srcurl` |
| `RTSP::collect` | `forms` `sfx` `ex` `srcurl` |
| `RTSP::header` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `RTSP::method` | `forms` `sfx` `ex` `ret` `srcurl` |
| `RTSP::msg_source` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `RTSP::payload` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `RTSP::release` | `forms` `sfx` `ex` `srcurl` |
| `RTSP::respond` | `forms` `sfx` `ex` `srcurl` |
| `RTSP::status` | `forms` `sfx` `ex` `ret` `srcurl` |
| `RTSP::uri` | `forms` `sfx` `ex` `ret` `srcurl` |
| `RTSP::version` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SCTP::client_port` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::collect` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::local_port` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::mss` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::payload` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::ppi` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::release` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::remote_port` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::respond` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::rto_initial` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::rto_max` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::rto_min` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::sack_timeout` | `forms` `sfx` `ex` `srcurl` |
| `SCTP::server_port` | `forms` `sfx` `ex` `srcurl` |
| `SDP::field` | `forms` `sfx` `ex` `srcurl` |
| `SDP::media` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `SDP::session_id` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SIP::call_id` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SIP::discard` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SIP::from` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SIP::header` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `SIP::message` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SIP::method` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SIP::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `SIP::persist` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `SIP::record-route` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SIP::respond` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SIP::response` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `SIP::route` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SIP::route_status` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SIP::to` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SIP::uri` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SIP::via` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `SIPALG::hairpin` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `SIPALG::hairpin_default` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `SIPALG::nonregister_subscriber_listener` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `SMTPS::activation_mode` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `SMTPS::disable` | `forms` `sfx` `ex` `srcurl` |
| `SMTPS::enable` | `forms` `sfx` `ex` `srcurl` |
| `SOCKS::allowed` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SOCKS::destination` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `SOCKS::version` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `SSE::field` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::allow_dynamic_record_sizing` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::allow_nonssl` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::alpn` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `SSL::authenticate` | `forms` `sub` `sfx` `ex` `srcurl` `sum≠` |
| `SSL::c3d` | `forms` `sub` `sfx` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `SSL::cert` | `forms` `sub` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `SSL::cert_constraint` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::cipher` | `forms` `sub` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `SSL::clientrandom` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::collect` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::disable` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::enable` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::extensions` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `SSL::forward_proxy` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `SSL::handshake` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::is_renegotiation_secure` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::maximum_record_size` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::mode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::modssl_sessionid_headers` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::nextproto` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `SSL::payload` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::profile` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::release` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::renegotiate` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::respond` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::secure_renegotiation` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::session` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::sessionid` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::sessionsecret` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::sessionticket` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `SSL::sni` | `forms` `sub` `sfx` `ex` `ret` `srcurl` |
| `SSL::tls13_secret` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `SSL::unclean_shutdown` | `forms` `sfx` `ex` `ret` `srcurl` |
| `SSL::verify_result` | `forms` `sfx` `ex` `ret` `srcurl` |
| `STATS::get` | `forms` `sfx` `ex` `ret` `srcurl` |
| `STATS::incr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `STATS::set` | `forms` `sfx` `ex` `srcurl` |
| `STATS::setmax` | `forms` `sfx` `srcurl` |
| `STATS::setmin` | `forms` `sfx` `srcurl` |
| `STREAM::disable` | `forms` `sfx` `ex` `srcurl` |
| `STREAM::enable` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `STREAM::encoding` | `forms` `sfx` `ex` `srcurl` |
| `STREAM::expression` | `forms` `sfx` `ex` `srcurl` |
| `STREAM::match` | `forms` `sfx` `ex` `ret` `srcurl` |
| `STREAM::max_matchsize` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `STREAM::replace` | `forms` `sfx` `ex` `srcurl` |
| `TAP::action` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `syn≠` |
| `TAP::config` | `forms` `sfx` `ret` `srcurl` `sum≠` |
| `TAP::insight` | `forms` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `TAP::insight_requested` | `forms` `sfx` `ret` `srcurl` |
| `TAP::score` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `TCP::abc` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::analytics` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `TCP::autowin` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::bandwidth` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::client_port` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::close` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::collect` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::congestion` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::delayed_ack` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::dsack` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::earlyrxmit` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::ecn` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::enhanced_loss_recovery` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::idletime` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::keepalive` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::limxmit` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::local_port` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::lossfilter` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::lossfilterburst` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::lossfilterrate` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::mss` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::nagle` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::naglemode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::naglestate` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::notify` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::offset` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::option` | `forms` `sub` `sfx` `evtreq` `ex` `ret` `srcurl` `syn≠` |
| `TCP::pacing` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::payload` | `forms` `sub` `sfx` `evtreq` `ex` `srcurl` `syn≠` |
| `TCP::proxybuffer` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::proxybufferhigh` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::proxybufferlow` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::push_flag` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::rcv_scale` | `forms` `sfx` `ex` `ret` `srcurl` `exev` |
| `TCP::rcv_size` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::recvwnd` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::release` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::remote_port` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `TCP::respond` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::rexmt_thresh` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `TCP::rt_metrics_timeout` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `TCP::rto` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::rtt` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `TCP::rttvar` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::sendbuf` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::server_port` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `TCP::setmss` | `forms` `sfx` `ex` `srcurl` |
| `TCP::snd_cwnd` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::snd_scale` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::snd_ssthresh` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::snd_wnd` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TCP::unused_port` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `TDS::msg` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `TDS::session` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `TMM::cmp_count` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TMM::cmp_group` | `forms` `sfx` `ex` `ret` `srcurl` |
| `TMM::cmp_groups` | `forms` `sfx` `srcurl` |
| `TMM::cmp_primary_group` | `forms` `sfx` `srcurl` |
| `TMM::cmp_unit` | `forms` `sfx` `ex` `ret` `srcurl` |
| `UDP::client_port` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `UDP::debug_queue` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `UDP::drop` | `forms` `sfx` `evtreq` `ex` `srcurl` `sum≠` |
| `UDP::hold` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `UDP::local_port` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `UDP::max_buf_pkts` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `UDP::max_rate` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `UDP::mss` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `UDP::payload` | `forms` `sfx` `evtreq` `ex` `srcurl` `syn≠` |
| `UDP::release` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `UDP::remote_port` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `UDP::respond` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `UDP::sendbuffer` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `UDP::server_port` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `UDP::unused_port` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `URI::basename` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::compare` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::decode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::encode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::encode_component` | `forms` `ex` `ret` `srcurl` |
| `URI::escape` | `forms` `ret` `srcurl` |
| `URI::host` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::path` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::port` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::protocol` | `forms` `sfx` `ex` `ret` `srcurl` |
| `URI::query` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `VALIDATE::protocol` | `forms` `sfx` `ex` `ret` `srcurl` |
| `VDI::disable` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `VDI::enable` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `WAM::disable` | `forms` `sfx` `ex` `srcurl` |
| `WAM::enable` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WEBSSO::disable` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `WEBSSO::enable` | `forms` `sfx` `evtreq` `evtprof` `srcurl` |
| `WEBSSO::select` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WS::collect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WS::disconnect` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WS::enabled` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `WS::frame` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `WS::masking` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WS::message` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WS::payload` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `syn≠` |
| `WS::payload_ivs` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` `sum≠` |
| `WS::payload_processing` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WS::release` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `WS::request` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `WS::response` | `forms` `sub` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` |
| `X509::cert_fields` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `X509::extensions` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::hash` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::issuer` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::not_valid_after` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::not_valid_before` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::pem2der` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::serial_number` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::signature_algorithm` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::subject` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::subject_public_key` | `forms` `sfx` `ex` `srcurl` |
| `X509::subject_public_key_RSA_bits` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::subject_public_key_type` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::verify_cert_error_string` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::version` | `forms` `sfx` `ex` `ret` `srcurl` |
| `X509::whole` | `forms` `sfx` `ex` `ret` `srcurl` |
| `XLAT::listen` | `forms` `opt` `sfx` `evtreq` `ex` `ret` `srcurl` `syn≠` |
| `XLAT::listen_lifetime` | `forms` `sfx` `ex` `ret` `srcurl` |
| `XLAT::src_addr` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `XLAT::src_config` | `forms` `sfx` `ex` `ret` `srcurl` `exev` |
| `XLAT::src_endpoint_reservation` | `forms` `opt` `sfx` `ret` `srcurl` `exev` `syn≠` |
| `XLAT::src_nat_valid_range` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `XLAT::src_port` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` |
| `XML::address` | `sfx` `syn≠` |
| `XML::collect` | `sfx` `syn≠` |
| `XML::disable` | `forms` `sfx` `srcurl` |
| `XML::element` | `sfx` `syn≠` |
| `XML::enable` | `forms` `sfx` `srcurl` |
| `XML::event` | `sfx` `syn≠` |
| `XML::eventid` | `sfx` `syn≠` |
| `XML::parse` | `sfx` `syn≠` |
| `XML::payload` | `forms` `sfx` `srcurl` `syn≠` |
| `XML::release` | `sfx` `syn≠` |
| `XML::soap` | `sfx` `syn≠` |
| `XML::subscribe` | `sfx` `syn≠` |
| `accumulate` | `syn≠` |
| `active_members` | `forms` `opt` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` |
| `active_nodes` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `after` | `forms` `opt` `sfx` `ex` `ret` `srcurl` `syn≠` |
| `b64decode` | `forms` `sfx` `ex` `ret` `srcurl` |
| `b64encode` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `call` | `forms` `opt` `sfx` `ex` `ret` `srcurl` |
| `check` | `forms` `sfx` `srcurl` |
| `class` | `forms` `opt` `sub` `sfx` `ex` `srcurl` `syn≠` |
| `client_addr` | `forms` `sfx` `ret` `srcurl` |
| `client_port` | `forms` `sfx` `ret` `srcurl` |
| `clientside` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` `arity≠` |
| `clone` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `close` | `forms` `sfx` `ex` `ret` `srcurl` |
| `connect` | `forms` `opt` `sfx` `evtreq` `ex` `ret` `srcurl` `syn≠` |
| `cpu` | `forms` `sfx` `ex` `srcurl` |
| `crc32` | `forms` `sfx` `ex` `srcurl` |
| `decode_uri` | `forms` `sfx` `srcurl` |
| `discard` | `forms` `sfx` `ex` `srcurl` |
| `domain` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `drop` | `forms` `sfx` `ex` `srcurl` |
| `event` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `fasthash` | `forms` `sfx` `ex` `ret` `srcurl` |
| `findclass` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `findstr` | `forms` `sfx` `ex` `srcurl` `sum≠` |
| `forward` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `getfield` | `forms` `sfx` `ex` `srcurl` |
| `html_encode` | `forms` `ret` |
| `html_escape` | `forms` `ret` |
| `htmlencode` | `forms` `ret` |
| `htonl` | `forms` `sfx` `ex` `srcurl` |
| `htons` | `forms` `sfx` `ex` `srcurl` |
| `http_client_ip` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `http_content_len_max` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `http_cookie` | `forms` `sfx` `srcurl` |
| `http_header` | `forms` `sfx` `srcurl` |
| `http_host` | `forms` `sfx` `srcurl` |
| `http_method` | `forms` `sfx` `srcurl` |
| `http_uri` | `forms` `sfx` `srcurl` `sum≠` |
| `http_version` | `forms` `sfx` `srcurl` |
| `ifile` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `imid` | `forms` `sfx` `evtreq` `evtprof` `ret` `srcurl` |
| `ip_addr` | `syn≠` |
| `ip_protocol` | `forms` `sfx` `srcurl` |
| `ip_tos` | `forms` `sfx` `srcurl` |
| `ip_ttl` | `forms` `sfx` `ex` `srcurl` |
| `lasthop` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `link_qos` | `forms` `sfx` `srcurl` |
| `listen` | `forms` `sfx` `evtreq` `ex` `srcurl` `sum≠` |
| `llookup` | `forms` `sfx` `ex` `ret` `srcurl` |
| `local_addr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `local_port` | `syn≠` |
| `log` | `forms` `opt` `sfx` `srcurl` |
| `matchclass` | `forms` `sfx` `ex` `srcurl` |
| `md4` | `forms` `sfx` `srcurl` `sum≠` |
| `md5` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `members` | `forms` `opt` `sfx` `ex` `srcurl` |
| `nexthop` | `forms` `sfx` `evtreq` `ex` `srcurl` |
| `node` | `forms` `sfx` `evtreq` `srcurl` |
| `nodes` | `forms` `opt` `sfx` `ex` `srcurl` |
| `ntohl` | `forms` `sfx` `ex` `srcurl` |
| `ntohs` | `forms` `sfx` `ex` `srcurl` |
| `peer` | `forms` `sfx` `ex` `srcurl` `syn≠` `arity≠` |
| `pem_dtos` | `forms` `sfx` `srcurl` |
| `persist` | `forms` `sfx` `evtreq` `ex` `srcurl` `syn≠` |
| `pool` | `forms` `sfx` `evtreq` `srcurl` |
| `priority` | `forms` `sfx` `ex` `srcurl` |
| `proc` | `forms` `sfx` `ex` `ret` `srcurl` |
| `radius_authenticate` | `forms` `sfx` `srcurl` `sum≠` |
| `rateclass` | `forms` `sfx` `ex` `srcurl` |
| `recv` | `forms` `opt` `sfx` `ex` `ret` `srcurl` |
| `redirect` | `forms` `sfx` `ex` `srcurl` |
| `reject` | `forms` `sfx` `ex` `srcurl` |
| `relate_client` | `forms` `sfx` `ex` `srcurl` |
| `relate_server` | `forms` `sfx` `ex` `srcurl` |
| `remote_addr` | `forms` `sfx` `ex` `ret` `srcurl` |
| `remote_port` | `syn≠` |
| `rmd160` | `forms` `sfx` `ex` `ret` `srcurl` |
| `send` | `forms` `opt` `sfx` `ex` `ret` `srcurl` |
| `server_addr` | `forms` `sfx` `srcurl` |
| `server_port` | `forms` `sfx` `srcurl` |
| `serverside` | `forms` `sfx` `evtreq` `ex` `ret` `srcurl` `sum≠` `arity≠` |
| `session` | `forms` `sub` `sfx` `evtreq` `ex` `srcurl` `sum≠` `syn≠` |
| `sha1` | `forms` `sfx` `ex` `ret` `srcurl` |
| `sha256` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `sha384` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `sha512` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` |
| `sharedvar` | `forms` `sfx` `ex` `srcurl` |
| `snat` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `snatpool` | `forms` `sfx` `ex` `srcurl` |
| `substr` | `forms` `sfx` `ex` `srcurl` |
| `table` | `forms` `opt` `sub` `sfx` `evtreq` `ex` `srcurl` `syn≠` |
| `tcpdump` | `forms` `sfx` `srcurl` |
| `timing` | `forms` `sfx` `ex` `srcurl` |
| `traffic_group` | `forms` `sfx` `ex` `srcurl` |
| `translate` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `uniq_ordered_ip_list` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `uniq_sorted_ip_list` | `forms` `sfx` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `urlcatblindquery` | `forms` `sfx` `srcurl` |
| `urlcatquery` | `forms` `sfx` `evtreq` `evtprof` `ex` `srcurl` |
| `use` | `forms` `sfx` `ex` `srcurl` `syn≠` |
| `virtual` | `forms` `sfx` `ex` `srcurl` `sum≠` `syn≠` |
| `vlan_id` | `forms` `sfx` `srcurl` |
| `when` | `forms` `sfx` `srcurl` `syn≠` |
| `whereis` | `forms` `sfx` `srcurl` |
| `xff_list` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `xff_uniq_ordered_ip_list` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |
| `xff_uniq_sorted_ip_list` | `forms` `sfx` `evtreq` `evtprof` `ex` `ret` `srcurl` `sum≠` `syn≠` |

</details>

<details><summary><b>iapps</b> — 49 entries · 0 ✅ · 49 need work</summary>

| entry | status |
|---|---|
| `iapp::apm_config` | `forms` |
| `iapp::conf` | `forms` |
| `iapp::debug` | `forms` |
| `iapp::destination` | `forms` |
| `iapp::downgrade` | `forms` |
| `iapp::downgrade_template` | `forms` |
| `iapp::get_items` | `forms` |
| `iapp::is` | `forms` |
| `iapp::make_safe_password` | `forms` |
| `iapp::pool_members` | `forms` |
| `iapp::substa` | `forms` |
| `iapp::template` | `forms` |
| `iapp::tmos_version` | `forms` |
| `iapp::upgrade` | `forms` |
| `iapp::upgrade_template` | `forms` |
| `script::help` | `forms` |
| `script::init` | `forms` |
| `script::run` | `forms` |
| `script::tabc` | `forms` |
| `tmsh::add_help` | `forms` |
| `tmsh::add_tabc` | `forms` |
| `tmsh::begin_transaction` | `forms` |
| `tmsh::builtin_help` | `forms` |
| `tmsh::builtin_tabc` | `forms` |
| `tmsh::cancel_transaction` | `forms` |
| `tmsh::cd` | `forms` |
| `tmsh::clear_screen` | `forms` |
| `tmsh::commit_transaction` | `forms` |
| `tmsh::create` | `forms` |
| `tmsh::delete` | `forms` |
| `tmsh::display` | `forms` |
| `tmsh::display_threshold` | `forms` |
| `tmsh::get_config` | `forms` |
| `tmsh::get_field_names` | `forms` |
| `tmsh::get_field_value` | `forms` |
| `tmsh::get_name` | `forms` |
| `tmsh::get_status` | `forms` |
| `tmsh::get_type` | `forms` |
| `tmsh::include` | `forms` |
| `tmsh::list` | `forms` |
| `tmsh::log` | `forms` |
| `tmsh::log_dest` | `forms` |
| `tmsh::log_level` | `forms` |
| `tmsh::modify` | `forms` |
| `tmsh::pwd` | `forms` |
| `tmsh::reset_stats` | `forms` |
| `tmsh::show` | `forms` |
| `tmsh::stateless` | `forms` |
| `tmsh::version` | `forms` |

</details>

<details><summary><b>tk</b> — 55 entries · 0 ✅ · 55 need work</summary>

| entry | status |
|---|---|
| `bell` | `forms` `opt` `sfx` `pkg` |
| `bind` | `forms` `sfx` `pkg` `syn≠` |
| `button` | `forms` `opt` `sfx` `pkg` |
| `canvas` | `forms` `opt` `sub` `sfx` `pkg` |
| `checkbutton` | `forms` `opt` `sfx` `pkg` |
| `clipboard` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `destroy` | `forms` `sfx` `pkg` |
| `entry` | `forms` `opt` `sfx` `pkg` |
| `event` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `focus` | `forms` `opt` `sfx` `pkg` `syn≠` |
| `font` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `frame` | `forms` `opt` `sfx` `pkg` |
| `grab` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `grid` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `image` | `forms` `sub` `sfx` `pkg` `syn≠` |
| `label` | `forms` `opt` `sfx` `pkg` |
| `labelframe` | `forms` `opt` `sfx` `pkg` |
| `listbox` | `forms` `opt` `sfx` `pkg` |
| `lower` | `forms` `sfx` `pkg` |
| `menu` | `forms` `opt` `sub` `sfx` `pkg` |
| `menubutton` | `forms` `opt` `sfx` `pkg` |
| `message` | `forms` `opt` `sfx` `pkg` |
| `option` | `forms` `sub` `sfx` `pkg` `syn≠` |
| `pack` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `panedwindow` | `forms` `opt` `sub` `sfx` `pkg` |
| `place` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `radiobutton` | `forms` `opt` `sfx` `pkg` |
| `raise` | `forms` `sfx` `pkg` |
| `scale` | `forms` `opt` `sfx` `pkg` |
| `scrollbar` | `forms` `opt` `sfx` `pkg` |
| `selection` | `forms` `opt` `sub` `sfx` `pkg` `syn≠` |
| `spinbox` | `forms` `opt` `sfx` `pkg` |
| `text` | `forms` `opt` `sfx` `pkg` |
| `tk` | `forms` `sub` `sfx` `pkg` `syn≠` |
| `tk_chooseColor` | `forms` `opt` `sfx` `pkg` |
| `tk_chooseDirectory` | `forms` `opt` `sfx` `pkg` |
| `tk_getOpenFile` | `forms` `opt` `sfx` `pkg` |
| `tk_getSaveFile` | `forms` `opt` `sfx` `pkg` |
| `tk_messageBox` | `forms` `opt` `sfx` `pkg` |
| `tk_popup` | `forms` `sfx` `pkg` |
| `toplevel` | `forms` `opt` `sfx` `pkg` |
| `ttk::button` | `forms` `opt` `sfx` `pkg` |
| `ttk::combobox` | `forms` `opt` `sfx` `pkg` |
| `ttk::entry` | `forms` `opt` `sfx` `pkg` |
| `ttk::frame` | `forms` `opt` `sfx` `pkg` |
| `ttk::label` | `forms` `opt` `sfx` `pkg` |
| `ttk::notebook` | `forms` `opt` `sfx` `pkg` |
| `ttk::progressbar` | `forms` `opt` `sfx` `pkg` |
| `ttk::scale` | `forms` `opt` `sfx` `pkg` |
| `ttk::separator` | `forms` `opt` `sfx` `pkg` |
| `ttk::sizegrip` | `forms` `opt` `sfx` `pkg` |
| `ttk::style` | `forms` `sub` `sfx` `pkg` |
| `ttk::treeview` | `forms` `opt` `sfx` `pkg` |
| `winfo` | `forms` `sub` `sfx` `pkg` `syn≠` |
| `wm` | `forms` `sub` `sfx` `pkg` `syn≠` |

</details>

<details><summary><b>expect</b> — 35 entries · 0 ✅ · 35 need work</summary>

| entry | status |
|---|---|
| `close` | `forms` `opt` |
| `debug` | `forms` `opt` |
| `disconnect` | `forms` |
| `exit` | `forms` `opt` `syn≠` |
| `exp_continue` | `forms` `opt` |
| `exp_internal` | `forms` `opt` |
| `exp_pid` | `forms` `opt` |
| `exp_version` | `forms` |
| `expect` | `forms` `opt` `syn≠` |
| `expect_after` | `forms` `opt` |
| `expect_background` | `forms` `opt` |
| `expect_before` | `forms` `opt` |
| `expect_tty` | `forms` `opt` |
| `expect_user` | `forms` `opt` |
| `fork` | `forms` `sum≠` |
| `interact` | `forms` `opt` `syn≠` |
| `log_file` | `forms` `opt` `syn≠` |
| `log_user` | `forms` `opt` `syn≠` |
| `match_max` | `forms` `opt` |
| `overlay` | `forms` |
| `parity` | `forms` `opt` |
| `remove_nulls` | `forms` `opt` |
| `send` | `forms` `opt` |
| `send_error` | `forms` `opt` |
| `send_log` | `forms` `opt` |
| `send_tty` | `forms` `opt` |
| `send_user` | `forms` `opt` |
| `sleep` | `forms` |
| `spawn` | `forms` `opt` |
| `strace` | `forms` |
| `stty` | `forms` `syn≠` |
| `system` | `forms` |
| `timestamp` | `forms` `opt` `syn≠` |
| `trap` | `forms` `syn≠` |
| `wait` | `forms` `opt` |

</details>

<details><summary><b>sdc-base</b> — 61 entries · 0 ✅ · 61 need work</summary>

| entry | status |
|---|---|
| `all_clocks` | `forms` |
| `all_fanin` | `forms` |
| `all_fanout` | `forms` |
| `all_inputs` | `forms` |
| `all_outputs` | `forms` |
| `all_registers` | `forms` `syn≠` |
| `append_to_collection` | `forms` |
| `check_timing` | `forms` |
| `create_clock` | `forms` |
| `create_generated_clock` | `forms` `syn≠` |
| `current_design` | `forms` |
| `define_proc_attributes` | `forms` |
| `filter_collection` | `forms` |
| `foreach_in_collection` | `forms` |
| `get_cells` | `forms` |
| `get_clocks` | `forms` |
| `get_lib_cells` | `forms` |
| `get_lib_pins` | `forms` |
| `get_libs` | `forms` |
| `get_nets` | `forms` |
| `get_object_name` | `forms` |
| `get_pins` | `forms` |
| `get_ports` | `forms` |
| `group_path` | `forms` `syn≠` |
| `link_design` | `forms` |
| `remove_from_collection` | `forms` |
| `report_area` | `forms` |
| `report_clock` | `forms` |
| `report_clock_timing` | `forms` |
| `report_constraint` | `forms` |
| `report_power` | `forms` |
| `report_timing` | `forms` `syn≠` |
| `set_case_analysis` | `forms` |
| `set_clock_groups` | `forms` `syn≠` |
| `set_clock_latency` | `forms` |
| `set_clock_transition` | `forms` |
| `set_clock_uncertainty` | `forms` `syn≠` |
| `set_disable_timing` | `forms` |
| `set_dont_touch` | `forms` |
| `set_dont_use` | `forms` |
| `set_driving_cell` | `forms` `syn≠` |
| `set_false_path` | `forms` `syn≠` |
| `set_ideal_latency` | `forms` |
| `set_ideal_network` | `forms` |
| `set_input_delay` | `forms` `syn≠` |
| `set_input_transition` | `forms` |
| `set_load` | `forms` |
| `set_max_area` | `forms` |
| `set_max_capacitance` | `forms` |
| `set_max_delay` | `forms` |
| `set_max_fanout` | `forms` |
| `set_max_transition` | `forms` |
| `set_min_delay` | `forms` |
| `set_multicycle_path` | `forms` `syn≠` |
| `set_output_delay` | `forms` `syn≠` |
| `set_propagated_clock` | `forms` |
| `set_size_only` | `forms` |
| `set_units` | `forms` `syn≠` |
| `set_wire_load_mode` | `forms` |
| `set_wire_load_model` | `forms` |
| `sizeof_collection` | `forms` |

</details>

<details><summary><b>synopsys</b> — 68 entries · 0 ✅ · 68 need work</summary>

| entry | status |
|---|---|
| `analyze` | `forms` |
| `characterize` | `forms` |
| `check_design` | `forms` |
| `check_library` | `forms` |
| `clock_opt` | `forms` |
| `compile` | `forms` `syn≠` |
| `compile_ultra` | `forms` `syn≠` |
| `connect_net` | `forms` |
| `create_cell` | `forms` |
| `create_floorplan` | `forms` `syn≠` |
| `create_net` | `forms` |
| `create_port` | `forms` |
| `current_instance` | `forms` |
| `disconnect_net` | `forms` |
| `elaborate` | `forms` |
| `get_timing_paths` | `forms` `syn≠` |
| `group` | `forms` |
| `initialize_floorplan` | `forms` |
| `insert_clock_gating` | `forms` |
| `insert_dft` | `forms` |
| `link` | `forms` |
| `match` | `forms` |
| `optimize_netlist` | `forms` |
| `place_opt` | `forms` |
| `printvar` | `forms` |
| `read_db` | `forms` |
| `read_ddc` | `forms` |
| `read_def` | `forms` |
| `read_file` | `forms` |
| `read_lef` | `forms` |
| `read_sdc` | `forms` |
| `read_verilog` | `forms` |
| `read_vhdl` | `forms` |
| `remove_cell` | `forms` |
| `remove_design` | `forms` |
| `report_analysis_coverage` | `forms` |
| `report_bottleneck` | `forms` |
| `report_cell` | `forms` |
| `report_clock_gating` | `forms` |
| `report_congestion` | `forms` |
| `report_delay_calculation` | `forms` |
| `report_design` | `forms` |
| `report_hierarchy` | `forms` |
| `report_net` | `forms` |
| `report_qor` | `forms` |
| `report_reference` | `forms` |
| `report_status` | `forms` |
| `route_auto` | `forms` |
| `route_opt` | `forms` |
| `set_app_var` | `forms` |
| `set_clock_gating_style` | `forms` `syn≠` |
| `set_host_options` | `forms` |
| `set_implementation_design` | `forms` |
| `set_operating_conditions` | `forms` |
| `set_reference_design` | `forms` |
| `set_scan_configuration` | `forms` |
| `set_technology` | `forms` |
| `size_cell` | `forms` |
| `swap_cell` | `forms` |
| `ungroup` | `forms` |
| `uniquify` | `forms` |
| `update_timing` | `forms` |
| `verify` | `forms` |
| `write` | `forms` |
| `write_def` | `forms` |
| `write_file` | `forms` |
| `write_gds` | `forms` |
| `write_sdc` | `forms` |

</details>

<details><summary><b>cadence</b> — 56 entries · 0 ✅ · 56 need work</summary>

| entry | status |
|---|---|
| `add_endcap` | `forms` |
| `add_filler` | `forms` |
| `add_well_tap` | `forms` |
| `ccopt_design` | `forms` |
| `check_design` | `forms` |
| `check_timing_intent` | `forms` |
| `create_analysis_view` | `forms` |
| `create_constraint_mode` | `forms` |
| `create_delay_corner` | `forms` |
| `create_floorplan` | `forms` `syn≠` |
| `create_route_rule` | `forms` |
| `dbGet` | `forms` |
| `dbQuery` | `forms` |
| `dbSet` | `forms` |
| `dbShape` | `forms` |
| `edit_pin` | `forms` |
| `elaborate` | `forms` |
| `get_db` | `forms` |
| `init_design` | `forms` |
| `opt_design` | `forms` |
| `place_opt_design` | `forms` |
| `read_hdl` | `forms` |
| `read_library` | `forms` |
| `read_mmmc` | `forms` |
| `read_netlist` | `forms` |
| `read_physical` | `forms` |
| `report_analysis_coverage` | `forms` |
| `report_area` | `forms` |
| `report_constraint` | `forms` |
| `report_dp` | `forms` |
| `report_gates` | `forms` |
| `report_power` | `forms` |
| `report_qor` | `forms` |
| `report_timing` | `forms` |
| `route_design` | `forms` |
| `set_analysis_view` | `forms` |
| `set_db` | `forms` |
| `stream_out` | `forms` |
| `syn_generic` | `forms` |
| `syn_map` | `forms` |
| `syn_opt` | `forms` |
| `time_design` | `forms` |
| `update_timing` | `forms` |
| `verify_connectivity` | `forms` |
| `verify_drc` | `forms` |
| `verify_geometry` | `forms` |
| `write_def` | `forms` |
| `write_design` | `forms` |
| `write_do_lec` | `forms` |
| `write_gds` | `forms` |
| `write_hdl` | `forms` |
| `write_netlist` | `forms` |
| `write_sdc` | `forms` |
| `xelab` | `forms` |
| `xrun` | `forms` |
| `xsim` | `forms` |

</details>

<details><summary><b>xilinx</b> — 64 entries · 0 ✅ · 64 need work</summary>

| entry | status |
|---|---|
| `apply_bd_automation` | `forms` |
| `close_hw_manager` | `forms` |
| `close_project` | `forms` |
| `close_sim` | `forms` |
| `config_ip_cache` | `forms` |
| `connect_bd_intf_net` | `forms` |
| `connect_bd_net` | `forms` |
| `connect_hw_server` | `forms` |
| `create_bd_cell` | `forms` |
| `create_bd_design` | `forms` |
| `create_bd_intf_port` | `forms` |
| `create_bd_port` | `forms` |
| `create_ip` | `forms` |
| `create_project` | `forms` |
| `create_run` | `forms` |
| `current_project` | `forms` |
| `generate_target` | `forms` |
| `get_ips` | `forms` |
| `get_projects` | `forms` |
| `get_property` | `forms` |
| `get_runs` | `forms` |
| `import_ip` | `forms` |
| `ipx::add_bus_interface` | `forms` |
| `ipx::package_project` | `forms` |
| `launch_runs` | `forms` |
| `launch_simulation` | `forms` |
| `open_bd_design` | `forms` |
| `open_checkpoint` | `forms` |
| `open_hw_manager` | `forms` |
| `open_hw_target` | `forms` |
| `open_project` | `forms` |
| `open_run` | `forms` `arity≠` |
| `opt_design` | `forms` |
| `phys_opt_design` | `forms` `syn≠` |
| `place_design` | `forms` |
| `program_hw_devices` | `forms` |
| `read_checkpoint` | `forms` |
| `read_edif` | `forms` |
| `read_ip` | `forms` |
| `read_verilog` | `forms` |
| `read_vhdl` | `forms` |
| `read_xdc` | `forms` `arity≠` |
| `report_clock_networks` | `forms` |
| `report_clock_utilization` | `forms` |
| `report_design_analysis` | `forms` |
| `report_drc` | `forms` |
| `report_io` | `forms` |
| `report_methodology` | `forms` |
| `report_power` | `forms` |
| `report_route_status` | `forms` |
| `report_timing` | `forms` `syn≠` |
| `report_timing_summary` | `forms` |
| `report_utilization` | `forms` |
| `reset_run` | `forms` |
| `route_design` | `forms` |
| `save_bd_design` | `forms` |
| `save_project_as` | `forms` |
| `set_property` | `forms` |
| `synth_design` | `forms` `syn≠` |
| `upgrade_ip` | `forms` |
| `validate_bd_design` | `forms` |
| `wait_on_run` | `forms` |
| `write_bitstream` | `forms` |
| `write_checkpoint` | `forms` |

</details>

<details><summary><b>quartus</b> — 48 entries · 0 ✅ · 48 need work</summary>

| entry | status |
|---|---|
| `check_timing` | `forms` |
| `close_device` | `forms` |
| `create_timing_netlist` | `forms` |
| `delete_timing_netlist` | `forms` |
| `derive_clocks` | `forms` |
| `derive_pll_clocks` | `forms` |
| `device_lock` | `forms` |
| `device_unlock` | `forms` |
| `execute_flow` | `forms` `syn≠` |
| `execute_module` | `forms` |
| `export_assignments` | `forms` |
| `get_all_assignments` | `forms` |
| `get_global_assignment` | `forms` |
| `get_instance_assignment` | `forms` |
| `get_io_assignment` | `forms` |
| `get_name_info` | `forms` |
| `get_names` | `forms` |
| `get_number_of_columns` | `forms` |
| `get_number_of_rows` | `forms` |
| `get_part_info` | `forms` |
| `get_part_list` | `forms` |
| `get_report_panel_data` | `forms` |
| `get_report_panel_id` | `forms` |
| `get_report_panel_row_index` | `forms` |
| `load_package` | `forms` |
| `load_report` | `forms` |
| `make_connection` | `forms` |
| `open_device` | `forms` |
| `project_close` | `forms` |
| `project_exists` | `forms` |
| `project_new` | `forms` |
| `project_open` | `forms` |
| `read_sdc` | `forms` |
| `remove_all_assignments` | `forms` |
| `remove_connection` | `forms` |
| `rename_node` | `forms` |
| `report_clock_fmax_summary` | `forms` |
| `report_datasheet` | `forms` |
| `report_min_pulse_width` | `forms` |
| `report_timing` | `forms` `syn≠` |
| `report_ucp` | `forms` |
| `save_report` | `forms` |
| `set_global_assignment` | `forms` |
| `set_instance_assignment` | `forms` |
| `set_io_assignment` | `forms` |
| `set_location_assignment` | `forms` |
| `set_parameter` | `forms` |
| `update_timing_netlist` | `forms` |

</details>

<details><summary><b>mentor</b> — 49 entries · 0 ✅ · 49 need work</summary>

| entry | status |
|---|---|
| `add_list` | `forms` |
| `add_log` | `forms` |
| `add_wave` | `forms` `syn≠` |
| `bc` | `forms` |
| `bd` | `forms` |
| `be` | `forms` |
| `bl` | `forms` |
| `bp` | `forms` |
| `calibre` | `forms` |
| `calibre_drc` | `forms` |
| `calibre_lvs` | `forms` |
| `calibre_pex` | `forms` |
| `change` | `forms` |
| `coverage` | `forms` |
| `describe` | `forms` |
| `drivers` | `forms` |
| `examine` | `forms` |
| `find` | `forms` |
| `force` | `forms` |
| `formal_analyze` | `forms` |
| `formal_compile` | `forms` |
| `formal_verify` | `forms` |
| `init_signal_driver` | `forms` |
| `init_signal_spy` | `forms` |
| `onbreak` | `forms` |
| `qrun` | `forms` |
| `qverilog` | `forms` |
| `qvhdl` | `forms` |
| `qwave` | `forms` |
| `readers` | `forms` |
| `release` | `forms` |
| `restart` | `forms` |
| `resume` | `forms` |
| `run` | `forms` |
| `signal_force` | `forms` |
| `signal_release` | `forms` |
| `toggle` | `forms` |
| `transcript` | `forms` |
| `vcom` | `forms` `syn≠` |
| `vcover` | `forms` |
| `vdel` | `forms` |
| `virtual` | `forms` |
| `vlib` | `forms` |
| `vlog` | `forms` `syn≠` |
| `vmap` | `forms` |
| `vopt` | `forms` |
| `vsim` | `forms` `syn≠` |
| `wave` | `forms` |
| `when` | `forms` |

</details>

### Meta / data registries

<details><summary><b>bigip object registry</b> — 743 entries · 0 ✅ · 743 unported (Rust has no BigIP registry)</summary>

Every entry is **✗ unported**. Re-run after a Rust BigIP registry lands.

| object kind | status |
|---|---|
| `auth_apm_auth` | ✗ unported |
| `auth_cert_ldap` | ✗ unported |
| `auth_ldap` | ✗ unported |
| `auth_login_failures` | ✗ unported |
| `auth_partition` | ✗ unported |
| `auth_password` | ✗ unported |
| `auth_password_policy` | ✗ unported |
| `auth_radius` | ✗ unported |
| `auth_radius_server` | ✗ unported |
| `auth_remote_role` | ✗ unported |
| `auth_remote_user` | ✗ unported |
| `auth_source` | ✗ unported |
| `auth_tacacs` | ✗ unported |
| `auth_user` | ✗ unported |
| `cm_add_to_trust` | ✗ unported |
| `cm_cert` | ✗ unported |
| `cm_config_sync` | ✗ unported |
| `cm_device` | ✗ unported |
| `cm_device_group` | ✗ unported |
| `cm_failover_status` | ✗ unported |
| `cm_ha_group` | ✗ unported |
| `cm_key` | ✗ unported |
| `cm_remove_from_trust` | ✗ unported |
| `cm_sha1_fingerprint` | ✗ unported |
| `cm_sniff_updates` | ✗ unported |
| `cm_sync_status` | ✗ unported |
| `cm_traffic_group` | ✗ unported |
| `cm_trust_domain` | ✗ unported |
| `cm_watch_devicegroup_device` | ✗ unported |
| `cm_watch_sys_device` | ✗ unported |
| `cm_watch_trafficgroup_device` | ✗ unported |
| `gtm_add` | ✗ unported |
| `gtm_datacenter` | ✗ unported |
| `gtm_distributed_app` | ✗ unported |
| `gtm_global_settings_general` | ✗ unported |
| `gtm_global_settings_load_balancing` | ✗ unported |
| `gtm_global_settings_metrics` | ✗ unported |
| `gtm_global_settings_metrics_exclusions` | ✗ unported |
| `gtm_iquery` | ✗ unported |
| `gtm_ldns` | ✗ unported |
| `gtm_link` | ✗ unported |
| `gtm_listener` | ✗ unported |
| `gtm_listener_doh_proxy` | ✗ unported |
| `gtm_listener_doh_server` | ✗ unported |
| `gtm_monitor_bigip` | ✗ unported |
| `gtm_monitor_bigip_link` | ✗ unported |
| `gtm_monitor_external` | ✗ unported |
| `gtm_monitor_firepass` | ✗ unported |
| `gtm_monitor_ftp` | ✗ unported |
| `gtm_monitor_gateway_icmp` | ✗ unported |
| `gtm_monitor_gtp` | ✗ unported |
| `gtm_monitor_http` | ✗ unported |
| `gtm_monitor_https` | ✗ unported |
| `gtm_monitor_imap` | ✗ unported |
| `gtm_monitor_ldap` | ✗ unported |
| `gtm_monitor_mssql` | ✗ unported |
| `gtm_monitor_mysql` | ✗ unported |
| `gtm_monitor_nntp` | ✗ unported |
| `gtm_monitor_none` | ✗ unported |
| `gtm_monitor_oracle` | ✗ unported |
| `gtm_monitor_pop3` | ✗ unported |
| `gtm_monitor_postgresql` | ✗ unported |
| `gtm_monitor_radius` | ✗ unported |
| `gtm_monitor_radius_accounting` | ✗ unported |
| `gtm_monitor_real_server` | ✗ unported |
| `gtm_monitor_scripted` | ✗ unported |
| `gtm_monitor_sip` | ✗ unported |
| `gtm_monitor_smtp` | ✗ unported |
| `gtm_monitor_snmp` | ✗ unported |
| `gtm_monitor_snmp_link` | ✗ unported |
| `gtm_monitor_soap` | ✗ unported |
| `gtm_monitor_tcp` | ✗ unported |
| `gtm_monitor_tcp_half_open` | ✗ unported |
| `gtm_monitor_udp` | ✗ unported |
| `gtm_monitor_wap` | ✗ unported |
| `gtm_monitor_wmi` | ✗ unported |
| `gtm_path` | ✗ unported |
| `gtm_persist` | ✗ unported |
| `gtm_pool_a` | ✗ unported |
| `gtm_pool_aaaa` | ✗ unported |
| `gtm_pool_cname` | ✗ unported |
| `gtm_pool_https` | ✗ unported |
| `gtm_pool_mx` | ✗ unported |
| `gtm_pool_naptr` | ✗ unported |
| `gtm_pool_srv` | ✗ unported |
| `gtm_pool_svcb` | ✗ unported |
| `gtm_prober_pool` | ✗ unported |
| `gtm_region` | ✗ unported |
| `gtm_rule` | ✗ unported |
| `gtm_server` | ✗ unported |
| `gtm_topology` | ✗ unported |
| `gtm_traffic` | ✗ unported |
| `gtm_wideip_a` | ✗ unported |
| `gtm_wideip_aaaa` | ✗ unported |
| `gtm_wideip_cname` | ✗ unported |
| `gtm_wideip_https` | ✗ unported |
| `gtm_wideip_mx` | ✗ unported |
| `gtm_wideip_naptr` | ✗ unported |
| `gtm_wideip_srv` | ✗ unported |
| `gtm_wideip_svcb` | ✗ unported |
| `ltm_alg_log_profile` | ✗ unported |
| `ltm_auth_crldp_server` | ✗ unported |
| `ltm_auth_kerberos_delegation` | ✗ unported |
| `ltm_auth_ldap` | ✗ unported |
| `ltm_auth_ocsp_responder` | ✗ unported |
| `ltm_auth_profile` | ✗ unported |
| `ltm_auth_radius` | ✗ unported |
| `ltm_auth_radius_server` | ✗ unported |
| `ltm_auth_ssl_cc_ldap` | ✗ unported |
| `ltm_auth_ssl_crldp` | ✗ unported |
| `ltm_auth_ssl_ocsp` | ✗ unported |
| `ltm_auth_tacacs` | ✗ unported |
| `ltm_cipher_group` | ✗ unported |
| `ltm_cipher_rule` | ✗ unported |
| `ltm_classification_application` | ✗ unported |
| `ltm_classification_auto_update_settings` | ✗ unported |
| `ltm_classification_auto_update_status` | ✗ unported |
| `ltm_classification_category` | ✗ unported |
| `ltm_classification_ce` | ✗ unported |
| `ltm_classification_signature_definition` | ✗ unported |
| `ltm_classification_signature_update_schedule` | ✗ unported |
| `ltm_classification_signature_version` | ✗ unported |
| `ltm_classification_signatures` | ✗ unported |
| `ltm_classification_stats_application` | ✗ unported |
| `ltm_classification_stats_url_category` | ✗ unported |
| `ltm_classification_stats_urlcat_cloud` | ✗ unported |
| `ltm_classification_update_signatures` | ✗ unported |
| `ltm_classification_updates` | ✗ unported |
| `ltm_classification_url_cat_policy` | ✗ unported |
| `ltm_classification_url_category` | ✗ unported |
| `ltm_classification_urldb_feed_list` | ✗ unported |
| `ltm_classification_urldb_file` | ✗ unported |
| `ltm_clientssl_ocsp_stapling_responses` | ✗ unported |
| `ltm_clientssl_proxy_cached_certs` | ✗ unported |
| `ltm_data_group_external` | ✗ unported |
| `ltm_data_group_internal` | ✗ unported |
| `ltm_default_node_monitor` | ✗ unported |
| `ltm_dns_analytics_global_settings` | ✗ unported |
| `ltm_dns_cache_global_settings` | ✗ unported |
| `ltm_dns_cache_records_all` | ✗ unported |
| `ltm_dns_cache_records_key` | ✗ unported |
| `ltm_dns_cache_records_msg` | ✗ unported |
| `ltm_dns_cache_records_nameserver` | ✗ unported |
| `ltm_dns_cache_records_rrset` | ✗ unported |
| `ltm_dns_cache_resolver` | ✗ unported |
| `ltm_dns_cache_transparent` | ✗ unported |
| `ltm_dns_cache_validating_resolver` | ✗ unported |
| `ltm_dns_dns_express_db` | ✗ unported |
| `ltm_dns_dnssec_key` | ✗ unported |
| `ltm_dns_dnssec_zone` | ✗ unported |
| `ltm_dns_hpke_key` | ✗ unported |
| `ltm_dns_hpke_profile` | ✗ unported |
| `ltm_dns_nameserver` | ✗ unported |
| `ltm_dns_tsig_key` | ✗ unported |
| `ltm_dns_zone` | ✗ unported |
| `ltm_eviction_policy` | ✗ unported |
| `ltm_global_settings_connection` | ✗ unported |
| `ltm_global_settings_general` | ✗ unported |
| `ltm_global_settings_rule` | ✗ unported |
| `ltm_global_settings_traffic_control` | ✗ unported |
| `ltm_ifile` | ✗ unported |
| `ltm_lsn_log_profile` | ✗ unported |
| `ltm_lsn_pool` | ✗ unported |
| `ltm_message_routing_diameter_peer` | ✗ unported |
| `ltm_message_routing_diameter_profile_router` | ✗ unported |
| `ltm_message_routing_diameter_profile_session` | ✗ unported |
| `ltm_message_routing_diameter_route` | ✗ unported |
| `ltm_message_routing_diameter_transport_config` | ✗ unported |
| `ltm_message_routing_generic_peer` | ✗ unported |
| `ltm_message_routing_generic_protocol` | ✗ unported |
| `ltm_message_routing_generic_route` | ✗ unported |
| `ltm_message_routing_generic_router` | ✗ unported |
| `ltm_message_routing_generic_transport_config` | ✗ unported |
| `ltm_message_routing_mqtt_peer` | ✗ unported |
| `ltm_message_routing_mqtt_profile_router` | ✗ unported |
| `ltm_message_routing_mqtt_profile_session` | ✗ unported |
| `ltm_message_routing_mqtt_route` | ✗ unported |
| `ltm_message_routing_mqtt_transport_config` | ✗ unported |
| `ltm_message_routing_sip_peer` | ✗ unported |
| `ltm_message_routing_sip_profile_router` | ✗ unported |
| `ltm_message_routing_sip_profile_session` | ✗ unported |
| `ltm_message_routing_sip_route` | ✗ unported |
| `ltm_message_routing_sip_transport_config` | ✗ unported |
| `ltm_monitor_diameter` | ✗ unported |
| `ltm_monitor_dns` | ✗ unported |
| `ltm_monitor_external` | ✗ unported |
| `ltm_monitor_firepass` | ✗ unported |
| `ltm_monitor_ftp` | ✗ unported |
| `ltm_monitor_gateway_icmp` | ✗ unported |
| `ltm_monitor_http` | ✗ unported |
| `ltm_monitor_http2` | ✗ unported |
| `ltm_monitor_https` | ✗ unported |
| `ltm_monitor_icmp` | ✗ unported |
| `ltm_monitor_imap` | ✗ unported |
| `ltm_monitor_inband` | ✗ unported |
| `ltm_monitor_ldap` | ✗ unported |
| `ltm_monitor_module_score` | ✗ unported |
| `ltm_monitor_mqtt` | ✗ unported |
| `ltm_monitor_mssql` | ✗ unported |
| `ltm_monitor_mysql` | ✗ unported |
| `ltm_monitor_nntp` | ✗ unported |
| `ltm_monitor_none` | ✗ unported |
| `ltm_monitor_oracle` | ✗ unported |
| `ltm_monitor_pop3` | ✗ unported |
| `ltm_monitor_postgresql` | ✗ unported |
| `ltm_monitor_radius` | ✗ unported |
| `ltm_monitor_radius_accounting` | ✗ unported |
| `ltm_monitor_real_server` | ✗ unported |
| `ltm_monitor_rpc` | ✗ unported |
| `ltm_monitor_sasp` | ✗ unported |
| `ltm_monitor_scripted` | ✗ unported |
| `ltm_monitor_sip` | ✗ unported |
| `ltm_monitor_smb` | ✗ unported |
| `ltm_monitor_smtp` | ✗ unported |
| `ltm_monitor_snmp_dca` | ✗ unported |
| `ltm_monitor_snmp_dca_base` | ✗ unported |
| `ltm_monitor_soap` | ✗ unported |
| `ltm_monitor_tcp` | ✗ unported |
| `ltm_monitor_tcp_echo` | ✗ unported |
| `ltm_monitor_tcp_half_open` | ✗ unported |
| `ltm_monitor_udp` | ✗ unported |
| `ltm_monitor_virtual_location` | ✗ unported |
| `ltm_monitor_wap` | ✗ unported |
| `ltm_monitor_wmi` | ✗ unported |
| `ltm_nat` | ✗ unported |
| `ltm_nat_stats` | ✗ unported |
| `ltm_node` | ✗ unported |
| `ltm_persistence_cookie` | ✗ unported |
| `ltm_persistence_dest_addr` | ✗ unported |
| `ltm_persistence_global_settings` | ✗ unported |
| `ltm_persistence_hash` | ✗ unported |
| `ltm_persistence_host` | ✗ unported |
| `ltm_persistence_msrdp` | ✗ unported |
| `ltm_persistence_persist_records` | ✗ unported |
| `ltm_persistence_sip` | ✗ unported |
| `ltm_persistence_source_addr` | ✗ unported |
| `ltm_persistence_ssl` | ✗ unported |
| `ltm_persistence_universal` | ✗ unported |
| `ltm_policy` | ✗ unported |
| `ltm_policy_strategy` | ✗ unported |
| `ltm_pool` | ✗ unported |
| `ltm_profile_analytics` | ✗ unported |
| `ltm_profile_certificate_authority` | ✗ unported |
| `ltm_profile_classification` | ✗ unported |
| `ltm_profile_client_ldap` | ✗ unported |
| `ltm_profile_client_ssl` | ✗ unported |
| `ltm_profile_connector` | ✗ unported |
| `ltm_profile_dhcpv4` | ✗ unported |
| `ltm_profile_dhcpv6` | ✗ unported |
| `ltm_profile_diameter` | ✗ unported |
| `ltm_profile_dns` | ✗ unported |
| `ltm_profile_dns_logging` | ✗ unported |
| `ltm_profile_doh_proxy` | ✗ unported |
| `ltm_profile_doh_server` | ✗ unported |
| `ltm_profile_fasthttp` | ✗ unported |
| `ltm_profile_fastl4` | ✗ unported |
| `ltm_profile_fix` | ✗ unported |
| `ltm_profile_ftp` | ✗ unported |
| `ltm_profile_georedundancy` | ✗ unported |
| `ltm_profile_gtp` | ✗ unported |
| `ltm_profile_html` | ✗ unported |
| `ltm_profile_http` | ✗ unported |
| `ltm_profile_http2` | ✗ unported |
| `ltm_profile_http3` | ✗ unported |
| `ltm_profile_http_compression` | ✗ unported |
| `ltm_profile_httprouter` | ✗ unported |
| `ltm_profile_icap` | ✗ unported |
| `ltm_profile_iiop` | ✗ unported |
| `ltm_profile_ilx` | ✗ unported |
| `ltm_profile_imap` | ✗ unported |
| `ltm_profile_ipother` | ✗ unported |
| `ltm_profile_ipsecalg` | ✗ unported |
| `ltm_profile_json` | ✗ unported |
| `ltm_profile_mapt` | ✗ unported |
| `ltm_profile_mblb` | ✗ unported |
| `ltm_profile_mqtt` | ✗ unported |
| `ltm_profile_mr_ratelimit` | ✗ unported |
| `ltm_profile_mr_ratelimit_action` | ✗ unported |
| `ltm_profile_mssql` | ✗ unported |
| `ltm_profile_netflow` | ✗ unported |
| `ltm_profile_ntlm` | ✗ unported |
| `ltm_profile_ocsp` | ✗ unported |
| `ltm_profile_ocsp_stapling_params` | ✗ unported |
| `ltm_profile_one_connect` | ✗ unported |
| `ltm_profile_pcp` | ✗ unported |
| `ltm_profile_pop3` | ✗ unported |
| `ltm_profile_pptp` | ✗ unported |
| `ltm_profile_qoe` | ✗ unported |
| `ltm_profile_quic` | ✗ unported |
| `ltm_profile_radius` | ✗ unported |
| `ltm_profile_ramcache` | ✗ unported |
| `ltm_profile_request_adapt` | ✗ unported |
| `ltm_profile_request_log` | ✗ unported |
| `ltm_profile_response_adapt` | ✗ unported |
| `ltm_profile_rewrite` | ✗ unported |
| `ltm_profile_rtsp` | ✗ unported |
| `ltm_profile_sctp` | ✗ unported |
| `ltm_profile_server_ldap` | ✗ unported |
| `ltm_profile_server_ssl` | ✗ unported |
| `ltm_profile_sip` | ✗ unported |
| `ltm_profile_smtp` | ✗ unported |
| `ltm_profile_smtps` | ✗ unported |
| `ltm_profile_socks` | ✗ unported |
| `ltm_profile_splitsessionclient` | ✗ unported |
| `ltm_profile_splitsessionserver` | ✗ unported |
| `ltm_profile_sse` | ✗ unported |
| `ltm_profile_statistics` | ✗ unported |
| `ltm_profile_stream` | ✗ unported |
| `ltm_profile_tcp` | ✗ unported |
| `ltm_profile_tcp_analytics` | ✗ unported |
| `ltm_profile_tdr` | ✗ unported |
| `ltm_profile_tftp` | ✗ unported |
| `ltm_profile_traffic_acceleration` | ✗ unported |
| `ltm_profile_udp` | ✗ unported |
| `ltm_profile_wa_cache` | ✗ unported |
| `ltm_profile_web_acceleration` | ✗ unported |
| `ltm_profile_web_security` | ✗ unported |
| `ltm_profile_websocket` | ✗ unported |
| `ltm_profile_xml` | ✗ unported |
| `ltm_rule` | ✗ unported |
| `ltm_rule_profiler` | ✗ unported |
| `ltm_snat` | ✗ unported |
| `ltm_snat_translation` | ✗ unported |
| `ltm_snatpool` | ✗ unported |
| `ltm_tacdb_customdb` | ✗ unported |
| `ltm_tacdb_customdb_file` | ✗ unported |
| `ltm_tacdb_licenseddb` | ✗ unported |
| `ltm_tacdb_query` | ✗ unported |
| `ltm_traffic_class` | ✗ unported |
| `ltm_traffic_matching_criteria` | ✗ unported |
| `ltm_urlcat_cloud_cache` | ✗ unported |
| `ltm_urlcat_query` | ✗ unported |
| `ltm_virtual` | ✗ unported |
| `ltm_virtual_address` | ✗ unported |
| `net_address_list` | ✗ unported |
| `net_arp` | ✗ unported |
| `net_bwc_policy` | ✗ unported |
| `net_bwc_priority_group` | ✗ unported |
| `net_bwc_traffic_group` | ✗ unported |
| `net_clone_stats` | ✗ unported |
| `net_cmetrics` | ✗ unported |
| `net_cos_global_settings` | ✗ unported |
| `net_cos_map_8021p` | ✗ unported |
| `net_cos_map_dscp` | ✗ unported |
| `net_cos_traffic_priority` | ✗ unported |
| `net_dag_globals` | ✗ unported |
| `net_dns_resolver` | ✗ unported |
| `net_f5optics` | ✗ unported |
| `net_fdb_tunnel` | ✗ unported |
| `net_fdb_vlan` | ✗ unported |
| `net_ike_evt_stat` | ✗ unported |
| `net_ike_msg_stat` | ✗ unported |
| `net_interface` | ✗ unported |
| `net_interface_cos` | ✗ unported |
| `net_interface_ddm` | ✗ unported |
| `net_ipsec_ike_daemon` | ✗ unported |
| `net_ipsec_ike_peer` | ✗ unported |
| `net_ipsec_ike_sa` | ✗ unported |
| `net_ipsec_ipsec_policy` | ✗ unported |
| `net_ipsec_ipsec_sa` | ✗ unported |
| `net_ipsec_manual_security_association` | ✗ unported |
| `net_ipsec_stat` | ✗ unported |
| `net_ipsec_traffic_selector` | ✗ unported |
| `net_ipv6_subscriber_prefix_length` | ✗ unported |
| `net_lacp_globals` | ✗ unported |
| `net_lldp_globals` | ✗ unported |
| `net_lldp_neighbors` | ✗ unported |
| `net_mroute` | ✗ unported |
| `net_multicast_globals` | ✗ unported |
| `net_ndp` | ✗ unported |
| `net_packet_filter` | ✗ unported |
| `net_packet_filter_trusted` | ✗ unported |
| `net_packet_tester_security` | ✗ unported |
| `net_port_list` | ✗ unported |
| `net_port_mirror` | ✗ unported |
| `net_rate_shaping_class` | ✗ unported |
| `net_rate_shaping_color_policer` | ✗ unported |
| `net_rate_shaping_drop_policy` | ✗ unported |
| `net_rate_shaping_queue` | ✗ unported |
| `net_rate_shaping_shaping_policy` | ✗ unported |
| `net_route` | ✗ unported |
| `net_route_domain` | ✗ unported |
| `net_router_advertisement` | ✗ unported |
| `net_routing_access_list` | ✗ unported |
| `net_routing_bfd` | ✗ unported |
| `net_routing_bgp` | ✗ unported |
| `net_routing_community_list` | ✗ unported |
| `net_routing_debug` | ✗ unported |
| `net_routing_extcommunity_list` | ✗ unported |
| `net_routing_prefix_list` | ✗ unported |
| `net_routing_profile_bgp` | ✗ unported |
| `net_routing_route_map` | ✗ unported |
| `net_rst_cause` | ✗ unported |
| `net_self` | ✗ unported |
| `net_self_allow` | ✗ unported |
| `net_service_policy` | ✗ unported |
| `net_sfc_chain` | ✗ unported |
| `net_sfc_hop` | ✗ unported |
| `net_sfc_sf` | ✗ unported |
| `net_sfc_stats` | ✗ unported |
| `net_stp` | ✗ unported |
| `net_stp_globals` | ✗ unported |
| `net_timer_policy` | ✗ unported |
| `net_trunk` | ✗ unported |
| `net_tunnels_endpoint` | ✗ unported |
| `net_tunnels_etherip` | ✗ unported |
| `net_tunnels_fec` | ✗ unported |
| `net_tunnels_fec_stat` | ✗ unported |
| `net_tunnels_geneve` | ✗ unported |
| `net_tunnels_gre` | ✗ unported |
| `net_tunnels_ipip` | ✗ unported |
| `net_tunnels_ipsec` | ✗ unported |
| `net_tunnels_lw4o6` | ✗ unported |
| `net_tunnels_map` | ✗ unported |
| `net_tunnels_ppp` | ✗ unported |
| `net_tunnels_tcp_forward` | ✗ unported |
| `net_tunnels_tunnel` | ✗ unported |
| `net_tunnels_v6rd` | ✗ unported |
| `net_tunnels_vxlan` | ✗ unported |
| `net_tunnels_wccp` | ✗ unported |
| `net_vlan` | ✗ unported |
| `net_vlan_allowed` | ✗ unported |
| `net_vlan_group` | ✗ unported |
| `net_wccp` | ✗ unported |
| `security_analytics_settings` | ✗ unported |
| `security_anti_fraud_engine_update` | ✗ unported |
| `security_anti_fraud_profile` | ✗ unported |
| `security_anti_fraud_signatures_update` | ✗ unported |
| `security_blacklist_publisher_all_blacklist_publisher` | ✗ unported |
| `security_blacklist_publisher_blacklist_publisher_stats` | ✗ unported |
| `security_blacklist_publisher_by_addr` | ✗ unported |
| `security_blacklist_publisher_by_category` | ✗ unported |
| `security_blacklist_publisher_category` | ✗ unported |
| `security_blacklist_publisher_profile` | ✗ unported |
| `security_bot_defense_anomaly` | ✗ unported |
| `security_bot_defense_anomaly_category` | ✗ unported |
| `security_bot_defense_class` | ✗ unported |
| `security_bot_defense_micro_service` | ✗ unported |
| `security_bot_defense_profile` | ✗ unported |
| `security_bot_defense_signature` | ✗ unported |
| `security_bot_defense_signature_category` | ✗ unported |
| `security_bot_defense_template` | ✗ unported |
| `security_cloud_services_cmd` | ✗ unported |
| `security_cloud_services_connector` | ✗ unported |
| `security_datasync_background_tasks` | ✗ unported |
| `security_datasync_device_stats` | ✗ unported |
| `security_datasync_global_profile` | ✗ unported |
| `security_datasync_local_profile` | ✗ unported |
| `security_debug_drop_redirect_stats` | ✗ unported |
| `security_debug_matcher` | ✗ unported |
| `security_debug_register` | ✗ unported |
| `security_device_device_context` | ✗ unported |
| `security_device_id_attribute` | ✗ unported |
| `security_dos_auto_thresholds_heavy_urls` | ✗ unported |
| `security_dos_auto_thresholds_stress_based` | ✗ unported |
| `security_dos_auto_thresholds_top_device_ids` | ✗ unported |
| `security_dos_auto_thresholds_top_geolocations` | ✗ unported |
| `security_dos_auto_thresholds_top_source_ips` | ✗ unported |
| `security_dos_auto_thresholds_top_urls` | ✗ unported |
| `security_dos_auto_thresholds_tps_based` | ✗ unported |
| `security_dos_autodos_file_object` | ✗ unported |
| `security_dos_behavioral_signature` | ✗ unported |
| `security_dos_bot_signature` | ✗ unported |
| `security_dos_bot_signature_category` | ✗ unported |
| `security_dos_device_config` | ✗ unported |
| `security_dos_dns_nxdomain_stat` | ✗ unported |
| `security_dos_dos_signature` | ✗ unported |
| `security_dos_dynamic_signatures` | ✗ unported |
| `security_dos_ip_uncommon_protolist` | ✗ unported |
| `security_dos_l4bdos_file_object` | ✗ unported |
| `security_dos_network_whitelist` | ✗ unported |
| `security_dos_profile` | ✗ unported |
| `security_dos_spva_stats` | ✗ unported |
| `security_dos_stress_stats` | ✗ unported |
| `security_dos_udp_portlist` | ✗ unported |
| `security_dos_virtual` | ✗ unported |
| `security_firewall_address_list` | ✗ unported |
| `security_firewall_config_change_log` | ✗ unported |
| `security_firewall_container_stat` | ✗ unported |
| `security_firewall_context_stat` | ✗ unported |
| `security_firewall_current_state` | ✗ unported |
| `security_firewall_fqdn_entity` | ✗ unported |
| `security_firewall_fqdn_info` | ✗ unported |
| `security_firewall_global_fqdn_policy` | ✗ unported |
| `security_firewall_global_rules` | ✗ unported |
| `security_firewall_ipi_category_info` | ✗ unported |
| `security_firewall_management_ip_rules` | ✗ unported |
| `security_firewall_matching_rule` | ✗ unported |
| `security_firewall_on_demand_compilation` | ✗ unported |
| `security_firewall_on_demand_rule_deploy` | ✗ unported |
| `security_firewall_policy` | ✗ unported |
| `security_firewall_port_list` | ✗ unported |
| `security_firewall_port_misuse_policy` | ✗ unported |
| `security_firewall_rule_list` | ✗ unported |
| `security_firewall_rule_stat` | ✗ unported |
| `security_firewall_schedule` | ✗ unported |
| `security_firewall_user_domain` | ✗ unported |
| `security_firewall_user_list` | ✗ unported |
| `security_firewall_uuid_default_autogenerate` | ✗ unported |
| `security_flowspec_route_injector_flowspec_advertised_route_info` | ✗ unported |
| `security_flowspec_route_injector_profile` | ✗ unported |
| `security_http_file_type` | ✗ unported |
| `security_http_mandatory_header` | ✗ unported |
| `security_http_profile` | ✗ unported |
| `security_ip_intelligence_blacklist_category` | ✗ unported |
| `security_ip_intelligence_feed_list` | ✗ unported |
| `security_ip_intelligence_global_policy` | ✗ unported |
| `security_ip_intelligence_info` | ✗ unported |
| `security_ip_intelligence_policy` | ✗ unported |
| `security_log_antifraud_storage_field` | ✗ unported |
| `security_log_network_storage_field` | ✗ unported |
| `security_log_profile` | ✗ unported |
| `security_log_protocol_dns_storage_field` | ✗ unported |
| `security_log_protocol_sip_storage_field` | ✗ unported |
| `security_log_remote_format` | ✗ unported |
| `security_log_storage_field` | ✗ unported |
| `security_malicious_sources_device_ids` | ✗ unported |
| `security_malicious_sources_ip_addresses` | ✗ unported |
| `security_nat_destination_translation` | ✗ unported |
| `security_nat_policy` | ✗ unported |
| `security_nat_source_translation` | ✗ unported |
| `security_packet_filter_default_rules` | ✗ unported |
| `security_packet_filter_policy` | ✗ unported |
| `security_packet_filter_rule_stat` | ✗ unported |
| `security_presentation_tmui_netflow_details` | ✗ unported |
| `security_presentation_tmui_netflow_list` | ✗ unported |
| `security_presentation_tmui_signature_details` | ✗ unported |
| `security_presentation_tmui_signature_list` | ✗ unported |
| `security_protected_servers_netflow_tmc_stat` | ✗ unported |
| `security_protected_zone` | ✗ unported |
| `security_protocol_inspection_auto_update_settings` | ✗ unported |
| `security_protocol_inspection_auto_update_status` | ✗ unported |
| `security_protocol_inspection_common_config` | ✗ unported |
| `security_protocol_inspection_compliance` | ✗ unported |
| `security_protocol_inspection_compliance_enums` | ✗ unported |
| `security_protocol_inspection_learning_stats` | ✗ unported |
| `security_protocol_inspection_learning_suggestions` | ✗ unported |
| `security_protocol_inspection_profile` | ✗ unported |
| `security_protocol_inspection_profile_status` | ✗ unported |
| `security_protocol_inspection_service` | ✗ unported |
| `security_protocol_inspection_signature` | ✗ unported |
| `security_protocol_inspection_staging` | ✗ unported |
| `security_protocol_inspection_system` | ✗ unported |
| `security_protocol_inspection_updates` | ✗ unported |
| `security_protocol_inspection_virtual_servers` | ✗ unported |
| `security_scrubber_dwbl_scrubber_category_stats` | ✗ unported |
| `security_scrubber_dwbl_scrubber_stat` | ✗ unported |
| `security_scrubber_profile` | ✗ unported |
| `security_scrubber_unredirect` | ✗ unported |
| `security_ssh_ciphers` | ✗ unported |
| `security_ssh_profile` | ✗ unported |
| `security_zone` | ✗ unported |
| `sys_air_filter_reset` | ✗ unported |
| `sys_alert_lcd` | ✗ unported |
| `sys_aom` | ✗ unported |
| `sys_appiq_config` | ✗ unported |
| `sys_application_apl_script` | ✗ unported |
| `sys_application_custom_stat` | ✗ unported |
| `sys_application_service` | ✗ unported |
| `sys_application_template` | ✗ unported |
| `sys_autoscale_group` | ✗ unported |
| `sys_availability` | ✗ unported |
| `sys_clock` | ✗ unported |
| `sys_cluster` | ✗ unported |
| `sys_config` | ✗ unported |
| `sys_config_diff` | ✗ unported |
| `sys_connection` | ✗ unported |
| `sys_console` | ✗ unported |
| `sys_core` | ✗ unported |
| `sys_cpu` | ✗ unported |
| `sys_crypto_acceleration_strategy` | ✗ unported |
| `sys_crypto_allow_key_export` | ✗ unported |
| `sys_crypto_ca_bundle_manager` | ✗ unported |
| `sys_crypto_cert` | ✗ unported |
| `sys_crypto_cert_order_manager` | ✗ unported |
| `sys_crypto_cert_validation_response_ocsp` | ✗ unported |
| `sys_crypto_cert_validator_crl` | ✗ unported |
| `sys_crypto_cert_validator_ocsp` | ✗ unported |
| `sys_crypto_check_cert` | ✗ unported |
| `sys_crypto_client` | ✗ unported |
| `sys_crypto_crl` | ✗ unported |
| `sys_crypto_csr` | ✗ unported |
| `sys_crypto_encrypted_attributes` | ✗ unported |
| `sys_crypto_fips_by_handle` | ✗ unported |
| `sys_crypto_fips_external_hsm` | ✗ unported |
| `sys_crypto_fips_key` | ✗ unported |
| `sys_crypto_key` | ✗ unported |
| `sys_crypto_master_key` | ✗ unported |
| `sys_crypto_pkcs12` | ✗ unported |
| `sys_crypto_server` | ✗ unported |
| `sys_daemon_ha` | ✗ unported |
| `sys_daemon_log_settings_clusterd` | ✗ unported |
| `sys_daemon_log_settings_csyncd` | ✗ unported |
| `sys_daemon_log_settings_icr_eventd` | ✗ unported |
| `sys_daemon_log_settings_icrd` | ✗ unported |
| `sys_daemon_log_settings_lind` | ✗ unported |
| `sys_daemon_log_settings_mcpd` | ✗ unported |
| `sys_daemon_log_settings_tmm` | ✗ unported |
| `sys_datastor` | ✗ unported |
| `sys_db` | ✗ unported |
| `sys_default_config` | ✗ unported |
| `sys_diags_ihealth` | ✗ unported |
| `sys_diags_ihealth_request` | ✗ unported |
| `sys_diags_ihealth_result` | ✗ unported |
| `sys_disk_application_volume` | ✗ unported |
| `sys_disk_directory` | ✗ unported |
| `sys_disk_logical_disk` | ✗ unported |
| `sys_dns` | ✗ unported |
| `sys_dynad_instrumentation` | ✗ unported |
| `sys_dynad_key` | ✗ unported |
| `sys_dynad_rpm` | ✗ unported |
| `sys_dynad_settings` | ✗ unported |
| `sys_dynad_status` | ✗ unported |
| `sys_ecm_config` | ✗ unported |
| `sys_ecm_register` | ✗ unported |
| `sys_failover` | ✗ unported |
| `sys_feature_module` | ✗ unported |
| `sys_file_apache_ssl_cert` | ✗ unported |
| `sys_file_browser_capabilities_db` | ✗ unported |
| `sys_file_data_group` | ✗ unported |
| `sys_file_device_capabilities_db` | ✗ unported |
| `sys_file_external_monitor` | ✗ unported |
| `sys_file_ifile` | ✗ unported |
| `sys_file_lwtunneltbl` | ✗ unported |
| `sys_file_rewrite_rule` | ✗ unported |
| `sys_file_ssl_cert` | ✗ unported |
| `sys_file_ssl_crl` | ✗ unported |
| `sys_file_ssl_key` | ✗ unported |
| `sys_fix_connection` | ✗ unported |
| `sys_folder` | ✗ unported |
| `sys_fpga_firmware_config` | ✗ unported |
| `sys_fpga_info` | ✗ unported |
| `sys_fpga_turboflex_profile` | ✗ unported |
| `sys_geoip` | ✗ unported |
| `sys_global_settings` | ✗ unported |
| `sys_ha_group` | ✗ unported |
| `sys_ha_status` | ✗ unported |
| `sys_hardware` | ✗ unported |
| `sys_host_info` | ✗ unported |
| `sys_httpd` | ✗ unported |
| `sys_hypervisor_info` | ✗ unported |
| `sys_iapp_restricted_key` | ✗ unported |
| `sys_iapprestricted_key` | ✗ unported |
| `sys_icall_event` | ✗ unported |
| `sys_icall_handler_periodic` | ✗ unported |
| `sys_icall_handler_perpetual` | ✗ unported |
| `sys_icall_handler_triggered` | ✗ unported |
| `sys_icall_istats_trigger` | ✗ unported |
| `sys_icall_publisher` | ✗ unported |
| `sys_icall_script` | ✗ unported |
| `sys_icmp_stat` | ✗ unported |
| `sys_icontrol_soap` | ✗ unported |
| `sys_integrity_status_check` | ✗ unported |
| `sys_internal_proxy` | ✗ unported |
| `sys_ip_address` | ✗ unported |
| `sys_ip_stat` | ✗ unported |
| `sys_ipfix_destination` | ✗ unported |
| `sys_ipfix_element` | ✗ unported |
| `sys_ipfix_irules` | ✗ unported |
| `sys_iprep_status` | ✗ unported |
| `sys_license` | ✗ unported |
| `sys_log` | ✗ unported |
| `sys_log_config_destination_alertd` | ✗ unported |
| `sys_log_config_destination_arcsight` | ✗ unported |
| `sys_log_config_destination_ipfix` | ✗ unported |
| `sys_log_config_destination_local_database` | ✗ unported |
| `sys_log_config_destination_local_syslog` | ✗ unported |
| `sys_log_config_destination_management_port` | ✗ unported |
| `sys_log_config_destination_remote_high_speed_log` | ✗ unported |
| `sys_log_config_destination_remote_syslog` | ✗ unported |
| `sys_log_config_destination_splunk` | ✗ unported |
| `sys_log_config_filter` | ✗ unported |
| `sys_log_config_publisher` | ✗ unported |
| `sys_log_rotate` | ✗ unported |
| `sys_mac_address` | ✗ unported |
| `sys_management_dhcp` | ✗ unported |
| `sys_management_ip` | ✗ unported |
| `sys_management_ovsdb` | ✗ unported |
| `sys_management_proxy_config` | ✗ unported |
| `sys_management_route` | ✗ unported |
| `sys_mcp_state` | ✗ unported |
| `sys_memory` | ✗ unported |
| `sys_nethsm_async_queue_stat` | ✗ unported |
| `sys_nethsm_pkcs11d_stat` | ✗ unported |
| `sys_nethsm_sync_queue_stat` | ✗ unported |
| `sys_ntp` | ✗ unported |
| `sys_outbound_smtp` | ✗ unported |
| `sys_performance_all_stats` | ✗ unported |
| `sys_performance_connections` | ✗ unported |
| `sys_performance_dnsexpress` | ✗ unported |
| `sys_performance_dnssec` | ✗ unported |
| `sys_performance_gtm` | ✗ unported |
| `sys_performance_ramcache` | ✗ unported |
| `sys_performance_system` | ✗ unported |
| `sys_performance_throughput` | ✗ unported |
| `sys_pfman_consumer` | ✗ unported |
| `sys_pfman_device` | ✗ unported |
| `sys_proc_info` | ✗ unported |
| `sys_provision` | ✗ unported |
| `sys_pva_traffic` | ✗ unported |
| `sys_raid_array` | ✗ unported |
| `sys_raid_bay` | ✗ unported |
| `sys_raid_disk` | ✗ unported |
| `sys_ready` | ✗ unported |
| `sys_scriptd` | ✗ unported |
| `sys_service` | ✗ unported |
| `sys_sflow_data_source_http` | ✗ unported |
| `sys_sflow_data_source_interface` | ✗ unported |
| `sys_sflow_data_source_system` | ✗ unported |
| `sys_sflow_data_source_vlan` | ✗ unported |
| `sys_sflow_global_settings_http` | ✗ unported |
| `sys_sflow_global_settings_interface` | ✗ unported |
| `sys_sflow_global_settings_system` | ✗ unported |
| `sys_sflow_global_settings_vlan` | ✗ unported |
| `sys_sflow_receiver` | ✗ unported |
| `sys_smtp_server` | ✗ unported |
| `sys_snmp` | ✗ unported |
| `sys_software_block_device_hotfix` | ✗ unported |
| `sys_software_block_device_image` | ✗ unported |
| `sys_software_hotfix` | ✗ unported |
| `sys_software_image` | ✗ unported |
| `sys_software_signature` | ✗ unported |
| `sys_software_status` | ✗ unported |
| `sys_software_update` | ✗ unported |
| `sys_software_update_status` | ✗ unported |
| `sys_software_volume` | ✗ unported |
| `sys_sshd` | ✗ unported |
| `sys_state_mirroring` | ✗ unported |
| `sys_sync_sys_files` | ✗ unported |
| `sys_syslog` | ✗ unported |
| `sys_tmm_info` | ✗ unported |
| `sys_tmm_traffic` | ✗ unported |
| `sys_traffic` | ✗ unported |
| `sys_turboflex_features` | ✗ unported |
| `sys_turboflex_profile_all` | ✗ unported |
| `sys_turboflex_profile_config` | ✗ unported |
| `sys_turboflex_profile_feature` | ✗ unported |
| `sys_turboflex_warning` | ✗ unported |
| `sys_ucs` | ✗ unported |
| `sys_url_db_download_result` | ✗ unported |
| `sys_url_db_download_schedule` | ✗ unported |
| `sys_url_db_url_category` | ✗ unported |
| `sys_version` | ✗ unported |

</details>

<details><summary><b>bigip — Python copy ↔ main divergence</b> — 554 shared · 245 main-only · 194 core-only (spec file stems)</summary>

The rust-branch `core/bigip/registry/specs` and `origin/main` `dialects/f5/bigip/registry/specs` have **diverged in both directions** (sampled main-only kinds confirmed absent from core's `OBJECT_SPECS`). Reconcile before/with the Rust port.

**main-only (245)** — present on main, missing from rust-branch core:

`analytics_afm_sweeper_scheduled_report`, `analytics_application_security_anomalies_scheduled_report`, `analytics_application_security_network_scheduled_report`, `analytics_application_security_scheduled_report`, `analytics_asm_bypass_scheduled_report`, `analytics_asm_cpu_scheduled_report`, `analytics_asm_memory_scheduled_report`, `analytics_asm_violation_scheduled_report`, `analytics_cpu_scheduled_report`, `analytics_device_traffic_scheduled_report`, `analytics_disk_info_scheduled_report`, `analytics_dns_protocol_scheduled_report`, `analytics_dns_scheduled_report`, `analytics_dos_l3_scheduled_report`, `analytics_fw_nat_scheduled_report`, `analytics_global_settings`, `analytics_http_scheduled_report`, `analytics_ip_intelligence_scheduled_report`, `analytics_ip_layer_scheduled_report`, `analytics_lsn_pool_scheduled_report`, `analytics_memory_scheduled_report`, `analytics_network_scheduled_report`, `analytics_pem_scheduled_report`, `analytics_pool_traffic_scheduled_report`, `analytics_proc_cpu_scheduled_report`, `analytics_protocol_security_http_scheduled_report`, `analytics_protocol_security_scheduled_report`, `analytics_sip_dos_scheduled_report`, `analytics_sip_scheduled_report`, `analytics_ssl_orchestrator_scheduled_report`, `analytics_ssl_orchestrator_service_virtual_scheduled_report`, `analytics_swg_blocked_scheduled_report`, `analytics_swg_scheduled_report`, `analytics_tcp_analytics_scheduled_report`, `analytics_tcp_scheduled_report`, `analytics_traffic_classification_scheduled_report`, `analytics_udp_scheduled_report`, `analytics_uri_type`, `analytics_vcmp_scheduled_report`, `analytics_virtual_scheduled_report`, `api_protection_profile_apiprotection`, `api_protection_response`, `api_protection_server`, `apm_aaa_active_directory`, `apm_aaa_active_directory_trusted_domains`, `apm_aaa_crldp`, `apm_aaa_endpoint_management_system`, `apm_aaa_f5_mfa_configuration`, `apm_aaa_f5_service_connector`, `apm_aaa_http`, `apm_aaa_http_connector_request`, `apm_aaa_kerberos`, `apm_aaa_kerberos_keytab_file`, `apm_aaa_ldap`, `apm_aaa_oam`, `apm_aaa_oauth_provider`, `apm_aaa_oauth_request`, `apm_aaa_oauth_server`, `apm_aaa_ocsp`, `apm_aaa_okta_connector`, `apm_aaa_radius`, `apm_aaa_saml`, `apm_aaa_saml_idp_automation`, `apm_aaa_saml_idp_connector`, `apm_aaa_securid`, `apm_aaa_tacacsplus`, `apm_acl`, `apm_apm_avr_config`, `apm_client_image`, `apm_configuration_captcha`, `apm_epsec_epsec_package`, `apm_log_setting`, `apm_ntlm_machine_account`, `apm_ntlm_ntlm_auth`, `apm_oauth_db_instance`, `apm_oauth_jwk_config`, `apm_oauth_jwt_config`, `apm_oauth_jwt_provider_list`, `apm_oauth_oauth_claim`, `apm_oauth_oauth_client_app`, `apm_oauth_oauth_resource_server`, `apm_oauth_oauth_scope`, `apm_policy_agent_aaa_active_directory`, `apm_policy_agent_aaa_client_cert`, `apm_policy_agent_aaa_crldp`, `apm_policy_agent_aaa_http`, `apm_policy_agent_aaa_ldap`, `apm_policy_agent_aaa_oauth`, `apm_policy_agent_aaa_radius`, `apm_policy_agent_aaa_saml`, `apm_policy_agent_aaa_securid`, `apm_policy_agent_acct_radius`, `apm_policy_agent_acct_tacacsplus`, `apm_policy_agent_api_authentication`, `apm_policy_agent_api_server_selection`, `apm_policy_agent_decision_box`, `apm_policy_agent_dynamic_acl`, `apm_policy_agent_ending_allow`, `apm_policy_agent_ending_deny`, `apm_policy_agent_ending_redirect`, `apm_policy_agent_endpoint_check_machine_cert`, `apm_policy_agent_endpoint_check_software`, `apm_policy_agent_endpoint_linux_check_file`, `apm_policy_agent_endpoint_linux_check_process`, `apm_policy_agent_endpoint_mac_check_file`, `apm_policy_agent_endpoint_mac_check_process`, `apm_policy_agent_endpoint_machine_info`, `apm_policy_agent_endpoint_windows_browser_cache_cleaner`, `apm_policy_agent_endpoint_windows_check_file`, `apm_policy_agent_endpoint_windows_check_process`, `apm_policy_agent_endpoint_windows_check_registry`, `apm_policy_agent_endpoint_windows_group_policy`, `apm_policy_agent_endpoint_windows_info_os`, `apm_policy_agent_endpoint_windows_protected_workspace`, `apm_policy_agent_external_logon_page`, `apm_policy_agent_http_header_modify`, `apm_policy_agent_ip_geolocation_lookup`, `apm_policy_agent_ip_reputation_lookup`, `apm_policy_agent_irule_event`, `apm_policy_agent_kerberos`, `apm_policy_agent_l7_protocol_lookup`, `apm_policy_agent_logging`, `apm_policy_agent_logon_page`, `apm_policy_agent_message_box`, `apm_policy_agent_oam`, `apm_policy_agent_oauth_authz`, `apm_policy_agent_request_classification`, `apm_policy_agent_resource_assign`, `apm_policy_agent_response_selection`, `apm_policy_agent_route_domain_selection`, `apm_policy_agent_server_cert_response_control`, `apm_policy_agent_server_cert_status`, `apm_policy_agent_session_check`, `apm_policy_agent_ssl_check`, `apm_policy_agent_tacacsplus`, `apm_policy_agent_variable_assign`, `apm_profile_access`, `apm_profile_connectivity`, `apm_profile_exchange`, `apm_profile_oauth`, `apm_profile_vdi`, `apm_report_custom_report_field`, `apm_resource_address_space`, `apm_resource_app_tunnel`, `apm_resource_client_rate_class`, `apm_resource_client_traffic_classifier`, `apm_resource_ipv6_leasepool`, `apm_resource_leasepool`, `apm_resource_network_access`, `apm_resource_portal_access`, `apm_resource_remote_desktop_citrix`, `apm_resource_remote_desktop_citrix_client_bundle`, `apm_resource_remote_desktop_citrix_client_package_file`, `apm_resource_remote_desktop_quest`, `apm_resource_remote_desktop_rdp`, `apm_resource_remote_desktop_vmware_view`, `apm_resource_sandbox`, `apm_resource_webtop`, `apm_resource_webtop_link`, `apm_saml_artifact_resolution_service`, `apm_saml_attribute_consuming_service`, `apm_saml_auth_context_class_list`, `apm_session`, `apm_sso_basic`, `apm_sso_form_based`, `apm_sso_form_basedv2`, `apm_sso_kerberos`, `apm_sso_ntlmv1`, `apm_sso_ntlmv2`, `apm_sso_oauth_bearer`, `apm_sso_saml`, `apm_sso_saml_resource`, `apm_sso_saml_sp_automation`, `apm_sso_saml_sp_connector`, `apm_swg_scheme`, `apm_url_filter`, `asm_httpclass_asm`, `asm_policy`, `cli_admin_partitions`, `cli_alias_private`, `cli_alias_shared`, `cli_global_settings`, `cli_preference`, `cli_script`, `cli_transaction`, `cli_version`, `ilx_global_settings`, `ilx_plugin`, `ilx_workspace`, `mgmt_shared_settings_api_status_availability`, `mgmt_shared_settings_api_status_log_resource`, `mgmt_shared_settings_api_status_log_resource_property`, `pem_forwarding_endpoint`, `pem_global_settings_analytics`, `pem_global_settings_gx`, `pem_global_settings_hsl_flow`, `pem_global_settings_hsl_report`, `pem_global_settings_insert_content`, `pem_global_settings_policy`, `pem_global_settings_quota_mgmt`, `pem_global_settings_session_mgmt_attributes`, `pem_global_settings_subscriber_activity_log`, `pem_interception_endpoint`, `pem_irule`, `pem_listener`, `pem_policy`, `pem_profile_diameter_endpoint`, `pem_profile_radius_aaa`, `pem_profile_spm`, `pem_profile_subscriber_mgmt`, `pem_protocol_diameter_avp`, `pem_protocol_profile_gx`, `pem_protocol_profile_radius`, `pem_protocol_radius_avp`, `pem_quota_mgmt_rating_group`, `pem_reporting_format_script`, `pem_service_chain_endpoint`, `pem_subscriber`, `pem_subscriber_attribute`, `saas_ap_ai_profile`, `saas_ati_profile`, `saas_bd_profile`, `saas_csd_profile`, `util_ipsecalgdb`, `vcmp_guest`, `vcmp_traffic_profile`, `vcmp_virtual_disk`, `vcmp_virtual_disk_template`, `wam_ad_policy`, `wam_application`, `wam_domain_list`, `wam_object_type`, `wam_policy`, `wam_resource_concat_set`, `wam_resource_domain_list`, `wam_resource_url`, `wom_advertised_route`, `wom_deduplication`, `wom_endpoint_discovery`, `wom_local_endpoint`, `wom_profile_cifs`, `wom_profile_isession`, `wom_profile_mapi`, `wom_remote_endpoint`, `wom_server_discovery`

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

