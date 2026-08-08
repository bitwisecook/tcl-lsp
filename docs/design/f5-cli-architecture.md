# F5 CLI architecture

> **Audience:** Developer / Maintainer
> **Type:** Architecture

The `f5` CLI is the native `f5-query` binary (crate `rust/f5-cli`), a
separate top-level binary from the `tcl` binary.  It is a pure-Rust,
`unsafe`-free binary — no Python, no zipapp — and is built around three
layers:

1. **Verb registry** (`rust/f5-cli/src/cli.rs`,
   `rust/f5-cli/src/lib.rs`) — a `clap` derive command tree with no
   auto-discovery.  Every verb is a variant of the `Command` enum in
   `cli.rs` (kebab-cased variant names become verb names,
   `#[command(visible_alias = …)]` supplies the aliases), and the
   `irule` verb group is a nested `IruleCommand` sub-enum.  Shared
   surfaces — the `--format` scf/tmsh/tmsh-delta flag, UCS passphrase
   handling, and the `f5mku` master-key flags — are reusable
   `#[derive(Args)]` structs flattened into each verb.  `lib.rs::dispatch`
   matches on the parsed `Command` and calls the matching handler in
   `rust/f5-cli/src/commands/` (one module per verb, e.g.
   `commands/grep.rs`, `commands/query.rs`, `commands/irule.rs`); a new
   verb is wired in by adding an enum variant, a `commands/` module, and a
   `dispatch` arm.  `main.rs` is a thin `f5_cli::run(std::env::args_os())`
   entry point (same thin-main-over-lib shape as `tcl-cli` /
   `tcl-lsp-server`).

2. **Core analysis** (`rust/tcl-bigip`) — the BIG-IP object model and
   config parser crate: a parser (`src/parser/`), a typed object model
   (`src/model/`, whose schema is reused from `tcl_registry::bigip`), a
   reference graph (`src/graph.rs` plus the reference extractors in
   `src/links.rs` / `src/refs.rs`), a lint / validation registry
   (`src/lint.rs`, `src/validator.rs`), emitters (canonical SCF via
   `src/canonical.rs`, `tmsh` script via `src/tmsh_emit.rs`, SCF→AS3 via
   `src/convert.rs`), secret/IP redaction (`src/redact.rs`,
   `src/secrets.rs`), PCAP remapping (`src/pcap_remap.rs` with the parsed
   F5 HSB trailer in `src/f5_trailer.rs`), object statistics
   (`src/stats.rs`), and the graph-driven analyses `src/cleanup.rs` /
   `src/grep.rs`.  The `f5 query` DSL lives in its own crate,
   `rust/tcl-bigip-query` (see
   [`f5-query-dsl.md`](../references/f5_query/dsl.md) for grammar, value
   model, edit pipeline, and jq-compatibility notes, and
   [`f5-query-dsl-builtins.md`](../references/f5_query/builtins.md) for the
   auto-generated per-function reference).  The verb handlers in
   `rust/f5-cli/src/commands/` are thin shells over these two crates —
   they do file I/O and argument shaping, then hand the parsed
   `BigipConfig` to the engine (`diff`, `explain`, and the rest all build
   on `tcl-bigip`).

3. **Networking** (`rust/tcl-bigip-io` + `rust/f5-cli/src/commands/remote/`)
   — `tcl-bigip-io` is the pure-Rust input layer: UCS archive extraction
   (`src/ucs.rs`, gzip-tar via `flate2` + `tar`, with encrypted UCS
   handled as an OpenPGP-symmetric message in `src/openpgp.rs` /
   `src/aes_cfb.rs` using the RustCrypto `aes` stack — no shelling out to
   `gpg`) and the `read_path` / `load_paths` resolver (`src/paths.rs`).
   The live-device transport lives in `rust/f5-cli/src/commands/remote/`:
   an iControl REST client (`rest.rs`, built on `ureq` 3.x + `rustls`,
   with `base64` Basic-auth encoding and self-signed-cert handling via
   `TlsConfig::disable_verification`), credential resolution with XDG
   config (`auth.rs`), single-object pull/push request shaping
   (`object_io.rs`), and an SSH transport slot (`ssh.rs`).  SSH/scp is
   currently a clean deferral stub — an in-process client would pull in
   `unsafe`/C dependencies the workspace forbids — so `--transport ssh`
   (and any `auto` fallback that reaches it) returns a deferral error;
   `--transport rest` is the supported path.  These verbs (`fetch` /
   `push` / `pull`) are shared by the credential resolver and REST agent.

## Verb taxonomy

The verbs cover the operator workflow end-to-end:

