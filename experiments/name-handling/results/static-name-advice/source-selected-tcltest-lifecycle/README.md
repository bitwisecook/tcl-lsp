# Do source-selected Tcltest lifecycle changes restore genuine missing-read diagnostics?

No restoration is demonstrated by the retained W210 selection run:
[the Tcltest log](logs/w210-selection-tcltest.log) reports six passes and three
failures. The genuine missing-read checks still receive no diagnostics:

| Exact failing test under `analyser::diagnostics::tests::` | Observed versus expected |
| --- | --- |
| `tcltest_aliases_keep_genuine_undefined_reads_and_result_data_opaque` | 0 versus 1 |
| `tcltest_shared_frame_keeps_genuine_undefined_reads_and_data_opaque` | 0 versus 1 |
| `tcltest_oracle_program_keeps_missing_reads_and_setup_completion` | 0 versus 2 |

The successful Tcltest controls retain the reported setup/body/cleanup reads,
namespace-scoped import handling, structural body indices and the runtime
barrier. Those successes do not prove that genuine absent-variable reads are
reported or that result data remains opaque on a fully passing diagnostic run.

| Other retained control or observation | Captured result |
| --- | --- |
| [Lifecycle controls](logs/w210-selection-lifecycle.log) | 5 passed; 0 failed |
| [Absent-dispatch selection carrier](logs/w210-selection-carrier.log) | 1 passed; 0 failed |
| [Original package source point](logs/w210-selection-package-point.log) | 1 passed; 0 failed |
| [Original source-purpose controls](logs/w210-selection-purpose.log) | 3 passed; 3 failed |
| [Temporary selection instrumentation](logs/w210-debug-selection.log) | Entry/config/word checks true, selected command `None`; 0 passed; 1 failed |
| [Existing selection trace](logs/w210-selection-existing-trace.log) | 0 passed; 1 failed |
| [Earlier compilation attempt](logs/w210-selection-compile-error.log) | E0506 borrow error; no completed test result |

Temporary trace observations explain what those instrumented invocations saw;
they do not establish that the same instrumentation or source still exists.
The compilation error belongs to a separate retained attempt and is not the
result of the later nine-test run.

The original [selection-before](../provenance/w210-source-inventories/w210-selection-before.json)
and [selection-after](../provenance/w210-source-inventories/w210-selection-after.json)
receipts contain the same SHA-256 values for three source owners. They provide
a bounded before/after identity check, not complete compiler-tree or binary
provenance. The separately retained
[frozen-before inventory](../provenance/w210-source-inventories/w210-frozen-source-before.json)
has 568 compiler source hashes and different values for all three of those
owners. It belongs to another stage and must not be presented as the full
source inventory for the selection run. There are no matching retained source
bytes or complete original invocation/build hashes for that selection stage;
the current working tree is not used to fabricate them.

See [the shared provenance index](../provenance/README.md),
[retention records](../records.json) and [integrity checks](../integrity.json).
