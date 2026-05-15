# F5 CLI architecture

> **Audience:** Developer / Maintainer
> **Type:** Architecture

The `f5` CLI is a separate top-level binary from `tcl` and `irule`,
distributed as a single-file zipapp.  It is built around three
layers:

1. **Verb registry** (`explorer/verbs/f5/_registry.py`,
   `explorer/f5_cli.py`) — argparse subparsers, brief-help builder,
   and zero auto-discovery.  Each verb module decorates a configure
   function with `@verb(name, aliases=..., help=...)`; new verbs are
   wired in by importing them from `explorer/verbs/f5/__init__.py:load_verbs()`.

2. **Core analysis** (`core/bigip/`) — a parser
   (`core/bigip/parser.py`), a typed object model (`core/bigip/model.py`),
   a reference graph (`core/bigip/link_extract.py`), a lint registry
   (`core/bigip/lint/`), emitters (`core/bigip/emit.py`,
   `core/bigip/tmsh_emit.py`, `core/bigip/convert/as3.py`,
   `core/bigip/redact_map.py`, `core/bigip/pcap_remap.py`,
   `core/bigip/diff.py`, `core/bigip/explain.py`,
   `core/bigip/graph_export.py`, `core/bigip/stats.py`,
   `core/bigip/rewrite.py`), and a query DSL
   (`core/bigip/query/`, see
   [`f5-query-dsl.md`](../references/f5_query/dsl.md) for grammar, value model,
   edit pipeline, and jq-compatibility notes, and
   [`f5-query-dsl-builtins.md`](../references/f5_query/builtins.md) for the
   auto-generated per-function reference).  All verbs are thin
   shells over these modules.

3. **Networking** (`explorer/f5_remote/`) — stdlib-only iControl REST
   client (`rest.py`, using `http.client`), SSH transport
   (`ssh.py`, wrapping the system `ssh`/`scp` binaries via
   `subprocess`), UCS extraction (`ucs.py`, gzip+tar), credential
   resolution with XDG config (`auth.py`), and single-object pull/push
   (`object_io.py`).  No third-party dependencies.

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

`irule` is a sub-parser group rather than a top-level verb, hosting
five sub-verbs (`event-order`, `event-info`, `lint`, `trace`,
`extract`).  See [`explorer/verbs/f5/irule.py`](../../explorer/verbs/f5/irule.py)
for the registration pattern; new sub-groups follow the same shape.

## Query DSL

`core/bigip/query/` is the largest single core-analysis module.  It
exposes a small jq-flavoured language that surfaces the parsed
`BigipConfig` as a navigable tree (`.ltm.virtual[]`,
`.ltm.pool["/Common/x"]`, …), supports stream / list / scalar
values, and routes mutating expressions through the same
token-bounded rewrite engine `f5 rename` uses.  Identity-field
writes (`.<kind>[X].name = Y`) are kind-scoped so a pool and a
virtual sharing a full-path don't collide.

Internal layering:

- `lexer.py`, `ast.py`, `parser.py` — recursive-descent parser
  producing a small AST.
- `values.py`, `projection.py` — runtime value model (Stream,
  PathRef, ObjectRef, FieldSlot) and the BigipConfig → tree
  projection that backs path navigation.
- `evaluator.py` — walks the AST, collecting `EditOp`s into an
  `EditPlan` rather than mutating in place.
- `edit_plan.py` — routes identity writes through
  `rewrite.rename_object` with kind-scoping, materialises or
  extends compound list blocks (`rules`, `profiles`, `persist`,
  `policies`, `members`) on `+=` / `=`, applies field-edit
  splices bottom-up, and detects overlapping edits.
- `builtins.py` — the function library (`ip`, `partition`,
  `rename`, `rename_partition`, …); a registry decorator binds
  each function's signature, summary, deep-explanation prose, and
  worked examples so the same source feeds runtime dispatch,
  `f5 query --help-builtins NAME`, and the generated reference
  doc at `docs/references/f5_query/builtins.md`
  (`scripts/dev/gen_query_builtins_doc.py`).
- `grammar.py`, `examples.py` — terminal-friendly grammar
  reference and the worked-example cookbook surfaced by
  `--help-dsl` / `--help-examples`.
