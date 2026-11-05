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

Status: `observed`. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source. Version: tcl8.4 8.4.20.

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

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

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

Status: `observed`. Channel: Counted original string objects passed through public direct original-object argv evaluation; private primary/reference/resident fields are sampled. Bootstrap only uses NUL-terminated ASCII source. Version: jim 0.84-9-g5bac7c9.

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

Status: `not-tested`.

No appliance/native-object-header capture for these direct argv windows is attached; C Tcl and Jim results do not establish BIG-IP event or object storage behaviour.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_generic_variable_consumers](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers)
