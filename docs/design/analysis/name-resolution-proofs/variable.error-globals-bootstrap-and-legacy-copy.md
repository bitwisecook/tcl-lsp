# naming.variable.error-globals-bootstrap-and-legacy-copy

Kind: `native-observation`

## Problem statement

A fresh errorInfo/errorCode root can be absent, present-but-undefined, or backed by an implicit private-object copy. Treating it as an ordinary quiet global read can change contents or suppress a recreation callback. The private legacy-copy bit is explicitly controlled; C8.4 has no inspected private-field counterpart.

## Question

What do original error-variable construction, controlled legacy-copy reads, unsets, missing private values and reset do to the physical global cells?

## Conclusion

C8.4 has ordinary lazily created error globals in this probe. C8.5–9.1 start with undefined allocated error cells; the controlled legacy-copy flag selects private-object copying on reads, and unsets retain/recreate an undefined root that can later copy again. The exact rows include null private values and reset. This is a special error-variable purpose, not a general quiet namespace read or a Jim/iRules policy.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Version: tcl8.4 8.4.20.

Observed capture for 8.4.20:
constructor	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
fresh-read	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
clear-flag-read	code=0	read="475545535420494e464f"	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
set-flag-read	code=0	read="475545535420494e464f"	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
unset-defined	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
read-after-unset	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
unset-code-defined	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
unset-code-undefined	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
null-private-read	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
reset-legacy	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null

### tcl8.5

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Version: tcl8.5 8.5.19.

Observed capture for 8.5.19:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### tcl8.6

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Version: tcl8.6 8.6.18.

Observed capture for 8.6.18:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode="54434c2052454144205641524e414d45"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### tcl9.0

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Version: tcl9.0 9.0.4.

Observed capture for 9.0.4:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode="54434c2052454144205641524e414d45"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### tcl9.1

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Version: tcl9.1 9.1.0.

Observed capture for 9.1.0:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode="54434c2052454144205641524e414d45"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `hidden_error_reads_and_unsets_match_original_native_objects`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::hidden_error_reads_and_unsets_match_original_native_objects` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_error_variables/manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/manifest.json)
- [rust/tcl-registry/tests/data/native_error_variables/probe.c](../../../../rust/tcl-registry/tests/data/native_error_variables/probe.c)
- [rust/tcl-registry/tests/data/native_error_variables/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.4.20.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0.tsv)