```
acquire ─→ analyse ─→ transform ─→ round-trip
fetch       stats       rename       pull
extract     graph       redact       push
            explain     unredact
            diff        pcap-remap
            grep        split / merge
            cleanup     convert (UCS↔SCF / SCF→AS3)
            query       tmsh (SCF→tmsh script)
            validate    query (DSL-driven property edits)
            irule …
```

`query` straddles the analyse + transform columns — it is both a
read-only filter / projector (replacing some `grep` / `stats`
patterns when the predicate is property-shaped) and a write-back
engine that supersedes `rename` for any DSL-expressible identity
or property edit.  `f5 rename` is now a thin shell that constructs
a `rename(OLD, NEW)` expression and runs it through the query
engine, so the two verbs share one rewrite path.

`irule` is a sub-command group rather than a top-level verb: the
`IruleCommand` sub-enum in `rust/f5-cli/src/cli.rs` hosts its sub-verbs
(`event-order`, `event-info`, `lint`, `trace`, `extract`,
`format`, `minify`, `context`), dispatched by
[`commands::irule::run_irule`](../../rust/f5-cli/src/commands/irule.rs);
new sub-groups follow the same nested-`Subcommand` shape. `pgo`
(profile-guided branch-reorder suggestions) is deliberately not a
sub-verb here — see the module doc on `commands::irule` and issue #1315.

## Query DSL

`rust/tcl-bigip-query` is the largest single core-analysis crate.  It
exposes a small jq-flavoured language that surfaces the parsed
`BigipConfig` as a navigable tree (`.ltm.virtual[]`,
`.ltm.pool["/Common/x"]`, …), supports stream / list / scalar
values, and routes mutating expressions through the same
token-bounded rewrite engine `f5 rename` uses.  Identity-field
writes (`.<kind>[X].name = Y`) are kind-scoped so a pool and a
virtual sharing a full-path don't collide.

Internal layering:

- `lexer.rs`, `ast.rs`, `parser.rs` — recursive-descent parser
  producing a small AST.
- `value.rs`, `projection.rs` — runtime value model (Stream,
  PathRef, ObjectRef, FieldSlot) and the BigipConfig → tree
  projection that backs path navigation.
- `eval.rs` — walks the AST, collecting `EditOp`s into an
  `EditPlan` rather than mutating in place.
- `edit_plan.rs` — routes identity writes through
  `rewrite.rs`'s `rename_object` with kind-scoping, materialises or
  extends compound list blocks (`rules`, `profiles`, `persist`,
  `policies`, `members`) on `+=` / `=`, applies field-edit
  splices bottom-up, and detects overlapping edits.
- `builtins/` — the function library (`ip`, `partition`,
  `rename`, `rename_partition`, …), split into per-category modules
  (`net.rs`, `string.rs`, `rename.rs`, `graph.rs`, …); a registry binds
  each function's signature and metadata so the same source feeds runtime
  dispatch, `f5 query --help-builtins NAME`, and the generated reference
  doc at `docs/references/f5_query/builtins.md`.
- `grammar.rs`, `examples.rs`, `manual.rs` — terminal-friendly grammar
  reference and the worked-example cookbook surfaced by
  `--help-dsl` / `--help-examples` / `--help-manual`.
- `runner.rs` — `;`-separated statements evaluate in order
  against the evolving source, so multi-step migrations
  (e.g. `rename_partition(...) ; .X[].destination |= ip(...)`)
  compose naturally.

## Reference graph

Every verb that walks references uses one builder in
[`rust/tcl-bigip/src/graph.rs`](../../rust/tcl-bigip/src/graph.rs).
It returns nodes-by-URI plus edges covering both
configuration-property references (a virtual's `pool`, a pool member's
`monitor`, etc.) and iRule body references (`pool`,
`class match … <data-group>`, `persist`, `snatpool`, `virtual`,
`node`, `LSN::pool`, `STATS::*`, `ifile`, etc.); the reference
extractors live alongside in `src/links.rs` / `src/refs.rs`.  `f5
cleanup`, `f5 grep`, `f5 graph`, `f5 explain`, and `f5 stats` all consume
the same graph.  Lint rules in `rust/tcl-bigip/src/lint.rs` see a
*merged* `BigipConfig` so cross-file refs aren't reported as orphans.

## IP-redaction model

`rust/tcl-bigip/src/redact.rs` defines a stable, reversible IP map:

- Source CIDRs are inferred from the data: every public IPv4 literal
  is grouped by its enclosing `/24` (or `/64` for IPv6).  Operators
  can override with `--source-cidr CIDR` (repeatable).
- Each unique source CIDR is assigned a same-prefix-length target
  CIDR out of a configurable pool (default RFC1918 plus `fd00::/8`).
  Allocation is greedy by first-seen order so the same input always
  produces the same map.
