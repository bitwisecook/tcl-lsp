# Rust port of the `tcl` and `f5` CLIs

Status and roadmap for porting the two Python console scripts —
`tcl` (`tooling/tcl/main.py`) and `f5-query` (`tooling/f5/main.py`) — to native
Rust binaries. Part of the broader Rust rewrite (`docs/rust-rewrite.md`).

End goal: `tcl` and `f5-query` ship as pure-Rust binaries with the Python CLI
glue deleted; behaviour stays byte-for-byte compatible (scripted/piped output),
verified by golden differential tests.

## Crate layout (under `rust/`)

| Crate | Role |
|---|---|
| `tcl-cli` | bin `tcl` — clap command tree + verb dispatch (thin shell) |
| `f5-cli` | bin `f5-query` — clap command tree + verb dispatch (thin shell) |
| `tcl-cli-support` | shared plumbing: input resolution, output writers, per-dialect registry cache, syntax highlighter, and the `chrome` module |
| `tcl-bigip-io` | f5 input layer: UCS archives (gzip-tar + OpenPGP-symmetric decrypt, pure Rust, no gpg), the `read_path`/`load_paths` resolver, and passphrase resolution. Pure crypto core (bytes in → SCF/bytes out, all in-memory); file/stdin I/O isolated to its `paths` module |
| `tcl-bigip-query` | f5 query DSL (`dialects/f5/query`): the jq-flavoured language powering `f5 query`. **Front-end + value model + output + evaluator + jq-core builtins landed** (golden-tested over external-JSON roots); projection over the typed BIG-IP model, the remaining ~180 builtins, edit-plan, probes, side-inputs, renderers, and the verb wiring still to come. Pure, I/O-free (typed in → typed out) |

Existing engine crates reused: `tcl-lexer`, `tcl-syntax`, `tcl-registry`,
`tcl-compiler` (lowering/CFG/codegen/optimiser/analyser/segmenter),
`tcl-lsp-core` (formatting, minify), `tcl-bigip` (BIG-IP model + config parser).

### Architecture principle (confirmed in review)

- **Per-subsystem reusable library crates** with **pure, UI-agnostic,
  PyO3-friendly APIs** (typed in → typed out, no file I/O / stdout in the core).
  The bins are thin clap + I/O shells.
- Where a pure core already exists (`tcl_lsp_core::{minify,formatting}`,
  `tcl_compiler::optimiser`, `tcl_registry`), call it directly rather than
  duplicating.
- **PyO3: structure now, bind later.** Design the typed APIs to be wrappable;
  defer building wheels.

### Chrome (terminal styling) — `tcl_cli_support::chrome`

`anstream` + `anstyle` + `tabled`. Auto-detects TTY, honours
`NO_COLOR` / `CLICOLOR_FORCE`, strips ANSI when piped.

> **Rule:** chrome drives stderr / error messages / new decorative surfaces
> ONLY — never the byte-parity verb *stdout*. Piped output stays plain so
> golden tests and scripted consumers remain byte-stable.

Helpers: `eprint_error`, `eprint_status`, the style palette
(`error/warn/success/heading/dim`), and `render_table` (one rounded-border house
style for tabular verbs).

## The core insight: wiring vs engines

The CLI port is mostly thin wiring. **Byte-parity is gated by the completeness
of the underlying Rust engine**, not by the CLI code:

- Fully-ported engines (formatter, minifier, registry, lexer, segmenter) →
  the verb reaches **byte-for-byte parity**.
- In-progress engines (analyser, optimiser, lowering/VM-compiler, BIG-IP
  ref-graph) → the verb is **correctly wired** but its output inherits the
  engine's current gaps. It converges on Python automatically as the engine
  does. These are tracked as separate workstreams.

## `tcl` verb status

