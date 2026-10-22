# CI 5205 shard 3 provisional phase routing

This is a routing aid for the exact `(binary_id, selector)` baseline-PASS/current-FAIL rows in the archived shard-3 comparison. It does not explain failures, claim a release result, or resolve the 20,988 identities not emitted by this shard. The authoritative raw evidence remains `ci5205-shard3-exact-selector-join-20261022T2307Z/shard3-raw.log`, `current-outcomes.json`, `comparison.json.gz`, and `matched-baseline-pass-current-fail.json` in the coordinator review archive.

| Current failure family | Count | Proposed owner | Evidence for routing only |
|---|---:|---|---|
| `bpf-tcl` | 21 | Compiler/source-selection | Failures include `bpf-tcl::deploy::attach_bad_arity_rejected` and the deploy attach family, plus BPF source diagnostics/templates/maps/ELF tests. Route to source admission / generated BPF compile input analysis; do not infer all share one cause. |
| `cshim` package rows | 13 | VM/runtime | These selectors exercise the C shim/runtime boundary (command/load/return/object behavior). Preserve per-selector triage; package name is not a cause. |
| `tcl-registry` | 3 | Compiler/registry | Exact selectors: `registry_commands::an_explicitly_empty_operation_table_does_not_inherit_the_parents`, `registry_commands::namespace_subcommands_declare_their_namespace_name_arguments`, `registry_commands::source_nopkg_is_version_gated`. |
| `compiler-explorer` | 21 | Compiler-codegen / explorer projection | Failures include output/rendering/serialization/coverage projection selectors; route to the owning report/projection path after examining each test body. Not a blanket compiler attribution. |
| CLI `dis` | 1 | Compiler | Dialect/diagnostic selector; exact identity is in `matched-baseline-pass-current-fail.json`. |
| CLI `compwasm` | 1 | Compiler-codegen | WASM compilation selector; exact identity is in the same P2F artifact. |

These are proposed next owners only. The 22 current nonpass rows without a baseline PASS remain outside this P2F table, and missing or skipped selectors remain unknown. Before attributing a shared cause, the receiving owner should inspect the exact test body and failure block in the raw shard log and preserve the baseline/current identity pair.
