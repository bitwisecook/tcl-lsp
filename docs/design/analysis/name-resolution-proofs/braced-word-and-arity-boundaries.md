# naming.expression.braced-word-and-arity-boundaries

Kind: `native-observation`

## Problem statement

A braced candidate can change whole-word parsing or collapse the original argument vector even when a C expression result would otherwise match.

## Question

Do the malformed-brace and multiple-operand controls preserve the actual source parse and expression argument-vector behavior?

## Conclusion

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: braced-boundary, multiple-operands. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual linked library; original executable; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual linked library; original executable; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual linked library; original executable; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual linked library; original executable; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual linked library; original executable; exact header/Makefile/source/compile command in receipt.. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context.. Dialect: Tcl.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual linked library; original executable; exact header/Makefile/source/compile command in receipt.. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context.. Dialect: Jim Tcl.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `inputs` (input): [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/inputs.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/inputs.json). JSON pointer `/cases`. Exact original/candidate native source and separate authored collector bytes; case claim_scope is input intent, not a result.
- `probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol/probe.c). Exact API scheduling, original-counted input, public channel read, getter chronology and unavailable branches.
- `named-row-audit` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/named-row-audit.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/named-row-audit.json). Offline named-field projection, original line coordinates and pair differences; no fresh execution.
- `v1-probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol/probe.c). Exact earlier probe source; original attempt APIs and abort behavior are preserved.
- `v1-aggregate` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/receipt.json). Independent earlier partial capture; its recorded provider/process coverage is never loaned to later rows.
- `v3-probe` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol/probe.c](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol/probe.c). Exact earlier probe source; original attempt APIs and abort behavior are preserved.
- `v3-aggregate` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/receipt.json). Independent earlier partial capture; its recorded provider/process coverage is never loaned to later rows.
- `tcl8.4-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/stdout.tsv). Exact captured original rows for this provider; finite cases: braced-boundary, multiple-operands.
- `tcl8.4-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.4.20/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.4-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.4.20/receipt.json). Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.4-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.4.20/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.4.20/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.5-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/stdout.tsv). Exact captured original rows for this provider; finite cases: braced-boundary, multiple-operands.
- `tcl8.5-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.5.19/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.5-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.5.19/receipt.json). Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.5-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.5.19/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.5.19/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.6-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/stdout.tsv). Exact captured original rows for this provider; finite cases: braced-boundary, multiple-operands.
- `tcl8.6-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/8.6.18/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl8.6-v1` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/8.6.18/receipt.json). Independent original v1 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl8.6-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.6.18/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/8.6.18/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl9.0-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/stdout.tsv). Exact captured original rows for this provider; finite cases: braced-boundary, multiple-operands.
- `tcl9.0-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.0.4/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `tcl9.0-v3` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/9.0.4/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/capture/9.0.4/receipt.json). Independent original v3 input/provider/build/stream receipt; original partial process state remains explicit.
- `tcl9.1-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/stdout.tsv). Exact captured original rows for this provider; finite cases: braced-boundary, multiple-operands.
- `tcl9.1-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/9.1.0/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.
- `jim-raw` (observation): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/stdout.tsv). Exact captured original rows for this provider; finite cases: braced-boundary, multiple-operands.
- `jim-receipt` (provider): [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/receipt.json](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture/jim/receipt.json). Actual compile/process codes, startup version query, source/header/Makefile/library/executable and stream hashes.

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