| Verb(s) | Status | Notes |
|---|---|---|
| `format` (`fmt`) | ✅ byte-parity | golden-tested |
| `minify` (`min`), incl. `--compact` | ✅ byte-parity | golden-tested |
| `unminify-error` (`umerr`) | ✅ parity | |
| `command-info` (`cmd-info`) | ✅ byte-parity (tcl) | golden-tested. Gap: iRules `validEvents` (needs event/profile-registry walk) |
| `highlight` (`hl`), ANSI + HTML | ✅ byte-parity | golden-tested; also powers `--colour` for format/minify/opt. Minor gap: `{*}` expansion marker |
| `completion` | ✅ idiomatic | clap_complete (not byte-identical to argcomplete, by design) |
| `diag` / `lint` / `validate` | ✅ wired | output tracks the **analyser** (missing codes e.g. E002/W220; W211 span anchoring) |
| `opt` (`optimise`) | ✅ wired | profile semantics correct (FULL=single-pass); output tracks the **optimiser** (O100/O109/O117 gaps) |
| `dis` (`asm`) | ⛔ deferred | CLI uses the VM compiler (`compile_script`: literal processing + foreach desugaring) in the excluded `runtime/rust` crate — not the raw codegen pipeline |
| `compwasm` (`wasm`) | ⛔ stub | wasm codegen pipeline + binary output |
| `symbols`/`callgraph`/`symbolgraph`/`dataflow`/`diagram` | ⛔ stub | need the serialise JSON shapes; `Scope.procs` is a `HashMap` (unordered) so output ordering won't match without sorting; analyser-gapped |
| `diff` | ⛔ stub | multi-layer AST/IR/CFG diff + serialise |
| `explore` | ⛔ stub | explorer report port |
| `find-legacy` | ⛔ stub | analyser-based detection |
| `registry-dump` | ⛔ stub | port `tooling/registry_snapshot.py` (deep dataclass-reflection → sorted-keys JSON) |
| `help` (`docs`) | ⛔ stub | KCS SQLite DB (`rusqlite`, embed `kcs.sqlite`) |
| `minimize` (`repro`) | ⛔ stub | `server/features/minimize.py` |
| `pkg` / `venv` / `docker` | ⛔ stub | the `tclpkg` subsystem (manifest/resolver/lockfile/CAS/registry/venv/docker) |

## `f5` verb status

The BIG-IP **object model + config parser** are ported (`tcl-bigip`), but most
**analysis/emit engines are still Python-only** (`dialects/f5/bigip/*`,
`dialects/f5/query/*`). The verbs needing only file I/O + the existing parser
helpers are done; the rest await engine ports.

