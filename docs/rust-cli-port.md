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
| `symbols` (`syms`) | ✅ wired | ports `_collect_scope_symbol_entries` + `_detect_event_entries` over the analyser scope tree (`tcl-compiler` `analyser`). `Scope.procs`/`variables` are `HashMap`s, so sorted by defining-token offset to recover Python's source-order dict iteration (the analyser now gives list bindings like `foreach {a b}` per-name spans, so multi-var loops stay deterministic + source-ordered). Text + JSON. Golden-tested on the faithful subset (`symbols.tcl`: namespaces, nested procs, namespace variables, params, multi-var loops, `when` events). **Analyser gaps** (output tracks the analyser, converges as it does): explicit `::`-qualified proc names report the simple name (`::unknown` → `unknown`); some implicitly-created variables (e.g. `append`-created globals) aren't recorded |
| `symbolgraph` (`symbol-graph`) | ✅ wired | ports `build_symbol_graph` + `_scope_to_dict` + `find_proc_call_sites` (`analyser/semantic_graph.py`) over the analyser scope tree + `command_invocations`. Ordered JSON via `serde_json` `preserve_order`; `HashMap`s sorted by defining-token offset, and proc-scope variables ordered params-first (params share the proc-name span) then body locals by offset. Text + JSON. Golden-tested on the faithful subset (`symbols.tcl`). **Analyser gaps:** explicit `::`-qualified proc names report the simple name — which also skews `ref_count`/`proc_references` (Python matches `inv.name == proc.name` against the as-written name) — and some variable references aren't tracked; converges as the analyser does |
| `callgraph` (`call-graph`) | ⛔ stub | **blocked on the interproc engine:** Rust `ProcSummary.calls` is the *transitive* closure, but `build_call_graph` needs the *direct* (local) call set (Python uses `local.calls`). The graph would gain spurious `A→C` edges. Needs a direct-/local-calls accessor on `InterproceduralAnalysis` before it can be wired. (Node list + `effects` string + call-site resolution otherwise straightforward.) |
| `dataflow` (`dataflow-graph`) | ⛔ stub | gated by the **taint engine** (`find_taint_warnings` is `pub(crate)` + the T-code set is incomplete, same gap as `diag`) and a whole-unit taints accessor (`cu.top_level`/`cu.procedures[*].analysis.taints`). The `proc_effects`/summary half (interproc `pure`/`effect_reads`/`effect_writes`/`has_barrier` + `_effect_region_str`) is available |
| `diagram` | ⛔ stub | port of `tooling/diagram/extract.py` — a ~339-line IR walk (`lower_to_ir`) with Python-exact condition/`expr_text` rendering, `when_event_name` + event-registry multiplicity/priority, and side-effect-classified action labels. Lowering/IR-gapped; sizeable |
| `diff` | ⛔ stub | multi-layer AST/IR/CFG diff + serialise |
| `explore` | ⛔ stub | explorer report port |
| `find-legacy` | ⛔ stub | analyser-based detection |
| `registry-dump` | 🚧 partial | `profiles` + `objects` + `events` snapshots byte-parity (`tcl-registry::snapshot`). **`events`**: ports `event_graph_snapshot` — per-event protocol props (`EventProps`→`props` with the `transport` string/list/null remapping), firing order (`orderIndex`/`masterOrder`), flow chains (incl. the new `FlowChain.notes` data) + the content-addressed `validCommandsDigest` (sha2) over the `event_info` cross-product; emitted via the `snapshot.rs` `Json` builder (json.dumps indent=2 sort_keys). `commands` (and default `all`) deferred — they embed the per-command `traits`/`scalars` dicts + command prose (no clean byte-identical Rust mapping; the Rust `CommandSpec` is a different shape) |
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
| `merge` | ✅ byte-parity | golden-tested; `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config` (golden-tested) |
| `split` | ✅ byte-parity | uses `extract_blocks`/`parse_generic_header`; round-trip golden-tested; `--format tmsh` writes per-partition `.tmsh` files via `render_config` |
| `diff` (`changes`) | ✅ parity (add/remove/scalar) | ports `compute_diff` over the model; fields read from `canon_fields()`; accepts tmsh input via `to_scf`. **Gap:** object-list field *display* (pool `members`, data-group `records`) shows canonical JSON vs Python's dataclass `repr` — change *detection* is still correct. Golden-tested (add/remove text+JSON, scalar modify, tmsh input) |
| `explain` (`describe`) | ✅ byte-parity | ports `compute_explain` + the `resolve_name` resolution layer (model-based — does **not** need the ref-graph); walks `canon_fields()` (`BigipList` navigation) for profiles/iRules/persistence/SNAT/pool. Verified across virtual/pool/auto/short-name/not-found, text + JSON; golden-tested |
| `extract` (`ucs2scf`) | ✅ byte-parity (scf) | golden-tested. Ports `extract_ucs_file` onto `tcl-bigip-io`: reads a `.ucs` (plain gzip-tar **or** OpenPGP-encrypted), extracts to SCF in memory, writes verbatim. Encrypted archives decrypt purely in Rust. `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config`. interactive TTY passphrase prompting not yet wired in the binary (env-var / explicit only) |
| `graph` (`deps`) | ✅ byte-parity | full ref-graph (nodes + pilot/legacy/iRule edges) → `export_graph` (DOT/JSON/Mermaid). Verified end-to-end vs the Python CLI across all formats × `--seed`/`--reverse`/`--max-depth`. (Byte-parity on configs free of the documented registry-data drift.) |
| `stats` (`summary`) | ✅ byte-parity | object/partition counts, iRule LOC + events, top-referenced, orphans over the graph. Text + JSON; golden + end-to-end verified |
| `cleanup` (`clean`) | ✅ byte-parity | BFS-reachability orphan detection → reverse-topological `tmsh delete` script. `--keep`/`--no-keep-common`, text + JSON; golden + end-to-end verified |
| `validate` (`lint`) | ✅ byte-parity | New **`tcl-bigip::lint`** module (sibling of `cleanup`/`stats`/`graph`) — `run_lint` + all 8 rules (orphan-monitor, empty-pool, virtual-without-pool, pool-without-monitor, irule-deprecated-command/empty-when/unknown-event/missing-`<kind>`). Reuses the typed model + `stats::is_root_kind` + `tcl-irules` object-refs + `tcl-registry` events + the `graph.rs` `resolve_name`; **does NOT use the query DSL** (lint walks raw model fields the projection would transform). Verb ports `text`/`json`/`sarif` + severity-based exit codes (0/1/2). Verified byte-identical end-to-end vs `python -m tooling.f5.main validate` across all formats + filters; golden-tested (`validate_parity.rs`, 15 cases) |
| `rename` | ✅ byte-parity | the edit-planner verb — a thin shell over the `f5 query` engine (`rename(OLD, NEW)` routed through `run_query`; `tcl-bigip-query::rewrite`). Default unified-diff preview / `--write` / `-o` / `--in-place`; `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config` (diff preview stays SCF↔SCF; `--in-place`+`tmsh` rejected). Golden-tested |
| `tmsh` (+delta), `convert` (`scf2as3`/`ucs2scf`), `redact`/`unredact` | ✅ byte-parity | `tmsh`: SCF→tmsh `create`/`modify`/delta emitter (`tcl-bigip::tmsh_emit`) — also powers `--format tmsh` in `extract`/`split`/`merge`/`redact`/`unredact`/`rename`. `convert`: AS3 declaration engine (`tcl-bigip::convert`) + ucs2scf (reuses `extract`). `redact`/`unredact`: secret-stripping + IP-remap (`tcl-bigip::redact`) incl. a hand-ported CPython-MT19937 `--shuffle`, with round-trip + sidecar-`.map` parity. `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config` (tmsh `modify` verb; no pre-edit original threaded, so delta treats all objects as created — matching Python); golden-tested |
| `grep`, `pcap-remap`, `enrich-pcapng`/`enrich-wireshark` | ✅ byte-parity | `grep`: `compute_grep` ref-graph search (`tcl-bigip::grep`; literal/regex/CIDR seeds, direction/depth, text/json/tmsh). `pcap-remap`: PCAP IP-remap (`tcl-bigip::pcap_remap`; classic libpcap + pcapng + F5 trailers, checksum byte-parity; custom `--schema` TOML deferred). `enrich-*`: config→capture/profile enrichment (`tcl-bigip::pcap_enrich`/`wireshark_profile`; Wireshark profile + NameIndex + direct-write PCAPNG annotation byte-parity; `editcap`-driven libpcap→pcapng conversion deferred as in Python) |
| `fetch`/`push`/`pull` | ✅ offline parity / live untested | Remote verbs (`f5-cli` `commands::remote`). `push --dry-run` request dump + `resolve_credentials` precedence/errors byte-parity (golden-tested offline). Live iControl REST via `ureq`+`rustls` (push PUT/POST, pull GET, fetch UCS→SCF via `tcl-bigip-io`) — implemented, only runs against a real device. **SSH transport deferred** (`russh` needs `unsafe`/C deps) |
| `irule` group | 🚧 partial | `event-order`/`extract`/`format`/`minify`/`event-info`/`lint`/`context` byte-parity (reuse `tcl-registry`/`tcl-bigip`/`tcl-lsp-core`). **`event-info`**: ports `lookup_event_info` over the reconciled command registry (`CommandRegistry::event_info` + the event-validity cross-product) + a generated `event_descriptions` prose table; verified byte-identical across all 178 events (text + JSON). **`lint`**: pure wiring over the already-ported `tcl-bigip::lint` engine (the same one powering byte-parity `f5 validate`) — extends the irule-input loader (`commands::irule` `load_irule_inputs`) to also return origin-keyed `configs`/`sources`, synthesising a single-rule `BigipConfig` for standalone `.irule`/`.tcl` files (`/{stem}`) and inline `--source` (`/inline_{n}`); calls `run_lint(category="irule")` with per-origin joined rule bodies and reuses the `f5 validate` `to_json`/`to_text` formatters + severity exit codes (error→2/warning→1/else 0). Golden-tested (`irule_parity.rs`) on standalone `.irule`, bigip.conf (all four irule rules), missing-object refs, `--source`, `--severity`, text + JSON. **`context`**: new **`tcl-bigip::irule_context`** engine (port of `ai/shared/irule_context.py`) — `build_irule_context` (object-reference walk over `tcl-irules` + a `resolve_name`-over-model resolver + one-hop transitive pool→node/monitor expansion + range-based source slices) and the `context_bundle_to_json`/`bundles_to_json`/`context_bundle_to_text` renderers (`json.dumps(indent=2, ensure_ascii)`-faithful insertion-ordered `Json`; 7 per-kind summarisers + 8 text renderers). The verb merges configs once (`tcl-bigip::lint::merge_configs`) for cross-file resolution, iterates rules with `--rule`/`--no-transitive`, and dispatches directory (one file per rule) / single-file / stdout, JSON `{"bundles":[…]}` vs text chunks. Golden-tested across every section (pool/data-group/persistence/snat-pool/profile/monitor/node + unresolved + slices), realistic `bigip.conf`, `--no-transitive`, standalone `.irule`, inline `--source`, and the no-iRules exit-1 path; verified byte-identical over all `.conf`/`.scf` fixtures × {text, json, no-transitive}. **`trace`**: a purely **static** event-handler trace (no VM — the old "compiler-VM" label was wrong) — `\bwhen EVENT\s*\{` block-match (case-insensitive) + balanced-brace body slice + first-token command extraction + object-reference walk (`tcl-irules` + the shared `irule_context::classify_kind`/`resolve_reference` resolver over the merged config). Text + JSON, golden-tested (realistic + all-kinds configs, case-insensitivity, no-match exit-1). `pgo` (needs the `compiler/pgo` branch-reorder engine; `--capture` needs tclsh) + `explain-flow` (needs the iRule simulator / VM) remain deferred |
| `explain-flow` | ⛔ blocked | needs the **iRule simulator** (`simulate_irule_for_session` → the Tcl runtime/VM, the excluded `runtime/rust` crate) |
| `query` (`q`) | ✅ byte-parity | **`f5 query` runs end-to-end byte-identical to the Python CLI** — read-only AND mutating. `tcl-bigip-query` ports the full engine: front-end (lexer/AST/parser), value model + `json.dumps`-faithful output (auto/json/raw/paths/scf/table/table-lineart), the evaluator (full jq core + all 29 special forms), **244/244 builtins**, the **BIG-IP projection** layer (Container/ObjectRef/PathRef over the typed model), graph `refs`/`referenced_by` + rule `.refs`, the **edit-plan** (field-value + identity-field/`rename*` mutations with a faithful `difflib.unified_diff` port, `--write`/`--in-place`/diff, `--format tmsh`/`tmsh-delta`/`--transaction` rendering of the rewritten config with the `--in-place`+tmsh guard + strict-UTF-8 in-place reads, and cross-file edits via `$name`), `--partition` source binding, `-f`/`--from-file`, **renderers** (`--render mermaid`/`gantt`/`ascii-blocks`), **side-inputs** (`--input-json`/`jsonl`/`csv`/`f5log` + loaders), **live probes** (dns/ping/tls/x509 + `cert_load`, `--enable-probes` gating; `x509_parse` byte-parity), `--merge` (cross-file unified namespace + cross-file refs), and `--help-dsl`/`--help-examples`/`--help-renderers`/`--help-inputs`. **All 24 cookbook examples + a broad query matrix verified byte-identical** end-to-end vs `python -m tooling.f5.main query`. ~25 golden-differential suites. **Documented deferrals:** `--help-builtins`/`--help-manual` (the per-function prose metadata was intentionally omitted from the Rust registry), live `url_*` HTTP (stub returns the faithful result-dict shape; gating + the `http_*` accessors are real), `ucs_cert`'s UCS reader (cross-layer), PKCS#12 `cert_load`, and the long-tail object kinds the Rust model doesn't carry |

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

