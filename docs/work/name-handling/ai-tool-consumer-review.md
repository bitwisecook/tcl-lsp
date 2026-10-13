# Bounded MCP and AI consumer review

Temporary coordination snapshot; remove before merge. This bounded review on
`work` covers MCP `tools.rs` and `irule_gen.rs`, Core `procedure_symbol.rs`,
and AI `gen_ai.rs` with their immediate contracts/templates/context seam.
It does not establish every AI/tool consumer or a green MCP suite.

## Current implementation

- `command_info` forwards the literal incoming command name to the shared
  context query; independent BIG-IP version trimming remains unchanged.
- Procedure documentation uses the shared original declaration inventory,
  transports original spelling separately from display labels, and distinguishes
  zero/one/multiple declarations. It supplies no live target or alias resolution.
- Graph tools retain the analysis's actual input/context; cursor tools adapt
  Core provider results for that source/analysis. Transport review does not
  establish provider completeness.
- Event readers receive the actual retained analysis. Conditional writer hints
  use the same source context/schema, actual arity/roles and original operands.
- The static-name repair keeps one genuine captured source context plus its
  presentation list in MCP-owned `SourceInventory`. The collector preserves
  variable-token hints and adds valid literal variable-role operands selected
  through shared schema, actual post-head ordinals, original-word guards and
  literal-byte projections. Dynamic/expanded/missing-original operands and
  invalid arity decline; context/currentness guards stay with the shared owner.
  Presentation labels select no cell, frame, Native handler, rename target or
  worker. Unknown advice proves no absence of writes.
- Generated AI catalogue prose uses the shared registry/context seam and
  describes modelled commands. Generator drift/generated copies remain
  unverified in this bounded task.

`tools.rs` SHA256 remains
`fd114232db002cc84b1752c4ef96409b634b953206edc4a50dcba1c861e1aad2`.
Current `irule_gen.rs` SHA256 is
`db83e63cb52238c77fa41061b51dc0bc3839cf4eb673ac9587cff5f61001a931`;
its earlier sealed captures use `ab48a470...`. All original test bodies,
fixtures and assertions are byte-identical; only the `executable` transport
helper changed. Direct Rust formatting passed.

## Observed validation

Earlier sealed evidence remains unchanged:
[first five-error compile capture](../../../experiments/name-handling/results/mcp-exact-command-name/validation-01/validation.json),
[exact literal-name pass](../../../experiments/name-handling/results/mcp-exact-command-name/validation-02/README.md),
and [three original controls](../../../experiments/name-handling/results/mcp-exact-command-name/existing-controls-01/README.md).
The exact-name control passed its real key and seven adverse values. The
original three controls passed helper/event reports and failed static inventory
with `[]` against `["hits"]`; the later multi-TMM assertion was not reached.

The current MCP-only repair compiled once offline with `-j2`, selected binary
`--no-run`, in 187.10 seconds. Three unchanged exact controls then ran once each
on that same new executable:

| Existing exact control | Observed result |
| --- | --- |
| `executable_inventory_ignores_comments_and_data_but_keeps_live_substitutions` | 1 passed, 0 failed |
| `called_helper_static_write_preserves_hot_event_for_multi_tmm` | 1 passed, 0 failed |
| `generator_builds_one_request_closure_for_every_execution_sensitive_output` | 0 passed, 1 failed |

Each filtered 132 tests. Inventory now passes both the original `["hits"]`
assertion and later multi-TMM assertion. The generator control passes its first
`request_closure_builds() == 1` assertion, then fails at
`rust/tcl-mcp/src/irule_gen.rs:1438`: datagroups `[]`, expected `["helper_dg"]`.
Later static-scaffold assertions remain unreached. No source changed or test
repeated after the failure. Full raw outputs, unchanged controls, exact
invocations, prepared patch, current source, source/artifact provenance and
classification are in the
[current receipts](../../../experiments/name-handling/results/mcp-static-name-inventory/validation-01/README.md).

The actual selected executable is
`/workspace/tcl-lsp/target/debug/deps/tcl_mcp-0700f7245459bc2c`, SHA256
`f0b613dfaa3f2567c11cdeb6c60421bd486bfcb19ffaa7c0ba4b87ecef31c3a2`.
The runner sourced `/tmp/name-env.sh` but omitted explicit `CARGO_TARGET_DIR`,
so Cargo used the default repository target rather than the root-granted issues
cache. Root confirmed coordinated-team default-target exclusivity during the build:
root ran no default-target build and Core/Registry/Runtime builds used explicit
runtime targets under separate leases. External host processes are not
independently attested. The
[scoped amendment](../../../experiments/name-handling/results/mcp-static-name-inventory/validation-01-cache-provenance/README.md)
records that attestation while preserving the original capture/seal unchanged.
No repeated build was performed; actual source/binary and fresh artifacts remain
retained.