| Verb | Status | Notes |
|---|---|---|
| `completion` | ✅ idiomatic | clap_complete |
| `merge` | ✅ byte-parity (scf) | golden-tested; tmsh deferred |
| `split` | ✅ byte-parity (scf) | uses `extract_blocks`/`parse_generic_header`; round-trip golden-tested; tmsh deferred |
| `diff` (`changes`) | ✅ parity (add/remove/scalar) | ports `compute_diff` over the model; fields read from `canon_fields()`; accepts tmsh input via `to_scf`. **Gap:** object-list field *display* (pool `members`, data-group `records`) shows canonical JSON vs Python's dataclass `repr` — change *detection* is still correct. Golden-tested (add/remove text+JSON, scalar modify, tmsh input) |
| `explain` (`describe`) | ✅ byte-parity | ports `compute_explain` + the `resolve_name` resolution layer (model-based — does **not** need the ref-graph); walks `canon_fields()` (`BigipList` navigation) for profiles/iRules/persistence/SNAT/pool. Verified across virtual/pool/auto/short-name/not-found, text + JSON; golden-tested |
| `extract` (`ucs2scf`) | ✅ byte-parity (scf) | golden-tested. Ports `extract_ucs_file` onto `tcl-bigip-io`: reads a `.ucs` (plain gzip-tar **or** OpenPGP-encrypted), extracts to SCF in memory, writes verbatim. Encrypted archives decrypt purely in Rust. tmsh/tmsh-delta deferred (needs the tmsh emitter); interactive TTY passphrase prompting not yet wired in the binary (env-var / explicit only) |
| `graph` (`deps`) | ✅ byte-parity | full ref-graph (nodes + pilot/legacy/iRule edges) → `export_graph` (DOT/JSON/Mermaid). Verified end-to-end vs the Python CLI across all formats × `--seed`/`--reverse`/`--max-depth`. (Byte-parity on configs free of the documented registry-data drift.) |
| `stats` (`summary`) | ✅ byte-parity | object/partition counts, iRule LOC + events, top-referenced, orphans over the graph. Text + JSON; golden + end-to-end verified |
| `cleanup` (`clean`) | ✅ byte-parity | BFS-reachability orphan detection → reverse-topological `tmsh delete` script. `--keep`/`--no-keep-common`, text + JSON; golden + end-to-end verified |
| `grep`, `validate`, `rename` | ⛔ stub | `grep` next (graph queries); `validate` is **independent of the query DSL** — it's a small lint registry (`dialects/f5/bigip/lint`, `run_lint` + 8 rules) over the already-ported model + ref-graph + `tcl-irules` object-refs + `tcl-registry` event lookup, so it can land on its own; `rename` an edit planner |
| `tmsh`, `convert`, `redact`/`unredact` | ⛔ stub | need the tmsh / AS3 emit + redaction engines |
| `fetch`/`push`/`pull`, `explain-flow`, `enrich-*`, `pcap-remap`, `irule` group | ⛔ stub | need remote / PCAP / query-DSL ports |
| `query` | 🚧 in progress | `tcl-bigip-query` now has the **front-end** (lexer/AST/parser), the **value model** (`Value` + `truthy`/`py_eq`/jq `sort_cmp` + `json.dumps`-faithful serialiser), the **output** render modes, the **evaluator** (full jq core: paths/pipes/streams/let/if/object-list literals/operators/broadcast dispatch), and the first **~60 builtins** (value/stream/string/math-core + all 29 special forms). All golden differential-tested end-to-end over external-JSON roots (`eval.json`, 109 cases). Remaining: BIG-IP **projection** (Container/ObjectRef/PathRef over the typed model), the other ~180 builtins (net/ip, regex-string, full math, time, graph, http, x509, side-inputs), **edit-plan/rename** (mutating queries), network **probes**, **renderers**, then the `f5 query` verb wiring |

**Shared parity helpers** (`tcl_cli_support`):
- `ensure_ascii` — escape non-ASCII as `\uXXXX`, matching Python's
  `json.dumps(ensure_ascii=True)`; applied to every JSON-emitting verb.
- `f5-cli` `commands::scf::to_scf` — normalise `tmsh create/modify` script input
  to SCF (port of `_to_scf` / `tmsh_parse.py`) before parsing.

**f5 input layer** (`tcl-bigip-io`, **done**): the UCS foundation. A plain UCS
is a gzip-tar of `/config`; an encrypted UCS is an OpenPGP *symmetric* message
(F5 KB K5437) whose plaintext is that gzip-tar. The crate decrypts **purely in
Rust** — a faithful 1:1 port of the Python `_openpgp`/`_aes` fallback (S2K +
AES-CFB + quick-check + SHA-1 MDC), chosen over the rPGP `pgp` crate because
"most faithfully reproduces the Python decryptor" points at a line-for-line
port on a tiny audited dep tree (`aes` + RustCrypto hashes + `flate2` + `tar`)
rather than a full-OpenPGP implementation. Everything is in-memory (decrypt →
gunzip → untar on cursors) so a UCS's SSL keys never touch disk. KAT-tested
(FIPS-197) + differential parity against gpg-produced fixtures. The
`read_path`/`load_paths` resolver makes `.ucs` (plain or encrypted) a
first-class input. It is wired into the model-reading verbs that are already
ported — `diff`, `explain`, `merge` accept a `.ucs` (plain or encrypted) exactly
like a `.conf`/`.scf`, golden-verified — and `stats`/`cleanup`/`grep`/`validate`/
`graph`/`rename` inherit it automatically once their ref-graph engine lands.

