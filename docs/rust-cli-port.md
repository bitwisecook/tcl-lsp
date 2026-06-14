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
| `stats`, `cleanup`, `grep`, `validate`, `graph`, `rename` | ⛔ stub | need the BIG-IP **ref-graph** (keystone, below) |
| `tmsh`, `convert`, `redact`/`unredact` | ⛔ stub | need the tmsh / AS3 emit + redaction engines |
| `fetch`/`push`/`pull`, `explain-flow`, `enrich-*`, `pcap-remap`, `query`, `irule` group | ⛔ stub | need remote / PCAP / query-DSL ports |

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
- ⛔ **registry-first dispatch** (`references_via_spec` / pilot value-spec engine,
  `registry/{pilot,references,value_specs,properties}.py` — `value_specs.py`
  alone is ~1777 LOC, ~25 spec classes) — used *additively* alongside the legacy
  path; full `f5 graph` parity needs it (owns the `viaProperty`/edge set for the
  19 migrated properties). **Approved for full port.**
- ⛔ **irules_refs** (`extract_irules_object_references`, ~368 LOC) for iRule edges.
- ⛔ **graph_export** (`graph_export.py`, ~170 LOC) — DOT/JSON/Mermaid for `f5 graph`.

> **Registry-data drift (blocks full byte-parity, separate workstream):** the
> generated Rust registry **data** (`tcl-registry/src/bigip/data`, a
> `gen_bigip_rust.py` baseline) has drifted from current Python for ~14
> properties' `references` lists (e.g. `monitor` on `cm traffic-group`; `pool`
> /`last-hop-pool` on `gtm listener*`; `prober-pool` on `gtm datacenter`/`server`;
> `fw-enforced-policy`/`fw-staged-policy`/`service-policy` on `ltm virtual`;
> `gw` on `net route`). The query *logic* matches Python; these edges will
> diverge until the data baseline is regenerated. Pinned in
> `tcl-registry/tests/fixtures/object_query.drift.txt`.
>
> A second drift class surfaced via the graph edges: properties Python
> **migrated to the pilot specs** had their *legacy* `references` cleared (so
> `candidate_kinds_for_key` returns empty), but the generated Rust data still
> carries them — e.g. `persist` on `ltm virtual`. These show up as *extra* Rust
> legacy edges (pinned in `graph_edges_legacy.drift.txt`). The object-query
> golden didn't catch this class because it only recorded Python-non-empty rows;
> regenerating the registry data (or expanding that golden to probe every Rust
> property name) is the fix.

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
6. **Query DSL** (`tcl-bigip-query`) and **tclpkg** (`pkg`/`venv`/`docker`) — the two largest sub-systems.
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
