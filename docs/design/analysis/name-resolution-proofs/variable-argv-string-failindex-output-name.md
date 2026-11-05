# naming.variable-argv.string-failindex-output-name

Kind: `native-observation`

## Problem statement

A string classification failindex is a variable setter only when the selected command grammar accepts the option. Jim's rejected argument form must not inherit C's successful name-cache observation.

## Question

Does string is alpha -failindex NAME a2 parse the actual original NAME object, and what header remains if the form is rejected?

## Conclusion

All C releases return0 and publish parsedVarName on each original failindex name. C8.4 retains one reference; later C uses two for simple names and one for qualified/indexed names. Jim rejects this exact option form with its string-is usage error, leaving type none with two references. All sampled resident bytes remain unchanged; Jim rejection supplies no successful output-variable observation.

## Scope

Five independently counted original names per provider: v (76), ::N::v (3a3a4e3a3a76), arr(k::part) (617272286b3a3a7061727429), FF-zero-x (ff0078) and decomposed e-acute (65cc81). Each case creates a fresh interpreter and externally retains the original name. The ASCII bootstrap creates N and sample without Tcl_Init; Jim registers core commands and static extensions. Direct C Tcl_EvalObjv uses TCL_EVAL_DIRECT; Jim uses Jim_EvalObjVector. Header snapshots read the original primary type, references and resident string bytes without calling its string getter. The after snapshot precedes the result string getter. Prestored lset/append/lappend/no-element-lappend snapshots occur after the general original-name setter. No trace, frame relocation, source literal zero, arbitrary Normal/cell lifetime, options output or BIG-IP capture is measured.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source. Version: tcl8.4 8.4.20.

Exact original observer windows, name ordinals0 through4:

```text
case=string/0
before|type=none|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=76
result=30
case=string/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=30
case=string/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=30
case=string/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=ff0078
result=30
case=string/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=65cc81
result=30
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact original observer windows, name ordinals0 through4:

```text
case=string/0
before|type=none|refs=1|bytes=1|resident=76
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=76
result=30
case=string/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=3a3a4e3a3a76
result=30
case=string/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=0
after|type=parsedVarName|refs=1|bytes=1|resident=617272286b3a3a7061727429
result=30
case=string/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=ff0078
result=30
case=string/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=0
after|type=parsedVarName|refs=2|bytes=1|resident=65cc81
result=30
```
The observer process exits0 with empty stderr. The actual selected command completes with code0 in all five windows. References include the probe's external hold and provider-retained references at the sampled instant.

### jim

Status: `unsupported`. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source. Version: jim 0.84-9-g5bac7c9.

Exact original observer windows, name ordinals0 through4:

```text
case=string/0
before|type=none|refs=1|bytes=1|resident=76
code=1
after|type=none|refs=2|bytes=1|resident=76
result=77726f6e67202320617267733a2073686f756c642062652022737472696e6720697320636c617373203f2d7374726963743f2073747222
case=string/1
before|type=none|refs=1|bytes=1|resident=3a3a4e3a3a76
code=1
after|type=none|refs=2|bytes=1|resident=3a3a4e3a3a76
result=77726f6e67202320617267733a2073686f756c642062652022737472696e6720697320636c617373203f2d7374726963743f2073747222
case=string/2
before|type=none|refs=1|bytes=1|resident=617272286b3a3a7061727429
code=1
after|type=none|refs=2|bytes=1|resident=617272286b3a3a7061727429
result=77726f6e67202320617267733a2073686f756c642062652022737472696e6720697320636c617373203f2d7374726963743f2073747222
case=string/3
before|type=none|refs=1|bytes=1|resident=ff0078
code=1
after|type=none|refs=2|bytes=1|resident=ff0078
result=77726f6e67202320617267733a2073686f756c642062652022737472696e6720697320636c617373203f2d7374726963743f2073747222
case=string/4
before|type=none|refs=1|bytes=1|resident=65cc81
code=1
after|type=none|refs=2|bytes=1|resident=65cc81
result=77726f6e67202320617267733a2073686f756c642062652022737472696e6720697320636c617373203f2d7374726963743f2073747222
```
The observer process exits0 with empty stderr. The actual guest form is rejected in all five windows; those headers supply no successful store outcome.

### bigip

Status: `not-tested`.

No appliance/native-object-header capture for these direct argv windows is attached; C Tcl and Jim results do not establish BIG-IP event or object storage behaviour.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_generic_variable_consumers](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers)