- `runner.py` — `;`-separated statements evaluate in order
  against the evolving source, so multi-step migrations
  (e.g. `rename_partition(...) ; .X[].destination |= ip(...)`)
  compose naturally.

## Reference graph

Every verb that walks references uses one builder:
[`build_bigip_object_graph`](../../core/bigip/link_extract.py).
It returns `(nodes_by_uri, edges)` covering both
configuration-property references (a virtual's `pool`, a pool member's
`monitor`, etc.) and iRule body references (`pool`,
`class match … <data-group>`, `persist`, `snatpool`, `virtual`,
`node`, `LSN::pool`, `STATS::*`, `ifile`, etc.).  `f5 cleanup`,
`f5 grep`, `f5 graph`, `f5 explain`, and `f5 stats` all consume the
same graph.  Lint rules in `core/bigip/lint/` see a *merged*
`BigipConfig` so cross-file refs aren't reported as orphans.

## IP-redaction model

`core/bigip/redact_map.py` defines a stable, reversible IP map:

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
- `f5 pcap-remap` applies the same map to a PCAP capture: rewrites
  IPv4/IPv6 src/dst at the IP layer, recomputes header + TCP/UDP/ICMP
  checksums, and sweeps the F5 HSB trailer (everything past
  `IP total_length`) for any byte sequence matching a known real
  address.  Replacements are length-preserving so the trailer's TLV
  structure stays valid.  L4 payload bytes are never scanned.

## tmsh emission

`core/bigip/tmsh_emit.py` walks `BigipConfig` in dependency order
(partitions → nodes → monitors → data-groups → profiles → snat-pools /
persistence → pools → iRules → virtuals) and renders each object as a
single `tmsh create` (or `--modify`) command.  Properties not modelled
by `BigipConfig` are recovered from the original SCF stanza text via
the block slicer in [`core/bigip/emit.py`](../../core/bigip/emit.py),
so iRule bodies, monitor send/recv strings, and similar arbitrary
content survive the round-trip verbatim.

## File layout

```
explorer/
├── f5_cli.py                     ← argparse entrypoint, brief help
├── f5_remote/                    ← network/auth (stdlib-only)
│   ├── auth.py                     credential resolution + XDG
│   ├── rest.py                     iControl REST client
│   ├── ssh.py                      ssh/scp wrapper
│   ├── ucs.py                      UCS archive handling
│   └── object_io.py                single-object pull/push
└── verbs/f5/
    ├── _registry.py              ← @verb decorator + brief help
    ├── _paths.py                 ← shared file-input helpers
    ├── cleanup.py grep.py …      ← one module per verb
    └── irule.py                  ← sub-parser group

core/bigip/
├── parser.py                     ← SCF tokeniser + block extractor
├── model.py                      ← BigipConfig dataclass family
├── link_extract.py               ← reference graph
├── cleanup.py grep.py            ← graph-based analyses
├── stats.py diff.py explain.py   ← analysis verbs' core
├── graph_export.py               ← DOT / JSON / Mermaid serialisation
├── lint/__init__.py              ← rule registry + 7 built-in rules
├── emit.py                       ← block slicer for split/merge
├── rewrite.py                    ← rename + redact pipeline
├── redact_map.py                 ← stable IP map + sidecar TOML
├── pcap_remap.py                 ← PCAP read/write/checksum
├── tmsh_emit.py                  ← SCF → tmsh script
└── convert/as3.py                ← SCF → AS3 declaration
```

## Adding a new verb

1. Create `explorer/verbs/f5/<name>.py` with a `@verb`-decorated
   `_configure(p, *, prog_name, default_dialect)` function that calls
   `p.set_defaults(handler=_run_<name>)`.
2. Add the import to `explorer/verbs/f5/__init__.py:load_verbs()`.
3. Put any non-trivial logic in `core/bigip/<name>.py` so it can be
   tested independently of argparse.
4. Add a CLI integration test in `tests/test_f5_<name>.py` and (for
   logic) a unit test in the same module.
5. Document the new verb in `docs/kcs/features/kcs-feature-f5-cli.md`
   and `README.md`.
