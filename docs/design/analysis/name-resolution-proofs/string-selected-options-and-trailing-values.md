# naming.string.selected-options-and-trailing-values

Kind: `native-observation`

## Problem statement

Option-looking trailing values are data when the selected string equal form reserves its two positional operands. Shortening every matching word could alter those values while a selected option prefix remains legal.

## Question

For six fixed ASCII string equal argv forms, can the selected subcommand/option be shortened while the two reserved trailing values remain exact data in C Tcl 8.4–9.1 and current Jim?

## Conclusion

All six captured providers return 1 for string equal -nocase -nocase -nocase and string e -n -nocase -nocase. Replacing either reserved data operand -nocase with -n produces 0 in the corresponding controls. The option-free two-value form and the selected -length 3 form each return 1. These observations distinguish selected option prefixes from reserved trailing values for the six exact forms; they do not establish arbitrary option grammars or minifier equivalence.

## Scope

Six fixed ASCII source-file argv forms per provider, constructed by Tcl list and invoked through uplevel #0 in the original native process. Exact queried versions, executable/build/source/stream SHA joins and caught results are retained. No raw/counting operand, physical object, compiler admission, cache, storage, observer, Native completion, opaque Unicode or BIG-IP result is established.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Selected executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: C Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Selected executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806. Required header/library/build/source SHA joins are recorded in this provider receipt; compiler and configure/UTF options are not independently interpreted.. Channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0. Dialect: Jim Tcl.

All six fixed forms return catch code 0. The baseline selected -nocase, its selected-prefix shortening, two literal trailing -nocase values, and selected -length 3 control return ASCII 1. Changing a trailing data value from -nocase to -n returns ASCII 0 in both corrupt-tail controls. The original complete stdout records the actual version and result hex; outer exit is 0 and stderr is empty.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation is attached to this exact question.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_string_option_trailing_values/probe.tcl](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/probe.tcl). SHA-256 `8a4987bd87d697144bb07f463116e1e96853bbbb9aa825f9b6ef2be3abd12619`. Exact ASCII source program constructs all six complete argv lists and invokes them through uplevel #0.
- `capture` (input): [rust/tcl-registry/tests/data/native_string_option_trailing_values/capture.py](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/capture.py). SHA-256 `d96185d44373899e52d8b309c5b27eb52a326fb9da6ac09a4d548d67f8433fb7`. Retained original capture runner; replay requires independently verified provider executables and libraries.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/receipt.json). SHA-256 `4d8ee8f81b9c1682c4b6d30c469cdb37b6fefd977d19b11ec56e6b47817c23ab`. Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stdout). SHA-256 `8486d76a25a4f2e580f62cb4a9f93c6cb1799f25f1dbd00c0f0635bcb2a2373d`. Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr stream.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/receipt.json). SHA-256 `1bbf05d2e33b6390e065f58b30a7dd7133265c92585d3bd37f2bd5ad5d91b902`. Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stdout). SHA-256 `7d7aa24c5ecd090b58d9bc1e65b6c8444c11b3d368355b3b2c6768954665bc8b`. Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr stream.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/receipt.json). SHA-256 `d6b6098dabcefcb4776c536b40be3f62161d4dc0b653fd4e39e325983cbc6911`. Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stdout). SHA-256 `6395846428df4e8e4afbc7a33bbb4a7502c1ca1c345abb27123855547c80b320`. Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr stream.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/receipt.json). SHA-256 `052ae545f4f57c9fc7c5e465e41725386e41c081bb3863f70c91bd733cd2b6e3`. Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stdout). SHA-256 `a7bac487c65aaa2bcfce91b60913425235cdf40d4856cb971ca7094fb03738da`. Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr stream.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/receipt.json). SHA-256 `59df3157a3b99425db4dd80df392d279d51f3d670d0f678f3a978656bac6aade`. Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stdout). SHA-256 `2095b20e694269585bcd664662460cdb92438450341d0926a17427cf38750ae9`. Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr stream.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/receipt.json). SHA-256 `caa074a4c78ff66ca06013f280c109b401960b75652eaa0eeeb7306ad4ac0f87`. Original argv, selected executable/version, required build/header/library/source hashes and exact process/stream digests.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stdout](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stdout). SHA-256 `80b27c28a2a24027e038040632b176f6ec965c441082a4b99134bf27730671af`. Exact actual patchlevel and six caught result rows; hex 31 is ASCII 1, hex 30 is ASCII 0.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stderr](../../../../rust/tcl-registry/tests/data/native_string_option_trailing_values/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Replay the byte-identical retained probe.tcl in fresh independently verified native shell processes. capture.py retains the original capture-machine executable paths; required provider/header/library/build/source and stream hashes remain in each receipt. This is a source-file CLI observation, not an object-vector API or Rust execution receipt.
