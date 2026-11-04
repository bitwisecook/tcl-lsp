# Tcl execution evidence

## Question

> Does guest error text that looks like a wrong-arguments diagnostic select its error-code classification, or does classification belong to the original error producer and explicit metadata? Distinguish an arbitrary error message, genuine set wrongargs and explicit supplied error code independently in every original provider.

> Does wrong-arguments-looking error text establish public errorCode metadata, and how do actual set arity errors and explicit error-code arguments differ across the pinned original C releases and Jim?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> The immutable original ASCII LF source performs three caught public script operations in one fresh process per provider, setting a distinct seed before each. Count-preserving binary-scan hex retains exact result and public errorCode bytes; both availability and returned value are measured. Seeds are overwritten in every captured case. The fixed source does not contain raw NUL, non-ASCII, binary-produced keys, private headers or direct object-vector entry. Fresh info patchlevel queries are retained independently of prior version receipts reused under exact original CLI/source/header/configuration/static archive pins. No new compile is run; selected original CLI executable identities, launch argv, explicit TCL_LIBRARY overlay and prelaunch 162 required pins are retained. Inherited environment, compiler version and unrelated system library identities are unrecorded. Nineteen exact original Tcl_ErrorObjCmd/Tcl_SetObjCmd/Tcl_WrongNumArgs and Jim error/set/wrongargs/registration windows explain producers separately from observed output. The unchanged preparation request includes a conditional unsupported/untouched Jim errorCode caution; the actual captured Jim query instead succeeds and reports NONE. Only Jim third-argument explicit-code acceptance is purpose-unavailable; it is never used to label the supported public query N/A. These three cases do not certify other commands, release points, arbitrary catch options or message parsing. Software API controls and typed Guest/Host settlement require independent receipts; all prior native records remain unchanged. BIG-IP is not tested. The embedding control has no executed outcome attached. Original Native430 provider answers, public query availability, explicit-code acceptance/refusal, exact source/streams and evidence pins remain unchanged. Software transport does not confer a generic diagnostic class, Native object/header/frame or successful handler. The linked assertion definition has no executed result attached by this binding. Its NativeOrigin and String-producer assertions are software selection contracts, not original public header observations. Original Native430 provider answers, explicit-code acceptance/refusal, evidence/source windows and replay remain unchanged.

## Script

```tcl-visible
# Counted ASCII source: double quotes inside braced error text are literal.
proc emit_metadata {case code result} {
    binary scan $result H* result_hex
    set available [info exists ::errorCode]
    set code_hex {}
    if {$available} {
        binary scan $::errorCode H* code_hex
    }
    puts [list CASE $case code $code result_hex $result_hex errorCode_available $available errorCode_hex $code_hex]
}
set version_code [catch {info patchlevel} version_result]
binary scan $version_result H* version_hex
puts [list VERSION_QUERY code $version_code result_hex $version_hex]
set ::errorCode SEEDED_ARBITRARY
set code [catch {error {wrong # args: should be "synthetic value"}} result]
emit_metadata arbitrary-message $code $result
set ::errorCode SEEDED_GENUINE
set code [catch {set} result]
emit_metadata genuine-set-wrongargs $code $result
set ::errorCode SEEDED_EXPLICIT
set code [catch {error {wrong # args: should be "synthetic value"} {} {USER CODE}} result]
emit_metadata explicit-error-code $code $result
puts CLOSED_CASES_3
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION_QUERY code 0 result_hex 382e342e3230
CASE arbitrary-message code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 4e4f4e45
CASE genuine-set-wrongargs code 1 result_hex 77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f22 errorCode_available 1 errorCode_hex 4e4f4e45
CASE explicit-error-code code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 5553455220434f4445
CLOSED_CASES_3
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION_QUERY code 0 result_hex 382e352e3139
CASE arbitrary-message code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 4e4f4e45
CASE genuine-set-wrongargs code 1 result_hex 77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f22 errorCode_available 1 errorCode_hex 4e4f4e45
CASE explicit-error-code code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 5553455220434f4445
CLOSED_CASES_3
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION_QUERY code 0 result_hex 382e362e3138
CASE arbitrary-message code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 4e4f4e45
CASE genuine-set-wrongargs code 1 result_hex 77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f22 errorCode_available 1 errorCode_hex 54434c2057524f4e4741524753
CASE explicit-error-code code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 5553455220434f4445
CLOSED_CASES_3
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION_QUERY code 0 result_hex 392e302e34
CASE arbitrary-message code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 4e4f4e45
CASE genuine-set-wrongargs code 1 result_hex 77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f22 errorCode_available 1 errorCode_hex 54434c2057524f4e4741524753
CASE explicit-error-code code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 5553455220434f4445
CLOSED_CASES_3
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION_QUERY code 0 result_hex 392e312e30
CASE arbitrary-message code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 4e4f4e45
CASE genuine-set-wrongargs code 1 result_hex 77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f22 errorCode_available 1 errorCode_hex 54434c2057524f4e4741524753
CASE explicit-error-code code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 5553455220434f4445
CLOSED_CASES_3
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original fixed ASCII LF CLI source; caught script completions, exact result/errorCode hex and public variable availability, with explicit seed before each case
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION_QUERY code 0 result_hex 302e38342d392d6735626163376339
CASE arbitrary-message code 1 result_hex 77726f6e67202320617267733a2073686f756c64206265202273796e7468657469632076616c756522 errorCode_available 1 errorCode_hex 4e4f4e45
CASE genuine-set-wrongargs code 1 result_hex 77726f6e67202320617267733a2073686f756c642062652022736574207661724e616d65203f6e657756616c75653f22 errorCode_available 1 errorCode_hex 4e4f4e45
CASE explicit-error-code code 1 result_hex 77726f6e67202320617267733a2073686f756c6420626520226572726f72206d657373616765203f737461636b74726163653f22 errorCode_available 1 errorCode_hex 4e4f4e45
CLOSED_CASES_3
```

- Standard error: empty.
