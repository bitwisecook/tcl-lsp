# F5 wild-pattern answers: <run-id>

## Provider and exact inputs

- Branch commit containing retained inputs and measurements:
- Run identifier and UTC timestamp:
- BIG-IP version/build/hotfix/platform:
- Context, invoking event/action and execution namespace where observed:
- Caller/provider rule identities, partition/folder and attachment:
- Profiles, expected TMM roster and actual responding unit IDs:
- Original question input path/hash and actual materialised input path/hash:
- Complete wrapper/configuration/invocation paths and SHA-256:
- Raw acceptance, stdout/stderr, logs and result record paths:

## Question/context results

| Bundle question ID | Canonical question / case / variant | Context | Load | Reached operation | Runtime code and result bytes | Error options | Status / exact evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| | | | | | | | |

Keep setup and intended lookup separate. An observed caught error needs its
inner completion, not only the wrapper's successful completion. Preserve
result bytes with an explicitly identified encoding and hex/base64 form when
text would be ambiguous. Record unsupported/unreached/unavailable independently.
For a TMM procedure name its invoking event and caller; for iApp distinguish
creation from execution. APL and iCall do not borrow an iApp or CLI answer.

## Individual JSON record

Use one record for each exact input/context/variant/TMM invocation; preserve
repeats. Null means not observed. Replace placeholders with measured values.
Do not infer an error option that the context did not return.

```json
{
  "schema_version": 1,
  "question_id": "f5-pattern-computed-qualified",
  "canonical_question_id": "naming.corpus-pattern.computed-qualified",
  "case_id": "computed-qualified",
  "variant_id": "<stable exact-input variant>",
  "run_id": "<run-id>",
  "provider": {
    "version": "<version>",
    "build": "<build>",
    "hotfix": null,
    "context": "tmm-traffic-event",
    "event": "<actually invoked event>",
    "tmm_unit": null,
    "execution_namespace": null
  },
  "original_input": {"path": "<path>", "sha256": "<hash>"},
  "actual_input": {"path": "<path>", "sha256": "<hash>"},
  "wrapper": {"path": "<path>", "sha256": "<hash>"},
  "adaptation": null,
  "load": {"status": "not-tested", "transcript": null},
  "setup": {"reached": null, "completion_code": null, "result_hex": null},
  "lookup": {
    "reached": null,
    "completion_code": null,
    "result_encoding": null,
    "result_hex": null,
    "errorcode": null,
    "errorinfo": null,
    "return_options": null,
    "unavailable_fields": []
  },
  "status": "not-tested",
  "raw_evidence": [],
  "cleanup_evidence": [],
  "configuration_saved": null
}
```

`load.status` is `not-tested`, `accept`, `warn`, `reject`, `not-applicable`, or
`infrastructure-failure`. `status` uses the states in the bundle README.
`not-applicable` load means the documented input channel has no distinct load
stage; it does not mean load succeeded. Unsupported literal load and a reached
dynamic-command error require separate variants.

## Controls, interpretation and remaining questions

Record exact literal/dynamic controls, capability discovery and repeats.
State any changed question caused by an adaptation, contradictory measurements,
and remaining not-tested contexts. Link the complete input and transcript for
each conclusion. State its precise version/context/input scope. Do not infer
an implementation mechanism from closed-source runtime observations.

## Cleanup proof

Record pre-create absence, exact created object names, cleanup/trap commands and
post-delete absence. Record failures/residual objects and whether configuration
was saved; the main protocol forbids saving configuration. State any traffic
or service operation needed to reach a case.

## Coverage bindings

List canonical questions that have exact accepted answers and the observation
record pointers. List related adapted controls separately. Link promoted test
IDs where present; otherwise state `not promoted`. No implementation history
or agent diary belongs in the answer document.
