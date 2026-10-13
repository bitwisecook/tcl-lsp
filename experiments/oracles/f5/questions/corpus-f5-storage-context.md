# corpus-f5-storage-context

## Question

Which local/static::/qualified cells do rule-procedure calls read or write across caller and provider rules?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | unavailable | partial | not separately recorded |
| iapp-implementation | unavailable | partial | not separately recorded |
| tmm-RULE_INIT | unavailable | partial | not separately recorded |
| tmm-procedure | unavailable | partial | not separately recorded |
| tmm-traffic-event | unavailable | partial | not separately recorded |
| appliance-stock-tclsh | unavailable | partial | not separately recorded |
| icall | unavailable | partial | not separately recorded |
| iapp-apl | unavailable | partial | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"call\"".

Only the selected operation and invocation are covered.

Exact dynamic native call capability input reached code1 invalid command name call; no native provider-rule context is available through this surface.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"call\"".

Only the selected operation and invocation are covered.

Exact dynamic native call capability input reached code1 invalid command name call; no native provider-rule context is available through this surface.

### tmm-RULE_INIT

Load warn (exit 0); setup completion 0; lookup completion 0; result "hex:312031203a3a".

Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.

Owned cross-rule native cells return scope/level and an incrementing static counter; fresh traffic invocations share the actual worker cell. Broader namespace/regular-global/connection storage is measured by the separate persistence run, whose review is linked below.

### tmm-procedure

Load warn (exit 0); setup completion 0; lookup completion 0; result "hex:312032203a3a".

Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.

Owned cross-rule native cells return scope/level and an incrementing static counter; fresh traffic invocations share the actual worker cell. Broader namespace/regular-global/connection storage is measured by the separate persistence run, whose review is linked below.

### tmm-traffic-event

Load warn (exit 0); setup completion 0; lookup completion 0; result "hex:312033203a3a".

Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.

Owned cross-rule native cells return scope/level and an incrementing static counter; fresh traffic invocations share the actual worker cell. Broader namespace/regular-global/connection storage is measured by the separate persistence run, whose review is linked below.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 1; result "invalid command name \"call\"".

Only the selected operation and invocation are covered.

Exact dynamic native call capability input reached code1 invalid command name call; no native provider-rule context is available through this surface.

### icall

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"call\"".

Only the selected operation and invocation are covered.

Exact dynamic native call capability input reached code1 invalid command name call; no native provider-rule context is available through this surface.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"call\"".

Only the selected operation and invocation are covered.

Exact dynamic native call capability input reached code1 invalid command name call; no native provider-rule context is available through this surface.

## Configuration, scripts and decoding

[corpus-f5-storage-context.json](corpus-f5-storage-context.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