**Keystone:** the BIG-IP **object reference graph** — `build_bigip_object_graph`
in `dialects/f5/bigip/link_extract.py` (~569 LOC, range-based node/edge
extraction). `stats`, `cleanup`, `grep`, `validate`, `graph`, and `rename` all
build on it — port + golden-test it in isolation first (verify via
`f5 graph --format json`). (Separate from `irules_object_refs.py` (~944 LOC),
the iRule-command → object reference resolver used by the `irule`/query paths.)

Keystone progress / remaining pieces:
- ✅ **object-registry query layer** (`object_registry.py`) → `tcl-registry::bigip`
  (`kind_for_header`, `candidate_kinds_for_key`/`_for_section_item`,
  `matches_section`, `default_registry`). Differentially golden-tested (3398
  probes). `kind_for_header` matches Python exactly.
- ✅ **node extraction** (`_build_objects_for_source`) → `tcl-bigip::graph`
  (`build_objects_for_source` / `ObjectNode` / `GraphContext`). All 28 nodes of
  a representative `bigip.conf` match Python exactly (node_id, kind, offsets,
  ranges).
- ✅ **name resolution** (`resolve_kind_in_configs` + `BigipConfig.resolve_name`
  / `resolve_generic_object`) → `tcl-bigip::graph`. Object ranges read from
  `canon_fields()["range"]`; `spec.module` stands in for the per-object module
  (verified safe for table-backed kinds). 84 probes match Python.
- ✅ **legacy forward edges** (`_build_forward_edges` token-scan path +
  `build_bigip_object_graph`) → `tcl-bigip::graph` (`ObjectEdge`/`ObjectGraph`).
  Reproduces every Python legacy edge in order; 2 frozen drift edges pinned
  (`graph_edges_legacy.drift.txt`).
- ✅ **registry-first dispatch** (`references_via_spec` / pilot value-spec engine)
  — **complete**. Wired into the edge walk (runs before the legacy path, shared
  dedup) + `candidate_registry_kinds_for_display`. The graph only consumes each
  `Reference`'s `(target_kind, target_path)`, so each pilot spec is a **slim
  extractor** over the raw value (no full `ValueSpec`/`BigipList` materialisation).
  - ✅ **all reference-producing specs ported + golden-tested**:
    `ListSpec(ObjectRefSpec)` (`rules`/`policies`/`vlans`/firewall lists),
    `Profile`/`Persistence` attachments, `MonitorExpressionSpec`, `SnatModeSpec`,
    `CertKeyChainSpec`, `FirewallRuleSpec`. Comprehensive `graph_pilot.conf`
    fixture (monitor min-of, SNAT pool, cert-key-chain, firewall source/dest
    lists) + the realistic `bigip.conf`.
  - ✅ reference-free migrated specs (`DestinationSpec`, `DataGroupRecordSpec`,
    `GtmRegionMemberSpec`, `LtmPolicyRuleSpec`) correctly fall through to legacy.
  - Drift edges (Rust legacy-section refs not cleared) pinned in
    `graph_pilot.drift.txt` / `graph_edges.drift.txt`.
- ✅ **iRule edges** (`irules_refs.py` + `irules_object_refs.py`) → new shared
  **`tcl-irules`** crate (deps: tcl-compiler/tcl-lexer/tcl-registry; consumed by
  both `tcl-bigip` graph and `tcl-lsp-core` semantic tokens). Ports
  `resolve_object_ref_args` (the hand-written `_BASE_SPECS` + pool templates +
  `class`/`persist` resolvers — the 1 MB generated graph backs only completion/
  coverage, not edge resolution) and the `extract_irules_object_references`
  walker with `set`-binding copy-propagation. Wired into `_build_forward_edges`
  (`via_property = "irule:<command>"`); `tcl-lsp-core`'s span-only port migrated
  onto it. Golden-tested: 117 resolve probes, 15 walker cases, and the full
  `bigip.conf` graph (61 edges incl. 3 iRule) — only the 1 pinned drift edge.