- Within each CIDR, host bits map either **direct** (identity, default)
  or **shuffle** (a deterministic Fisher-Yates permutation seeded by a
  per-CIDR key derived from `--seed`).  Shuffle is capped at /20-worth
  of host bits (1M permutation entries); wider widths fall back to
  direct silently.
- The full assignment + IP cache lives in a TOML sidecar
  (`<output>.redact.toml` by default).  `f5 redact --map-file PATH`
  also accepts the file as input — when supplied, every prior
  assignment is reused so successive redactions stay consistent
  (essential when iterating with F5 support across many redacted
  artefacts).
- `f5 unredact` reads the sidecar and walks the map in reverse over
  any text (configs, support emails, log snippets).
- `f5 pcap-remap` (`rust/tcl-bigip/src/pcap_remap.rs`) applies the same
  map to a PCAP capture: rewrites IPv4/IPv6 src/dst at the IP layer,
  recomputes header + TCP/UDP/ICMP checksums, and sweeps the F5 HSB
  trailer (parsed via `src/f5_trailer.rs`) for any byte sequence matching
  a known real address.  Replacements are length-preserving so the
  trailer's TLV structure stays valid.  L4 payload bytes are never
  scanned.

## tmsh emission

`rust/tcl-bigip/src/tmsh_emit.rs` walks `BigipConfig` in dependency order
(partitions → nodes → monitors → data-groups → profiles → snat-pools /
persistence → pools → iRules → virtuals) and renders each object as a
single `tmsh create` (or `--modify`) command.  Properties not modelled
by `BigipConfig` are recovered from the original SCF stanza text via the
canonical block emitter in
[`rust/tcl-bigip/src/canonical.rs`](../../rust/tcl-bigip/src/canonical.rs),
so iRule bodies, monitor send/recv strings, and similar arbitrary
content survive the round-trip verbatim.

## File layout

```
rust/f5-cli/                          ← the `f5-query` binary crate
├── src/main.rs                       ← thin f5_cli::run(args) entry point
├── src/cli.rs                        ← clap Command / IruleCommand tree
├── src/lib.rs                        ← run + dispatch (verb registry)
├── src/f5mku.rs                      ← f5mku master-key crypto helpers
└── src/commands/                     ← one module per verb
    ├── cleanup.rs grep.rs …            (graph-driven analyses)
    ├── query.rs                        (query-DSL front end)
    ├── irule.rs                        (irule sub-command group)
    └── remote/                        ← network/auth
        ├── auth.rs                     credential resolution + XDG
        ├── rest.rs                     iControl REST client (ureq+rustls)
        ├── ssh.rs                      ssh transport slot (deferral stub)
        └── object_io.rs                single-object pull/push

rust/tcl-bigip/src/                   ← BIG-IP object model + parser
├── parser/                           ← SCF tokeniser + block extractor
├── model/                            ← BigipConfig typed object family
├── graph.rs links.rs refs.rs         ← reference graph + extractors
├── cleanup.rs grep.rs                ← graph-based analyses
├── stats.rs                          ← object statistics
├── lint.rs validator.rs              ← lint rule registry
├── canonical.rs                      ← canonical SCF block emitter
├── redact.rs secrets.rs              ← stable IP map + secret redaction
├── pcap_remap.rs f5_trailer.rs       ← PCAP read/write/checksum + trailer
├── tmsh_emit.rs                      ← SCF → tmsh script
└── convert.rs                        ← SCF → AS3 declaration

rust/tcl-bigip-io/src/                ← input layer
├── ucs.rs                            ← UCS archive handling (gzip-tar)
├── openpgp.rs aes_cfb.rs             ← encrypted-UCS OpenPGP-symmetric
└── paths.rs                          ← read_path / load_paths resolver

rust/tcl-bigip-query/src/             ← the f5 query DSL engine
```

## Adding a new verb

1. Add a variant to the `Command` enum in `rust/f5-cli/src/cli.rs` (the
   kebab-cased variant name becomes the verb; add
   `#[command(visible_alias = …)]` for aliases and flatten any shared
   `Args` structs).
2. Add a `dispatch` arm in `rust/f5-cli/src/lib.rs` calling the handler.
3. Create `rust/f5-cli/src/commands/<name>.rs` with the handler and
   register the module in `commands/mod.rs`.  Put any non-trivial logic
   in `rust/tcl-bigip` (or `rust/tcl-bigip-query`) so it can be tested
   independently of `clap`.
4. Add a CLI integration test under `rust/f5-cli/tests/` and (for logic)
   a unit test in the owning crate.
5. Document the new verb in `docs/kcs/features/kcs-feature-f5-cli.md`
   and `README.md`.
