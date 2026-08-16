# `f5 query` Reference

Man-page-style canonical reference for everything a user touches
on the `f5 query` DSL.  Every function, operator, output mode,
flag, sample config, F5 KB cross-reference, and on-disk format
lives here.

For engine internals (architecture, evaluator, edit-plan
dispatch, projection layer, parser AST shape) see
[`docs/design/f5-query-engine-internals.md`](../../design/f5-query-engine-internals.md).

## Documents

- [`manual.md`](manual.md) — the comprehensive reference manual.
  Quick-lookup index, grammar, operators, builtins families,
  probe taxonomy, cert dict shape, sample configs, cert
  one-liners, end-to-end walkthroughs, 100% coverage map. The
  curated long-form companion to `f5 query --help-manual`, which
  emits an auto-generated grammar + builtins + examples trio rather
  than this file's prose.
- [`dsl.md`](dsl.md) — the full DSL grammar reference (EBNF,
  divergences from jq, design rationales).  Companion to
  `f5 query --help-dsl`, which emits an abridged version of the same
  grammar.
- [`builtins.md`](builtins.md) — hand-maintained alphabetical
  catalogue of every builtin function with signature, examples,
  return type, and category, kept in sync by hand against the
  registry in `rust/tcl-bigip-query/src/builtins/`.
  `f5 query --help-builtins [NAME]` prints the registry's own
  metadata (name / category / arity / flags) rather than this
  file's prose.
- [`f5-kb-monitor-articles.md`](f5-kb-monitor-articles.md) — F5
  Knowledge-Base cross-reference for the `ltm monitor http(s)` /
  cert-audit recipes (K2167, K3451, K3224, K12531, K10655,
  K16526, K29224049, K000148880).

## How to look something up

| Looking for | Where |
|---|---|
| A single builtin's signature + examples | `manual.md` → builtin family link → `builtins.md` anchor.  CLI: `f5 query --help-builtins NAME`. |
| Grammar of a syntactic form | `dsl.md`.  CLI: `f5 query --help-dsl`. |
| How probes / cert audit work | `manual.md#network-probes-network-probes`, `manual.md#x509-cert-dict-shape-x509-cert-dict-shape`. |
| Why a monitor check fails (response truncation, etc.) | `f5-kb-monitor-articles.md`. |
| Sample config + cert one-liner | `manual.md#operator-handbook-operator-handbook`. |
| All-in-one CLI dump | `f5 query --help-manual`. |
