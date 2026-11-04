# naming.variable-argv.info-default-output-name

Kind: `native-observation`

## Problem statement

The output variable of info default is distinct from the procedure and formal-name inputs. Jim's absent default subcommand could be mistaken for a successful default-value store if only a shared command name were considered.

## Question

What happens to each original output-name object in direct info default sample x NAME for the DEFAULT formal?

## Conclusion

All C releases return1 and publish parsedVarName on the original output name. C8.4 retains one reference; later C uses two for simple names and one for qualified/indexed names. Jim rejects info default as an unknown subcommand, retaining type none and two references. Resident original-name bytes remain unchanged; no Jim default-value store is observed.

## Scope

Five independently counted original names per provider: v (76), ::N::v (3a3a4e3a3a76), arr(k::part) (617272286b3a3a7061727429), FF-zero-x (ff0078) and decomposed e-acute (65cc81). Each case creates a fresh interpreter and externally retains the original name. The ASCII bootstrap creates N and sample without Tcl_Init; Jim registers core commands and static extensions. Direct C Tcl_EvalObjv uses TCL_EVAL_DIRECT; Jim uses Jim_EvalObjVector. Header snapshots read the original primary type, references and resident string bytes without calling its string getter. The after snapshot precedes the result string getter. Prestored lset/append/lappend/no-element-lappend snapshots occur after the general original-name setter. No trace, frame relocation, source literal zero, arbitrary Normal/cell lifetime, options output or BIG-IP capture is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=info/0
before|type=none|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=76
result=31
case=info/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=31
case=info/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=31
case=info/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=ff0078
result=31
case=info/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=65cc81
result=31
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=info/0
before|type=none|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=31
case=info/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=31
case=info/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=31
case=info/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=31
case=info/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=31
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=info/0
before|type=none|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=31
case=info/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=31
case=info/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=31
case=info/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=31
case=info/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=31
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=info/0
before|type=none|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=31
case=info/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=31
case=info/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=31
case=info/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=31
case=info/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=31
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: C Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=info/0
before|type=none|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=31
case=info/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=31
case=info/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=31
case=info/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=31
case=info/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=31
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Recorded compiler, linked library, compiled observer; full consumed-header hashes and successful compiler/dependency/launch receipts are retained. Provider configure flags are not recorded here; Jim's full revision and UTF build configuration are not queried by this observer.. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source.. Dialect: Jim Tcl.

Exact original observer windows, name ordinals0 through4:

```text
case=info/0
before|type=none|refs=1|bytes=1|resident=76
code=1
after|type=none|refs=2|bytes=1|resident=76
result=696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
case=info/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=1
after|type=none|refs=2|bytes=1|resident=3a3a4e3a3a76
result=696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
case=info/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=1
after|type=none|refs=2|bytes=1|resident=617272286b3a3a7061727429
result=696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
case=info/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=1
after|type=none|refs=2|bytes=1|resident=ff0078
result=696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
case=info/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=1
after|type=none|refs=2|bytes=1|resident=65cc81
result=696e666f2c20756e6b6e6f776e20636f6d6d616e64202264656661756c74223a2073686f756c6420626520616c6961732c20616c69617365732c20617267732c20626f64792c206368616e6e656c732c20636f6d6d616e64732c20636f6d706c6574652c206578697374732c206672616d652c20676c6f62616c732c2068656c702c20686f73746e616d652c206c6576656c2c206c6f63616c732c206e616d656f6665786563757461626c652c2070617463686c6576656c2c2070726f63732c207265666572656e6365732c2072657475726e636f6465732c207363726970742c20736f757263652c20737461636b74726163652c20737461746963732c207461696e7465642c2075736167652c20766172732c2076657273696f6e
```
The observer process exits0 with empty stderr. The actual guest form is rejected in all five windows; those headers supply no successful store outcome.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance/native-object-header capture for these direct argv windows is attached; C Tcl and Jim results do not establish BIG-IP event or object storage behaviour.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/probe.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/probe.c). Exact original counted-name construction, command argv and before/after header observer; this input harness is not a native implementation source excerpt.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). Original compiler binary/version/target, consumed headers, linked library and compiled probe digests, exact argument vectors, statuses and separate stream hashes.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/0`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stdout). Lines 102–126. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.4.20.stderr). Exact separate native observer stderr.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/1`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stdout). Lines 102–126. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.5.19.stderr). Exact separate native observer stderr.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/2`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stdout). Lines 102–126. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-8.6.18.stderr). Exact separate native observer stderr.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/3`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stdout). Lines 102–126. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.0.4.stderr). Exact separate native observer stderr.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/4`. Exactly this launched provider association and complete build/launch closure.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stdout). Lines 102–126. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-9.1.0.stderr). Exact separate native observer stderr.
- `provider-jim` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-manifest.json). JSON pointer `/engines/5`. Exactly this launched provider association and complete build/launch closure.
- `rows-jim` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stdout). Lines 102–126. Five actual original-name before/code/after/result windows, in fixed ordinal order.
- `stderr-jim` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stderr](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/header-jim.stderr). Exact separate native observer stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

No maintained native replay runner is attached to this physical header question. The exact retained probe and original per-provider compiler/dependency/execution argument vectors are attached. A reconfirmation must select the recorded provider/header/library policy, preserve the original -std=c11 and JIM_PROBE branch, compile the unchanged probe, record actual compiler/runtime/build hashes, and compare the complete separately captured stdout/stderr and process status. Absolute original output paths and absent compiled executables are provenance, not a callable current runner. The permanent input audit verifies existing byte/hash/case closure only; it does not launch a provider or execute Rust. No Rust parity assertion is claimed for these original header windows.