> **Command-dialect reconcile — RESOLVED.** The Rust command registry
> (`tcl-registry/src/commands/**`) encoded Tcl-version availability and Tk
> membership in the `DialectSet` itself (`Some(ALL_TCL)`/`Some(TK)`/version
> subsets) where Python uses `dialects=None` (universal) or an explicit
> per-command frozenset. ~627 commands diverged, breaking the iRules
> event/command cross-product (`commands_for_event`). Reconciled every
> spec's `dialects:` field to mirror Python across all 13 modelled dialects
> via `scripts/registry-audit/reconcile_irules_dialects.py` (+ a
> `NON_IRULES_OPERATORS` aggregate). `valid_irules_commands_for_event` is now
> byte-identical to Python for all 176 events (HTTP_REQUEST: 1290). This is
> the registry-data half of the old `~191-vs-~1236` `irule event-info` gap.

> **`registry-dump` `events` — DONE; `commands`/`all` — still deferred.** The
> `profiles`/`objects`/`events` sections are byte-parity. `events` landed via
> `event_graph_snapshot` (`tcl-registry::snapshot`): the event-validity
> cross-product + sha2 `validCommandsDigest` come from `event_info`, plus a
> `FlowChain.notes` data add and the `EventProps`→`props` field/transport
> remapping (string for single, list for dual, null for none). `commands`/`all`
> still embed `command_registry_snapshot`'s per-command `traits` (≈50 keys) /
> `scalars` dicts, which mirror the Python `CommandSpec` *dataclass field
> layout* — no clean byte-identical Rust mapping (the Rust `CommandSpec` is a
> different shape, traits are bitflags). Same boundary as the `tcl`
> `registry-dump commands` deferral.

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
6. **Query DSL** (`tcl-bigip-query`) — ✅ **DONE** (byte-parity). The query DSL (`dialects/f5/query`, ~18k LOC) is fully ported across these increments, each golden-differential-tested:
   - ✅ **front-end** (`lexer`/`ast`/`parser`) — code-point offsets match Python; 93-query token+AST matrix + error cases.
   - ✅ **value model** (`value.rs`) + **`jsonfmt`** — `Value` enum, `truthy`/`py_eq`/jq `sort_cmp`, `json.dumps`-faithful (ensure_ascii + surrogate pairs + Python float `repr`).
   - ✅ **output** (`output.rs`) — auto/scf/raw/paths/json/table/table-lineart.
   - ✅ **evaluator** (`eval.rs` + `special.rs`) — full jq core + all 29 special forms.
   - ✅ **builtins** — **244/244** registered across the category modules (value/stream/string/regex/math/net-ip-CIDR/time/encoding/graph/rename/probes/extras). Math uses CPython's Lanczos gamma + A&S Bessel for bit-parity; net reproduces CPython 3.11 `ipaddress` IANA tables; regex via the `regex` crate (documented backref/lookaround divergence).
   - ✅ **projection** (`projection.rs`) — Container/ObjectRef/PathRef over the typed `tcl-bigip` model (bridges the `Vec<Placed>` model to the dict-per-kind DSL shape); core LTM kinds + pool-member/policy sub-objects + rule `.refs` (graph-backed); per-kind field maps reproduce the pilot-spec/PathRef/typed-string projection.
   - ✅ **graph** — `refs`/`referenced_by`/`references_to`/`check_partition_visibility` over `build_bigip_object_graph` (cross-file in `--merge`).
   - ✅ **edit-plan** (`edit_plan.rs` + `rewrite.rs`) — field-value (`=`/`|=`/`+=`/`-=`) + identity-field/`rename*` mutations; faithful `difflib.unified_diff`+`SequenceMatcher` port; `--write`/`--in-place`/diff + `renamed …` reports.
   - ✅ **renderers** (`renderers/`) — mermaid/gantt/ascii-blocks + `--render`/`--render-opt`.
   - ✅ **side-inputs** (`inputs.rs`) — `--input-json/jsonl/csv/f5log` + the `*_load` builtins.
   - ✅ **probes** (`probes.rs`) — `--enable-probes`-gated dns/ping/tls/x509; `x509_parse`/`cert_load` byte-parity; live `url_*` HTTP stubbed (deferral) + the pure `http_*` accessors real.
   - ✅ **runner + verb** (`runner.rs`, `f5-cli` `commands::query`) — multi-file, `--merge`, `$name` binding, output modes, exit codes, `--help-dsl`/`--help-examples`/`--help-renderers`/`--help-inputs`.
   - **Deferrals (documented):** `--help-builtins`/`--help-manual` (per-function prose metadata intentionally omitted from the Rust registry), live `url_*` HTTP body fetch, `ucs_cert`'s UCS reader (cross-layer), PKCS#12 `cert_load`, long-tail object kinds.
   - **Note:** `f5 validate` does **not** depend on the query DSL — it runs `dialects/f5/bigip/lint.run_lint` over already-ported engines, so it can land independently.
   **tclpkg** (`pkg`/`venv`/`docker`) remains the other large sub-system.
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
