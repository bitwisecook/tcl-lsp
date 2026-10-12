# alias target read diagnostic and completion

ID: `naming.variable.alias-target-read-diagnostic-and-completion`

## Problem statement

An alias read may fail at a missing target, a nested alias, an unset root or a scalar/index boundary. The caller spelling and the ultimate target can differ, and C array cells and Jim dictionary variables expose different diagnostic paths. Diagnostic wording is a separate purpose from cell/cache identity.

## Question

Which catch completion and result bytes do the six native engines return for the six original scalar/index alias-chain read cases?

## Answers

### tcl8.4 — observed

missing-one: 1 / can't read "first": no such variable
missing-chain: 1 / can't read "second": no such variable
missing-element: 1 / can't read "second": no such variable
scalar-element: 1 / can't access "target(k)": variable isn't array
defined-chain: 0 / VALUE
unset-chain: 1 / can't read "second": no such variable

Version: 8.4 engine label; patch/revision not recorded. Build: Captured executable SHA 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4. Input channel: Retained ASCII case script through native shell file evaluator; original presenter unavailable.

### tcl8.5 — observed

missing-one: 1 / can't read "first": no such variable
missing-chain: 1 / can't read "second": no such variable
missing-element: 1 / can't read "second": no such variable
scalar-element: 1 / can't access "target(k)": variable isn't array
defined-chain: 0 / VALUE
unset-chain: 1 / can't read "second": no such variable

Version: 8.5 engine label; patch/revision not recorded. Build: Captured executable SHA 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935. Input channel: Retained ASCII case script through native shell file evaluator; original presenter unavailable.

### tcl8.6 — observed

missing-one: 1 / can't read "first": no such variable
missing-chain: 1 / can't read "second": no such variable
missing-element: 1 / can't read "second": no such variable
scalar-element: 1 / can't access "target(k)": variable isn't array
defined-chain: 0 / VALUE
unset-chain: 1 / can't read "second": no such variable

Version: 8.6 engine label; patch/revision not recorded. Build: Captured executable SHA 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0. Input channel: Retained ASCII case script through native shell file evaluator; original presenter unavailable.

### tcl9.0 — observed

missing-one: 1 / can't read "first": no such variable
missing-chain: 1 / can't read "second": no such variable
missing-element: 1 / can't read "second": no such variable
scalar-element: 1 / can't access "target(k)": variable isn't array
defined-chain: 0 / VALUE
unset-chain: 1 / can't read "second": no such variable

Version: 9.0 engine label; patch/revision not recorded. Build: Captured executable SHA cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18. Input channel: Retained ASCII case script through native shell file evaluator; original presenter unavailable.

### tcl9.1 — observed

missing-one: 1 / can't read "first": no such variable
missing-chain: 1 / can't read "second": no such variable
missing-element: 1 / can't read "second": no such variable
scalar-element: 1 / can't access "target(k)": variable isn't array
defined-chain: 0 / VALUE
unset-chain: 1 / can't read "second": no such variable

Version: 9.1 engine label; patch/revision not recorded. Build: Captured executable SHA d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c. Input channel: Retained ASCII case script through native shell file evaluator; original presenter unavailable.

### jim — observed

missing-one: 1 / can't read "first": no such variable
missing-chain: 1 / can't read "second": no such variable
missing-element: 1 / can't read "second": no such variable
scalar-element: 1 / can't read "second": no such variable
defined-chain: 0 / VALUE
unset-chain: 1 / can't read "second": no such variable

Version: jim engine label; patch/revision not recorded. Build: Captured executable SHA d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0. Input channel: Retained ASCII case script through native shell file evaluator; original presenter unavailable.

### bigip — not-tested

No appliance observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.


## Conclusion

Every captured missing/unset chain errors and the defined chain returns VALUE. The scalar-element control distinguishes C array diagnostics from Jim alias-target diagnostics. The retained 36 exact result-byte rows determine each spelling and completion; they do not authenticate a physical alias, cached native name, refcount, return options or normal read in another state.

## Scope

Six retained ASCII programs encoded in cases.tsv, observed original catch-code/result hex across C8.4/8.5/8.6/9.0/9.1/current-label Jim. Patch/revision was not queried in this manifest; executable SHA is retained. Original capture wrapper hashes remain metadata because wrapper files are not retained. No BIG-IP observation.

## Exact retained evidence

- `rust/tcl-syntax/tests/data/native_jim_alias_read/provenance.json` SHA256 `0a47b4d99f3909b44f6f2e15369503d080b156882cef6329b416bd7268c6ca3d`: Exact original native provenance/captured observations.
- `rust/tcl-syntax/tests/data/native_jim_alias_read/cases.tsv` SHA256 `07d5db5df01e9cc19e9cad4bce857ce86785cc26b39c37092914fbc083d167d8`: Retained exact script/case bytes for the stated question.
- `rust/tcl-syntax/tests/data/native_jim_alias_read/8.4.tsv` SHA256 `45910703c3589f87effa362228f718d878e67d284fbadf9052dc5f0a417cc853`: Six original case/catch-code/result-hex rows.
- `rust/tcl-syntax/tests/data/native_jim_alias_read/8.5.tsv` SHA256 `45910703c3589f87effa362228f718d878e67d284fbadf9052dc5f0a417cc853`: Six original case/catch-code/result-hex rows.
- `rust/tcl-syntax/tests/data/native_jim_alias_read/8.6.tsv` SHA256 `45910703c3589f87effa362228f718d878e67d284fbadf9052dc5f0a417cc853`: Six original case/catch-code/result-hex rows.
- `rust/tcl-syntax/tests/data/native_jim_alias_read/9.0.tsv` SHA256 `45910703c3589f87effa362228f718d878e67d284fbadf9052dc5f0a417cc853`: Six original case/catch-code/result-hex rows.
- `rust/tcl-syntax/tests/data/native_jim_alias_read/9.1.tsv` SHA256 `45910703c3589f87effa362228f718d878e67d284fbadf9052dc5f0a417cc853`: Six original case/catch-code/result-hex rows.
- `rust/tcl-syntax/tests/data/native_jim_alias_read/jim.tsv` SHA256 `a69562c38f9b4a7a129f1e984225421ee3ed9438faa7af9e3a3da0cb508d9104`: Six original case/catch-code/result-hex rows.

## Exact input cases

`missing-one`: `proc f {} {upvar #0 missing first; set first}; f`
`missing-chain`: `proc f {} {upvar #0 missing first; upvar 0 first second; set second}; f`
`missing-element`: `proc f {} {upvar #0 missing(k) first; upvar 0 first second; set second}; f`
`scalar-element`: `set target VALUE; proc f {} {upvar #0 target(k) first; upvar 0 first second; set second}; f`
`defined-chain`: `set target VALUE; proc f {} {upvar #0 target first; upvar 0 first second; set second}; f`
`unset-chain`: `set target VALUE; proc f {} {upvar #0 target first; upvar 0 first second; unset first; set second}; f`

## Replay

```sh

```

No original capture wrapper or managed replayer is retained for this question. Decode the retained case bytes, execute each in a fresh recorded engine build, and supply an explicit catch-code/result-byte presenter. A fresh capture is a new observation; require complete equality with every original catch/result row, empty stderr and recorded process completion. No fresh rerun is claimed.

## Rust comparison

`interp::native_jim_links::tests::original_alias_read_errors_match_thirty_six_native_results` in `runtime/rust/src/interp/native_jim_links.rs`. No Rust execution result is recorded here.
