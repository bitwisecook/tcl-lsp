# naming.expression.braced-substitution-and-function-context

Kind: `native-observation`

## Problem statement

Bracing moves variable/script substitution into expression evaluation, while function lookup remains an independent execution dependency.

## Question

What do the retained variable-read, script callback and registered-function controls measure in the original and braced evaluation contexts?

## Conclusion

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: variable-substitution, script-substitution, registered-function. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual linked library SHA256 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47; original executable SHA256 598d9b72ed8e4c235656b9f9e5223f2b7859085fc4e5e0f11d10f7dac108a029; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual linked library SHA256 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; original executable SHA256 f9949823402f6f9661c4da4c9eb8c713a789eae596a37434d2ceaafb5c2396e4; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual linked library SHA256 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; original executable SHA256 fe6291baefd99c21fe25d5ee33342738be2f88facc51e649e7981ac073237eb0; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual linked library SHA256 dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; original executable SHA256 2a54ce88d18b84c1b97637c5d2ce83f6bdf8e37ad153325fc72214fe2e780f1c; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual linked library SHA256 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; original executable SHA256 e4f14fe01bbedb3f521c7a1c06f1f80ffdf5fe1b2a50d2633983e5d2b3fbf4ee; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual linked library SHA256 a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da; original executable SHA256 3a4fbb7ce3c3a54e0ff42e3c633e794a12af62a696653f6465f2b3c12eb3f6ca; exact header/Makefile/source/compile command in receipt.. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context.. Dialect: Jim Tcl.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `inputs` (input): [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/inputs.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/inputs.json). SHA-256 `124ebfcf57fd39baf1a43abae30c629a0c344dbdc8e3375fcfd29bbb3a3d9579`. JSON pointer `/cases`. Exact original/candidate native source and separate authored collector bytes; case claim_scope is input intent, not a result.
- `probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/probe.c). SHA-256 `a4488e0294425c25de6ec2ecbae861b71e3cec00f55bbe498a62ec99a659270b`. Exact API scheduling, original-counted input, public channel read, getter chronology and unavailable branches.
- `named-row-audit` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/named-row-audit.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/named-row-audit.json). SHA-256 `56af7482ede0131992aedf69df42bde4d7ca60a95353f0955d4e100656de06b1`. Offline named-field projection, original line coordinates and pair differences; no fresh execution.
- `v1-probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol/probe.c). SHA-256 `a55348a9ce001e5669b9ee7a22041dfdeb47b1f29082543ddfdd59ca1c6e8ce5`. Exact earlier probe source; original attempt APIs and abort behavior are preserved.
- `v1-aggregate` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/receipt.json). SHA-256 `5b78abb835cb762c2096b3c814c78f7457a25b911ae33d9a1215313619f5f935`. Independent earlier partial capture; its recorded provider/process coverage is never loaned to later rows.
- `v3-probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol/probe.c). SHA-256 `4ffa31dc77b6d2f3f7ae2a498d16b93a257d5ed8aebf5635806c43b3680ac57a`. Exact earlier probe source; original attempt APIs and abort behavior are preserved.
- `v3-aggregate` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/receipt.json). SHA-256 `ee73fca28d4e5d1e26eb1c4f8f3276267ff524e2d439ddb0fe0f8cc8416b6a59`. Independent earlier partial capture; its recorded provider/process coverage is never loaned to later rows.
- `tcl8.4-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/stdout.tsv). SHA-256 `6cabb0944f89c06a936c9acc9e0acff1bc2c47de9bfb284fcc07eae02d1a5f30`. Exact captured original rows for this provider; finite cases: variable-substitution, script-substitution, registered-function.
- `tcl8.4-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/receipt.json). SHA-256 `16bc7679f12f9c91985d3b900886c4ceddbd7f97e972e1140794708aed200a76`. Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.4-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.4.20/receipt.json). SHA-256 `7d1780aabd4ae56ee53404f249084e0a08eba1e1764aeca53cffa887471ee90c`. Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.4-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.4.20/receipt.json). SHA-256 `cbeec90742bf585b1655a251dcd2127f7d80d8f38b26eea908f60f728a10cdd3`. Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.5-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/stdout.tsv). SHA-256 `947f99e7f9d8a99a3dbdfae6916b484e7172e5c6c0922c7e106df75afc717e2f`. Exact captured original rows for this provider; finite cases: variable-substitution, script-substitution, registered-function.
- `tcl8.5-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/receipt.json). SHA-256 `6c7f55b8bbb603bdc20e17ae08f0cc4077781cb32e6cd1a3c4a37bb5a735ec1a`. Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.5-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.5.19/receipt.json). SHA-256 `c5be88b8723de10e7c55869af571c3a80c9d7bd819b7bccd291e49f3c7adb2d4`. Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.5-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.5.19/receipt.json). SHA-256 `81b883c95d0678f2f4767837b7bbc570103ca5247327f82290caabd790dfdd6c`. Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.6-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/stdout.tsv). SHA-256 `f3d57171ed2d58e213542389520f7d79ac1c35311d0b2eb0897e06712a15f590`. Exact captured original rows for this provider; finite cases: variable-substitution, script-substitution, registered-function.
- `tcl8.6-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/receipt.json). SHA-256 `7281b8ed2cbda742c8927e07187a86c394c7120bf0ba4bedf57e1f7e4825b3a8`. Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.6-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.6.18/receipt.json). SHA-256 `ecc5d50b094c951130b20a2b4ff0a5d9e10836ddc19b7f43a5acd7c5b6027e67`. Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.6-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.6.18/receipt.json). SHA-256 `1b03511eb68a16eadcc3c20deb011ffc44339d35fd44fdcc9f1a6fc7aac8e1bd`. Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl9.0-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/stdout.tsv). SHA-256 `b2594f39e8fad3dd170b8f33db35a94647b6953b069691f330772b86c78f141c`. Exact captured original rows for this provider; finite cases: variable-substitution, script-substitution, registered-function.
- `tcl9.0-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/receipt.json). SHA-256 `584628499b005164ec74dc2460c5384349a5201d5422730336e14f8349e08546`. Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl9.0-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/9.0.4/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/9.0.4/receipt.json). SHA-256 `2ca775de7a7c854feb0bede292944db4b4eb7de6ccac9ea65da748d4fffebf84`. Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl9.1-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/stdout.tsv). SHA-256 `408723d0ef8a7968441c8bddd4560cb3b17190674bb22d47c54b6cdbcea21fe0`. Exact captured original rows for this provider; finite cases: variable-substitution, script-substitution, registered-function.
- `tcl9.1-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/receipt.json). SHA-256 `3927bcd8ebe938a6ae56461b6893fc3a15a74556959b264ae38125dc9299a283`. Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `jim-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/stdout.tsv). SHA-256 `b140b826ec7c52f55a4319fad973ee0afa389a867c0c1bd62bf257d0f14462ec`. Exact captured original rows for this provider; finite cases: variable-substitution, script-substitution, registered-function.
- `jim-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/receipt.json). SHA-256 `929cdb9248b0acb3ab7f094357a78e2919428309fed1e83bcdb167c4c7dc97fc`. Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-compiler/tests/data/native_brace_expression/replay.py",
  "--c-root",
  "/workspace/tcl-lsp/tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "<new-output-directory>"
]
```

Replay reproduces v4 only with original pinned headers/Makefiles/libraries/sources and exact streams. Earlier failed/partial attempts remain immutable limitations. Public result getters may materialise strings after the recorded primary/resident sample. No optimiser or Rust test is executed by this protocol.
