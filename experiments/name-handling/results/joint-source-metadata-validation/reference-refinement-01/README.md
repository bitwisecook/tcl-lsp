# Typed reference-boundary refinements

The selected source-reference projection now preserves the shared
InvocationWord::proves_non_option premise for DynamicNonOption instead of
turning that exact ordinary item into unknown option selection. The global
option scanner is unchanged. The iRules consumer treats a selected typed
configuration layout as authoritative even when its projection declines;
legacy complete-literal fallback cannot bypass its guards. The public legacy
resolver/search behavior and golden fixtures remain unchanged.

One selected offline -j2 Registry/iRules library build passed in 120.67 seconds
on the explicit root-leased tmp/name-target-issues cache. Four exact controls
ran once each on its same two pinned current binaries:

| Exact control | Passed | Failed | Filtered |
| --- | --- | --- | --- |
| `source_configuration_reference::tests::selected_configuration_layout_keeps_unknown_item_and_original_ordinal` | 1 | 0 | 1758 |
| `source_configuration_reference::tests::selected_configuration_layout_declines_unknown_control_and_cardinality` | 1 | 0 | 1758 |
| `source_object::tests::original_data_group_operands_keep_unknown_item_and_refuse_unknown_reference` | 1 | 0 | 71 |
| `source_object::tests::typed_configuration_decline_does_not_reenter_literal_compatibility` | 1 | 0 | 71 |

The unchanged Registry positive expectations now pass through the non-option
case and six-operator loop. The new iRules adverse control rejects literal
extra/missing tails, unknown option/operator and NUL reference; it also asserts
that the existing public literal compatibility resolver still returns its
original permissive extra-tail ordinal. Existing fixtures/assertions were not
weakened. No new source changed during build or controls.

[Validation](validation.json), exact invocations, full raw stdout/stderr,
[build binaries](binaries.json), source readiness and prepared patch retain
the actual results. Registry binary SHA256 is
08b1d904eb8fb37293eb6a5bdce6f29ef2422a2962022159009e16964477307d;
iRules binary SHA256 is
cef81ecd1897af4497f7c82c4736fa306690455a811f130c76634d7a4d75b290.
The two changed-owner full images are compressed. Source hashes are
793554f0627c42c0e6c64d24cf5d279c4e862bfe12e3b912af48f78846e3924a
(Registry projection) and
93a5f64241b1db2ad19a6730912c9304c89bb198e4f8ef9b21f3c307f9225c03
(iRules object operands).

The actual two-root manifest closure contains 25 conservative local packages
and 11,764 source/header/manifest/common data/text asset hashes. All stayed
unchanged. Cargo emitted 145 artifact records, including 25 owned packages.
Target source hashes map to the frozen scope; every hashed file is not claimed
compiled. External registry inputs/arbitrary generated build outputs are not
independently frozen. Standalone Runtime is excluded. Disk minimum free was
2.62GB, with no ENOSPC or space abort. All source/cache leases are released.

The [first joint capture](../validation-01/README.md) remains sealed unchanged,
including Registry non-option-bit failure and the unrelated original braced
expression datagroup failure. The latter was not repaired or repeated here.
Compiler2/VM1 acceptance remains scoped to that exact preimage; their own
production files are unchanged, but Registry dependency bytes now differ.
No new Compiler/VM, SpecTcl/Studio, MCP, integration/full-suite control or
interpreter oracle ran. These source candidates supply no accepted argv,
Native handler, cell/frame, object or F5 worker identity. No canonical proof
or provider status is promoted wholesale.
