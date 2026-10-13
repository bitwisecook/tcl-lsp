# corpus-f5-rule-path

## Question

For identical procedure tails in isolated rules and partitions, what does call select through absolute partition/rule::proc, relative rule::proc and a bare local proc?

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

Load warn (exit 0); setup completion 0; lookup completion 0; result "hex:4c4f43414c207b6120627d207b6320647d".

Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.

app3 measures Common bare-local, relative-provider, absolute-provider, other provider and missing route/proc. Existing canonical owner-route supplies folders and cross-partition controls; these exact configurations have independent scope.

### tmm-procedure

Load warn (exit 0); setup completion 0; lookup completion 0; result "hex:4c4f43414c207b6120627d207b6320647d".

Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.

app3 measures Common bare-local, relative-provider, absolute-provider, other provider and missing route/proc. Existing canonical owner-route supplies folders and cross-partition controls; these exact configurations have independent scope.

### tmm-traffic-event

Load warn (exit 0); setup completion 0; lookup completion 0; result "hex:4c4f43414c207b6120627d207b6320647d".

Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.

app3 measures Common bare-local, relative-provider, absolute-provider, other provider and missing route/proc. Existing canonical owner-route supplies folders and cross-partition controls; these exact configurations have independent scope.

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

[corpus-f5-rule-path.json](corpus-f5-rule-path.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
