# naming.variable.original-frame-array-trace-retirement-horizon

Kind: `native-attempt`

## Problem statement

A procedure-owned array trace probe first attempts to expose its member through a namespace variable. That setup has its own provider-specific restrictions, which must be measured before attributing any log to procedure-frame teardown.

## Question

Do the two original namespace-alias frame probes reach trace teardown, and what exact completion/result prevents an admitted retirement-horizon observation?

## Conclusion

Neither original supplies a frame array-trace retirement-horizon answer. C8.4 rejects the namespace alias with guest code1 and bad variable name "xx": upvar won't create namespace variable that refers to procedure variable. C8.5–C9.1 reject it with guest code1 and bad variable name "xx": can't create namespace variable that refers to procedure variable. Jim passes the source's preceding alias read and then rejects trace with guest code1 and invalid command name "trace". Those are the measured setup outcomes, not teardown chronology. All twelve driver processes exit0 with empty stderr; each whole counted completion/result, compile/ELF and input remains retained. The provider status stays not-tested for the proposed trace-retirement observation. These failures supply no entered callback, old-variable accessibility/mutation, private alias/cell/frame identity, Native authority or Rust assertion pass. The intended question was not reached. The setup captures cannot be used as a positive answer or as execution, teardown or implementation equivalence evidence.

## Scope

Two untouched ASCII LF originals each run independently in six fresh fully initialised interpreters under the unchanged counted-source CAPI driver. Twelve actual process receipts/streams and six compile receipts/ELFs preserve the different provider setup failures. The initial request and expanded capture request remain separate inputs. No original reaches the requested trace-teardown horizon, and no substituted alias/trace API or corrected source is merged into these inputs. The independent bad-level owner-teardown inputs and any admitted caller-access source remain distinct questions. BIG-IP, opaque/NUL/Unicode names and software assertion execution are not tested. Current linked comparisons consume complete unchanged public code/result windows without host refusal, keeping admitted namespace/caller observations separate from rejected frame setups and no executed Rust pass claim. Software namespace teardown retains its actual original parent edge while callbacks run and retires only the selected root binding; same-spelled foreign state and private receiver correspondence require their independent software owners. Public logs do not prove those private identities.

## Attempt scope

The unreached intended question is: What original array-trace callback and variable-access retirement horizon occurs during procedure-frame teardown?

The intended question was not reached. The setup captures cannot be used as a positive answer or as execution, teardown or implementation equivalence evidence.

The original probe, both original source inputs, twelve complete process receipts and twelve stdout/stderr pairs retain their existing evidence IDs and whole-file digests. The reached failure stages are:

- Namespace-alias setup fails before trace installation or procedure-frame teardown. C8.4–C9.1 each retain both independent source captures.
- Trace-command setup fails after the preceding alias read and before trace installation or procedure-frame teardown. Jim retains both independent source captures.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Two fresh fullInit original counted-source evaluations, independent VERSION and ORIGINAL hex completion/result rows. Setup is rejected; the requested trace-retirement horizon is not entered. Version: tcl8.4 8.4.20.

Frame trace-retirement chronology not tested: both exact originals measure guest code1 with setup result 'bad variable name "xx": upvar won\'t create namespace variable that refers to procedure variable'. The namespace alias is rejected before trace setup or procedure-frame teardown. Both independent host processes exit0 with empty stderr; complete raw setup observations are retained separately from the untested retirement question.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `not-tested`. Channel: Two fresh fullInit original counted-source evaluations, independent VERSION and ORIGINAL hex completion/result rows. Setup is rejected; the requested trace-retirement horizon is not entered. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Frame trace-retirement chronology not tested: both exact originals measure guest code1 with setup result 'bad variable name "xx": can\'t create namespace variable that refers to procedure variable'. The namespace alias is rejected before trace setup or procedure-frame teardown. Both independent host processes exit0 with empty stderr; complete raw setup observations are retained separately from the untested retirement question.

### jim

Status: `not-tested`. Channel: Two fresh fullInit original counted-source evaluations, independent VERSION and ORIGINAL hex completion/result rows. Setup is rejected; the requested trace-retirement horizon is not entered. Version: jim 0.84-9-g5bac7c9.

Frame trace-retirement chronology not tested: both exact originals measure guest code1 with setup result 'invalid command name "trace"'. The preceding source alias read does not supply an entered trace or teardown result. Both independent host processes exit0 with empty stderr; complete raw setup observations are retained separately from the untested retirement question.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_teardown.rs](../../../../runtime/rust/src/interp/native_variable_teardown.rs), `interp::native_variable_teardown::tests::original_frame_alias_refusals_do_not_claim_teardown_chronology` (linked): The twelve original348 alias/trace setup rejections compare independently alongside twelve347 bad-level failures; provider trace-retirement status stays not-tested, with no chronology from host exit0.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_array_frame_teardown348/8.4.20](../../../../rust/tcl-registry/tests/data/native_array_frame_teardown348/8.4.20)
- [rust/tcl-registry/tests/data/native_array_frame_teardown348/8.5.19](../../../../rust/tcl-registry/tests/data/native_array_frame_teardown348/8.5.19)
- [rust/tcl-registry/tests/data/native_array_frame_teardown348/8.6.18](../../../../rust/tcl-registry/tests/data/native_array_frame_teardown348/8.6.18)
- [rust/tcl-registry/tests/data/native_array_frame_teardown348/9.0.4](../../../../rust/tcl-registry/tests/data/native_array_frame_teardown348/9.0.4)
- [rust/tcl-registry/tests/data/native_array_frame_teardown348/9.1.0](../../../../rust/tcl-registry/tests/data/native_array_frame_teardown348/9.1.0)
- [rust/tcl-registry/tests](../../../../rust/tcl-registry/tests)
- [rust/tcl-registry/tests/data/native_array_frame_teardown348/jim](../../../../rust/tcl-registry/tests/data/native_array_frame_teardown348/jim)
