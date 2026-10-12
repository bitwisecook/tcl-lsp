# Original OO root namespace creation

Does actual VM bootstrap create the private namespace owned by the original C9.1 stock OO receiver before publishing its command, and retain that exact token through helper installation and object teardown?

The completed [capture-01](../vm-oo-name-target-predicate/capture-01/results.md) ran the unchanged original constructor control. Its first false predicate was `current.namespace_owner`: token 25 had an arena path, live world and identical intern entry, but `declared=false`, `dying=false`, `deferred_owner=false`, `retained=false`. The complete neutral cause remains `ValueAccessRefusal(CommandProtocolUnavailable("original OO Name target/facet"))`. Later assertions were unreached.

This source-only follow-up inspected exactly three Rust owners: `cmd_oo.rs`, `cmd_oo/native_context.rs`, `interp.rs`. Matching C9.1 bootstrap source and retained oracle provenance are evidence, not additional Rust owners. No Rust change, build, fresh oracle execution, assertion change or namespace publication was performed. [source-and-proposal.md](source-and-proposal.md) describes the concrete allocation repair and acceptance boundary; [source-hashes.sha256](source-hashes.sha256) records this inspection's current source.