- ✅ **graph_export** (`graph_export.py`) → `tcl-bigip::graph` (`export_graph` +
  `filter_to_subgraph` BFS + DOT/JSON/Mermaid serialisers; JSON hand-written for
  `json.dumps(indent=2)` key-order parity). Byte-parity golden-tested.
- ✅ **`f5 graph` (alias `deps`)** wired end to end (`load_paths` →
  `build_bigip_object_graph` → `export_graph` → file/stdout). Verified
  byte-identical to the Python CLI across all 3 formats × the full flag matrix
  (`--seed` / `--reverse` / `--max-depth`).

> **Registry-data drift — RESOLVED.** The generated Rust registry **data**
> (`tcl-registry/src/bigip/data`) had drifted from current Python (a stale
> 992-kind baseline; current Python has 798). It has been **regenerated** from
> the reconciled Python `OBJECT_SPECS` by the restored
> `scripts/registry-audit/gen_bigip_rust.py`. All drift is gone: `candidate_kinds_*`
> and `candidate_registry_kinds_for_display` match Python on every probe (drift
> pins deleted), and the BIG-IP graph is byte-identical to the Python `f5 graph`
> on real configs. Re-run the generator whenever the Python `OBJECT_SPECS`
> baseline moves.

## Prioritised remaining roadmap

Done so far (f5): `merge`, `split`, `diff`, `explain` (model-based); `extract`
(UCS). The **f5 input layer + UCS** foundation (`tcl-bigip-io`) is complete.
Remaining, in dependency order:

1. **BIG-IP ref-graph** (`build_bigip_object_graph`, `tcl-bigip` extension) —
   unblocks `stats`, `cleanup`, `grep`, `validate`, `graph`, `rename`. Highest
   leverage; port + golden-test in isolation first. **(in progress)**
2. **tmsh / AS3 emit engines** → `tmsh` (+delta), `convert`; **redaction** →
   `redact`/`unredact`.
3. **`tcl-cli-serialise`** (port `tooling/cli/serialise.py`) — unblocks the tcl
   JSON verbs (symbols/callgraph/symbolgraph/dataflow/diagram, tcl `diff`).
   Gated by analyser parity for full fidelity.
