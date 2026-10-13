# MCP static-name inventory validation

The MCP collector previously received source variable-token labels but omitted
literal variable-role operands. `SourceInventory` now retains the same genuine
captured context and its presentation list. Static-name scaffold hints also
use the current shared source schema, valid arity, actual role/operand ordinals,
guarded original words and shared literal-byte projections. Dynamic, expanded
or missing-original values decline; hints establish no runtime cell, entered
frame, handler identity, rename target or worker reachability. The original
fixtures/assertions are byte-identical; only helper transport changed.

One selected MCP test-target build completed offline with `-j2` and `--no-run`
in 187.10 seconds. Three existing exact controls ran once each on its same
newly hashed binary:

| Exact existing control | Result |
| --- | --- |
| `irule_gen::tests::executable_inventory_ignores_comments_and_data_but_keeps_live_substitutions` | 1 passed, 0 failed |
| `irule_gen::tests::called_helper_static_write_preserves_hot_event_for_multi_tmm` | 1 passed, 0 failed |
| `irule_gen::tests::generator_builds_one_request_closure_for_every_execution_sensitive_output` | 0 passed, 1 failed |

Each invocation filtered 132 tests. The inventory control now reaches and
passes its unchanged `["hits"]` assertion and later multi-TMM assertion.
The third control reaches/passes `request_closure_builds() == 1`, then fails
at `rust/tcl-mcp/src/irule_gen.rs:1438`: actual datagroups `[]`, expected
`["helper_dg"]`. Its later command/static-scaffold assertions were not reached.
The MCP suite is not claimed green, and no source changed after this failure.

[Validation](validation.json), exact invocation JSON files and full raw
stdout/stderr retain the successful build, both passes and the original new
failure. [Control identities](control-sources.json) retain unchanged source
excerpts. The [prepared patch](irule_gen.patch) and compressed full current
`irule_gen.rs` preserve the compiled MCP change. Its SHA256 is
`db83e63cb52238c77fa41061b51dc0bc3839cf4eb673ac9587cff5f61001a931`;
`tools.rs` remains `fd114232db002cc84b1752c4ef96409b634b953206edc4a50dcba1c861e1aad2`.

The selected [binary](binary.json) SHA256 before/after all controls is
`f0b613dfaa3f2567c11cdeb6c60421bd486bfcb19ffaa7c0ba4b87ecef31c3a2`.
Its actual executable is
`/workspace/tcl-lsp/target/debug/deps/tcl_mcp-0700f7245459bc2c`.
The runner sourced `/tmp/name-env.sh` but omitted explicit
`CARGO_TARGET_DIR`; Cargo used `/workspace/tcl-lsp/target` rather than the
root-granted `/workspace/tcl-lsp/tmp/name-target-issues` cache. Root was
notified and is checking default-target exclusivity. Actual source/binary
currency and fresh artifact records are retained; cache exclusivity remains
unverified in this capture. No repeat build or control was performed.

The frozen [baseline](baseline.json) covers 39 conservative local manifest
packages and 5,307 Rust/C/header/manifest/root configuration/lock files.
The new source and current clock/Core fixes are included. Before/after
snapshots and each exact control retain zero changes during capture.
VM is included; standalone Runtime is excluded. This conservative file scope
is not a claim that every file compiled or that generated build outputs were
independently frozen. Cargo emitted 361 artifact records. Exactly 39 owned
workspace manifests appear in 43 target records, all freshly built; retained
record source hashes map to frozen inputs. Registry cache manifests under
repository `tmp/` are excluded from the corrected local-package count.
The retained runner initially used the repository path prefix for this derived
count; the corrected receipt uses the exact 39 owned manifest paths. Raw Cargo
output remains unchanged.

The earlier [five-error compile capture](../../mcp-exact-command-name/validation-01/validation.json),
[exact literal-name pass](../../mcp-exact-command-name/validation-02/README.md),
and [original three-control capture](../../mcp-exact-command-name/existing-controls-01/README.md)
remain sealed unchanged. The latter retains the original static-name failure.
No literal/event control repeated, full suite, generator codegen or interpreter
oracle ran here. No F5 TMM/MCPD test or Tcl dispatch conformance claim follows.
Sources and all leases are released; the datagroup failure remains an open
implementation obligation requiring a separately scoped owner review.
