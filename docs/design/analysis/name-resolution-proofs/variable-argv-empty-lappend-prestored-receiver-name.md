# naming.variable-argv.empty-lappend-prestored-receiver-name

Kind: `native-observation`

## Problem statement

A no-element lappend might normalise a stored list or perform a variable read despite adding no elements. The unchanged display cannot establish callback silence, so the original header and exact returned spacing must be retained separately.

## Question

Does direct no-element lappend NAME retain the original name header and exact A-double-space-B result after that value was prestored?

## Conclusion

All six providers return the exact bytes A  B with two spaces and retain the sampled prestored name header and resident bytes. C uses parsedVarName; Jim uses variable or indexed dict-substitution. Reference grouping is one throughout C8.4, and two for simple names versus one for qualified/indexed names in later C and Jim. This untraced probe does not establish observer silence or arbitrary read completion.

## Scope

Five independently counted original names per provider: v (76), ::N::v (3a3a4e3a3a76), arr(k::part) (617272286b3a3a7061727429), FF-zero-x (ff0078) and decomposed e-acute (65cc81). Each case creates a fresh interpreter and externally retains the original name. The ASCII bootstrap creates N and sample without Tcl_Init; Jim registers core commands and static extensions. Direct C Tcl_EvalObjv uses TCL_EVAL_DIRECT; Jim uses Jim_EvalObjVector. Header snapshots read the original primary type, references and resident string bytes without calling its string getter. The after snapshot precedes the result string getter. Prestored lset/append/lappend/no-element-lappend snapshots occur after the general original-name setter. No trace, frame relocation, source literal zero, arbitrary Normal/cell lifetime, options output or BIG-IP capture is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=empty_lappend/0
before|type=parsedVarName|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=76
result=41202042
case=empty_lappend/1
before|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=41202042
case=empty_lappend/2
before|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=41202042
case=empty_lappend/3
before|type=parsedVarName|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=ff0078
result=41202042
case=empty_lappend/4
before|type=parsedVarName|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=65cc81
result=41202042
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=empty_lappend/0
before|type=parsedVarName|refs=2|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=41202042
case=empty_lappend/1
before|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=41202042
case=empty_lappend/2
before|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=41202042
case=empty_lappend/3
before|type=parsedVarName|refs=2|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=41202042
case=empty_lappend/4
before|type=parsedVarName|refs=2|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=41202042
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=empty_lappend/0
before|type=parsedVarName|refs=2|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=41202042
case=empty_lappend/1
before|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=41202042
case=empty_lappend/2
before|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=41202042
case=empty_lappend/3
before|type=parsedVarName|refs=2|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=41202042
case=empty_lappend/4
before|type=parsedVarName|refs=2|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=41202042
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=empty_lappend/0
before|type=parsedVarName|refs=2|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=41202042
case=empty_lappend/1
before|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=41202042
case=empty_lappend/2
before|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=41202042
case=empty_lappend/3
before|type=parsedVarName|refs=2|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=41202042
case=empty_lappend/4
before|type=parsedVarName|refs=2|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=41202042
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=empty_lappend/0
before|type=parsedVarName|refs=2|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=41202042
case=empty_lappend/1
before|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=41202042
case=empty_lappend/2
before|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=41202042
case=empty_lappend/3
before|type=parsedVarName|refs=2|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=41202042
case=empty_lappend/4
before|type=parsedVarName|refs=2|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=41202042
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: Jim Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=empty_lappend/0
before|type=variable|refs=2|bytes=1|resident=76
code=0
after|type=variable|refs=2|bytes=1|resident=76
result=41202042
case=empty_lappend/1
before|type=variable|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=variable|refs=1|bytes=1|resident=3a3a4e3a3a76
result=41202042
case=empty_lappend/2
before|type=dict-substitution|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=dict-substitution|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=41202042
case=empty_lappend/3
before|type=variable|refs=2|bytes=1|resident=ff0078
code=0
after|type=variable|refs=2|bytes=1|resident=ff0078
result=41202042
case=empty_lappend/4
before|type=variable|refs=2|bytes=1|resident=65cc81
code=0
after|type=variable|refs=2|bytes=1|resident=65cc81
result=41202042
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance/native-object-header capture for these direct argv windows is attached; C Tcl and Jim results do not establish BIG-IP event or object storage behaviour.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/probe.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/probe.c). Exact original counted-name construction, command argv and before/after header observer; this input harness is not a native implementation source excerpt.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). Original compiler binary/version/target, consumed headers, linked library and compiled probe digests, exact argument vectors, statuses and separate stream hashes.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/0`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stdout). Lines 202–226. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stderr). Exact separate native observer stderr.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/1`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stdout). Lines 202–226. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stderr). Exact separate native observer stderr.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/2`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stdout). Lines 202–226. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stderr). Exact separate native observer stderr.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/3`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stdout). Lines 202–226. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stderr). Exact separate native observer stderr.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/4`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stdout). Lines 202–226. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stderr). Exact separate native observer stderr.
- `provider-jim` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/5`. Exactly this launched provider association and complete build/launch closure.
- `rows-jim` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stdout). Lines 202–226. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-jim` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stderr). Exact separate native observer stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

No maintained native replay runner is attached to this physical header question. The exact retained probe and original per-provider compiler/dependency/execution argument vectors are attached. A reconfirmation must select the recorded provider/header/library policy, preserve the original -std=c11 and JIM_PROBE branch, compile the unchanged probe, record actual compiler/runtime/build hashes, and compare the complete separately captured stdout/stderr and process status. Absolute original output paths and absent compiled executables are provenance, not a callable current runner. The permanent input audit verifies existing byte/hash/case closure only; it does not launch a provider or execute Rust. No Rust parity assertion is claimed for these original header windows.