4. **`registry-dump`** (port `registry_snapshot.py`) — real parity, both CLIs; finicky (field-by-field).
5. **f5 remote** (`tcl-bigip-remote`: REST/SSH) and **PCAP** (`tcl-bigip-pcap`) — pure-Rust crates; covers `fetch`/`push`/`pull`/`explain-flow`/`enrich-*`/`pcap-remap`. (UCS/OpenPGP/AES already done in `tcl-bigip-io`.)
6. **Query DSL** (`tcl-bigip-query`) and **tclpkg** (`pkg`/`venv`/`docker`) — the two largest sub-systems. The query DSL (`dialects/f5/query`, ~18k LOC) is being ported in verifiable increments:
   - ✅ **front-end** — lexer (`lexer.py`), AST (`ast.py`), recursive-descent parser (`parser.py`) → `tcl-bigip-query::{lexer,ast,parser}`. Offsets are code-point indices (scanned over a `Vec<char>`) so they match Python exactly. Golden differential-tested: `scripts/codegen/gen_f5_query_fixtures.py` captures the Python token stream + AST (serialised to a tagged JSON shape) and the lexer/parser errors for a query matrix incl. every `examples.py` cookbook entry; the Rust front-end re-derives the same JSON and asserts equality (self-contained — no Python at test time).
   - ✅ **value model** (`values.py`) → `value.rs`: the `Value` enum (jq-shaped + `Stream`/`PathRef`/`ObjectRef`/`Drop`) plus `truthy`, Python-`==` `py_eq`, jq `sort_cmp` (port of `_sort_key`), `coerce_scalar`; and `jsonfmt.rs` — `json.dumps(indent=2 / separators)` faithful (ensure_ascii + surrogate pairs + Python float `repr`).
   - ✅ **output** (`output.py`) → `output.rs`: auto/scf/raw/paths/json/table/table-lineart (renderer-registry fall-through pending).
   - ✅ **evaluator** (`evaluator.py`) → `eval.rs` + `special.rs`: the full jq core over JSON/plain values + `Stream`/`ObjectRef`/`PathRef`; all 29 special forms. `Container` projection-navigation branches and edit-plan application are stubbed (unreachable from JSON roots) pending the projection / edit-plan increments.
   - 🚧 **builtin library** (`builtins.py`, 9.7k LOC, 244 builtins) — ported in category batches, each with golden coverage. **Done: ~60** (value/stream/string-core/math-core + the special forms). Pre-stubbed category modules (`math`/`net`/`time_dt`/`regex_str`/`value2`/`encoding`) are wired and being filled.
   - ⬜ **projection** (`projection/`) — Container/ObjectRef/PathRef over the typed `tcl-bigip` model (`canon_fields()` / `BigipList`, reused — do not re-derive). The 178 KB `projection/_data.py` dispatch table is the bulk; unblocks BIG-IP-rooted queries + the graph/refs builtins.
   - ⬜ **renderers** (`renderers/`); then the **edit-plan / rename** mutation engine (`edit_plan.py`); then network **probes** (`_probes.py`), side-**inputs** (`_inputs.py`), and the `f5 query` / `validate` verb wiring.
   - **Note:** `f5 validate` does **not** depend on the query DSL — it runs `dialects/f5/bigip/lint.run_lint` (8 rules over the model + ref-graph + `tcl-irules` object-refs + `tcl-registry` event lookup, all already ported), so it can land independently of this item.
7. **tcl-only engine-gap verbs**: `help` (KCS SQLite), `minimize`, `find-legacy`, `explore`/`diagram` (explorer report), `compwasm` (wasm pipeline), `dis` (VM compiler).
8. **Engine-gap closure** (separate workstreams): analyser, optimiser, lowering/VM-compiler — these flip diag/opt/dis to parity.
9. **Cutover** — native-binary packaging, remove `[project.scripts]` entries, rework `zipapp-tcl`/`zipapp-f5`, delete CLI-exclusive Python (NOT the shared `dialects/f5/bigip` etc. still imported by the Python LSP server/analyser/ai).

## Adding a verb (the pattern)

1. Resolve inputs via `tcl_cli_support::read_input_documents` (+ `combine_sources`).
2. Call the pure engine core (existing crate or a new per-subsystem lib).
3. Format output to match the Python verb exactly (field order, separators;
   JSON via `serde_json::to_string_pretty` with field-ordered structs).
4. Write via `write_text_output` / `write_highlighted_output`.
5. Capture a golden from the Python CLI and add a test in the matching parity
   suite (`rust/tcl-cli/tests/cli_parity.rs` or `rust/f5-cli/tests/cli_parity.rs`)
   — **only** for verbs whose engine is fully ported (don't assert byte-parity on
   engine-gapped verbs; for those, verify ad-hoc against the Python CLI and note
   the gap here).

## Verification

Each CLI has a `tests/cli_parity.rs` that runs the built binary and diffs stdout
against committed `tests/fixtures/*.golden` files captured from
`python -m tooling.{tcl,f5}.main`. Self-contained (no Python at test time), so it
runs under `cargo test --workspace`. Current coverage: 7 tcl + 11 f5 golden
tests, plus `tcl-bigip-io`'s FIPS-197 AES KATs and UCS differential tests
(self-contained — fixtures captured once from the Python CLI and `gpg
--symmetric`; no Python/gpg at test time).
`make rust-tcl` / `rust-f5` / `rust-clis` build the binaries.
