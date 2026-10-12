# Original source provenance

These original receipts and snapshot bytes are retained once for the two
[questions](../README.md). Absolute temporary paths inside the JSON files
are preserved exactly; the following links locate their retained equivalents.

[Source-advice manifest](source-advice-counterfactual/manifest.json),
[comparison](source-advice-counterfactual/comparison.json), and
[restoration](source-advice-counterfactual/restoration.json) describe the
bounded owner-file counterfactual over other work in progress.

| Original owner path under `rust/tcl-compiler/src/command_binding/` | Captured WIP bytes | Captured original `head` bytes |
| --- | --- | --- |
| `source_transition_advice.rs` | [0.current](source-advice-counterfactual/0.current) | [0.head](source-advice-counterfactual/0.head) |
| `source_transition_advice/body_effects.rs` | [1.current](source-advice-counterfactual/1.current) | [1.head](source-advice-counterfactual/1.head) |
| `source_transition_advice/logical_namespace.rs` | [2.current](source-advice-counterfactual/2.current) | None; original manifest records `head_sha256: null` |
| `original_command_table.rs` | [3.current](source-advice-counterfactual/3.current) | [3.head](source-advice-counterfactual/3.head) |
| `original_advice_slot_tests.rs` | [4.current](source-advice-counterfactual/4.current) | [4.head](source-advice-counterfactual/4.head) |

The original [other-work-in-progress patch](source-advice-counterfactual/other-wip.diff)
and [baseline patch](source-advice-counterfactual/baseline-other-wip.diff) are
contextual provenance. They do not turn the partial counterfactual into a clean
baseline. The manifest's other-WIP hashes are original receipts; this archive
does not contain those other source bytes or retrospectively recheck restoration.

The W210 [selection-before](w210-source-inventories/w210-selection-before.json)
and [selection-after](w210-source-inventories/w210-selection-after.json)
inventories agree for their three owners. The separate
[frozen-before inventory](w210-source-inventories/w210-frozen-source-before.json)
records 568 compiler source hashes from an earlier stage and differs for all
three selection owners. No matching selection-stage source snapshots or full
original build hashes are available in these receipts. No present-day source
bytes are labelled as originals.

[records.json](../records.json) records original paths, byte lengths and digests
for every capture. [integrity.json](../integrity.json) checks the retained
source-advice bytes against the original manifest, the two original owner-log
digests and exact failure names against the comparison receipt, and the W210
inventory stage distinction.
