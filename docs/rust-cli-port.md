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

The BIG-IP **object model + config parser** are ported (`tcl-bigip`), but **every
analysis/emit engine is still Python-only** (`dialects/f5/bigip/*`,
`dialects/f5/query/*`). So apart from `completion`, no f5 verb is wireable yet.

- `completion` — ✅ idiomatic (clap_complete).
- Everything else (`stats`, `cleanup`, `grep`, `diff`, `validate`, `explain`,
  `graph`, `tmsh`, `convert`, `split`, `merge`, `rename`, `redact`/`unredact`,
  `query`, `fetch`/`push`/`pull`, `explain-flow`, `enrich-*`, `pcap-remap`,
  `registry-dump`, the `irule` group) — ⛔ stub, pending engine ports.

**Keystone:** the BIG-IP **reference graph** (`build_bigip_object_graph` /
`dialects/f5/bigip/irules_object_refs.py`, ~944 LOC). `stats`, `cleanup`, `grep`,
`explain`, `rename`, `graph` all build on it — port + golden-test it first
(diff against the prebuilt `irules_object_refs_graph.json`).

## Prioritised remaining roadmap

1. **BIG-IP ref-graph** (`tcl-bigip` extension) — unblocks ~6 f5 verbs. Highest leverage.
2. **f5 input plumbing** (read `.conf`/`.scf`/`.ucs`, parse → `BigipConfig`) — shared by all f5 config verbs.
3. **f5 core engines** on the model: `stats` → `grep`/`cleanup` → `diff`/`split`/`merge` → `validate`/`explain` → `graph` → `tmsh`(+delta) → `convert` → `rename`/`redact`.
4. **`tcl-cli-serialise`** (port `tooling/cli/serialise.py`) — unblocks the tcl JSON verbs (symbols/diagram/callgraph/diff). Gated by analyser parity for full fidelity.
5. **`registry-dump`** (port `registry_snapshot.py`) — real parity, both CLIs; finicky (field-by-field).
6. **f5 remote** (`tcl-bigip-remote`: REST/SSH/UCS/OpenPGP/AES) and **PCAP** (`tcl-bigip-pcap`) — pure-Rust crates.
7. **Query DSL** (`tcl-bigip-query`) and **tclpkg** — the two largest sub-systems.
8. **Engine-gap closure** (separate workstreams): analyser, optimiser, lowering/VM-compiler — these flip diag/opt/dis to parity.
9. **Cutover** — native-binary packaging, remove `[project.scripts]` entries, rework `zipapp-tcl`/`zipapp-f5`, delete CLI-exclusive Python (NOT the shared `dialects/f5/bigip` etc. still imported by the Python LSP server/analyser/ai).

## Adding a verb (the pattern)

1. Resolve inputs via `tcl_cli_support::read_input_documents` (+ `combine_sources`).
2. Call the pure engine core (existing crate or a new per-subsystem lib).
3. Format output to match the Python verb exactly (field order, separators;
   JSON via `serde_json::to_string_pretty` with field-ordered structs).
4. Write via `write_text_output` / `write_highlighted_output`.
5. Capture a golden from the Python CLI and add a test in
   `rust/tcl-cli/tests/cli_parity.rs` — **only** for verbs whose engine is fully
   ported (don't assert byte-parity on engine-gapped verbs).

## Verification

`rust/tcl-cli/tests/cli_parity.rs` runs the built binary and diffs stdout against
`tests/fixtures/*.golden` captured from `python -m tooling.tcl.main`. Self-contained
(no Python at test time), so it runs under `cargo test --workspace`. `make rust-tcl`
/ `rust-f5` / `rust-clis` build the binaries.