All 5,307 frozen files in the 39-package conservative MCP scope stayed unchanged
through build and controls; current clock/Core fixes are included. VM is
included; standalone Runtime is excluded. Cargo emitted 361 artifact records,
including exactly 39 owned workspace packages/43 fresh target records. The
scope does not claim every frozen file compiled or independently freeze every
generated build output. Registry-cache manifests under repository `tmp/` are
excluded from the corrected owned-package count; raw Cargo output is unchanged.
No literal/event controls repeated, full suite, codegen, interpreter oracle or
F5 TMM/MCPD test ran. All source/cache leases are released.

## Current typed-reference validation

Registry owns the optional typed `SubCommand.source_configuration_reference`
DataGroupComparison layout and six comparison operators. Genuine original
roles/ordinals, known option topology, exact cardinality and literal reference
geometry guard the projection; Dynamic ITEM stays conditional. iRules maps
the typed configuration-object family and suppresses legacy fallback whenever
the selected descriptor owns the layout, including when projection declines.
The shared `proves_non_option` premise preserves DynamicNonOption distinction.
Public literal compatibility/search behavior remains unchanged. Full authored
SpecTcl/Studio declaration, loader, renderer and form parity is implemented;
no GAPS or Native object/handler identity follows from this metadata.

The [joint six-library capture](../../../experiments/name-handling/results/joint-source-metadata-validation/validation-01/README.md)
build passed; fifteen exact controls ran once with 13 passes/two failures.
Metadata10/2 included the original Registry DynamicNonOption bit failure and
unchanged braced-expression datagroup failure. Compiler source-selector2 and
corrected VM SamePosition1 passed on that exact dependency preimage. Its26
packages/11,803 source-data hashes and six binary hashes stayed unchanged.
Both failures and full raw logs remain sealed.

The [two-file refinements](../../../experiments/name-handling/results/joint-source-metadata-validation/reference-refinement-01/README.md)
build passed all four selected Registry/iRules controls once, preserving old
expectations/fixtures. Finite literal unknown-option/operator/cardinality/NUL
controls passed and public legacy compatibility remains tested. Current
Registry projection SHA-256793554f0...924a and iRules object operand
SHA-25693a5f642...5c03 were unchanged through the25-package/11,764-hash scope
and both pinned binaries. Compiler/VM acceptance retains its earlier exact
dependency preimage. Studio schema/reference documentation integration2 and
field-document regeneration passed in the separate
[Studio capture](../../../experiments/name-handling/results/mcp-static-name-inventory/structured-reference-studio-doc-01/README.md).

The [fresh current MCP generator capture](../../../experiments/name-handling/results/mcp-static-name-inventory/current-generator-validation-01/README.md)
selected the actual tcl-mcp binary on explicit default target and passed the
unchanged generator control once:1passed/132filtered. The original single
request-closure, helper_dg, multi-TMM, live/static and inert exclusion assertions
all reached. Its39-package/12,059-hash source scope and binary
SHA-2562e99d5ba...32ebd stayed unchanged. Prior literal, inventory, helper and
event controls retain their own source-pinned receipts; none repeated here.
No full-suite or interpreter/F5 provider result follows. Completed leases are
released; root owns subsequent source/cache coordination.

## Remaining obligations

The unchanged braced-expression failure remains independently open and was
not rerun after the boundary refinements. Read-only inspection narrows a
candidate loss to authentic original-vector retention, which visits Registry
Syntax bodies and lexical executable command parts without asking the existing
`source_expression_script_bodies` issuer. Final[] does not establish the first
rejected guard. A bounded temporary Compiler/SourceContext logging probe is
being prepared in scratch to observe original body origin, Expr-issued child
geometry and source-vector admission. No third production repair or probe
build is approved. Preserve the original fixture/assertions and every sealed
failure; apply a probe only under a new root-coordinated source/cache lease.

Future finite consumer comparisons remain unexecuted: dynamic procedure
requests followed by rename; literal callback alias before/after unknown
writer; namespace-imported cursor location before rename and unknown writer.
Tie any conformance claim to the retained interpreter/native-context oracle
for the exact question. Canonical proof/provider records are unchanged.
