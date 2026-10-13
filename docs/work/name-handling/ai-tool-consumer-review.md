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
cache. Root was informed; default-target exclusivity is unverified in the
sealed receipt. No repeated build was performed. Actual source/binary identity
and fresh artifact results remain retained with that provenance limitation.

All 5,307 frozen files in the 39-package conservative MCP scope stayed unchanged
through build and controls; current clock/Core fixes are included. VM is
included; standalone Runtime is excluded. Cargo emitted 361 artifact records,
including exactly 39 owned workspace packages/43 fresh target records. The
scope does not claim every frozen file compiled or independently freeze every
generated build output. Registry-cache manifests under repository `tmp/` are
excluded from the corrected owned-package count; raw Cargo output is unchanged.
No literal/event controls repeated, full suite, codegen, interpreter oracle or
F5 TMM/MCPD test ran. All source/cache leases are released.

## Remaining obligations

Check default-target exclusivity for the actual 00:33:33–00:37:02 UTC capture
before treating cache provenance as settled. Root coordinates any required
follow-up; no repeat build/test is authorised by this snapshot.

The unchanged generator datagroup assertion is now the bounded unresolved
implementation failure. Trace its actual source-reference owner and closure
before changing MCP or shared source. Preserve the original fixture/assertions
and sealed failing logs. A new source edit or build needs a separately scoped
owner/cache lease and fresh source/binary evidence.

Proposed future finite consumer comparisons remain unexecuted: dynamic
procedure declarations followed by rename; a literal callback alias before
and after an unknown writer; namespace-imported cursor locations before rename
and with an unknown intervening writer. Tie conformance claims to the retained
interpreter/native-context oracle for the exact question.
