# naming.nested-expression-syntax-failure-order

Kind: `native-observation`

## Problem statement

An expression function containing an incomplete Tcl command can be rejected by the script delimiter parser, the outer expression parser or function validation. Reporting a live substitution or normal evaluation from recovery spans could invent an operand effect before the real rejection.

## Question

Which delimiter failure is reported before an absent command bracket in each original malformed nested expression, and can the inner set run?

## Conclusion

All retained cases return an error and leave x equal to NONE. C reports the inner brace, quote or variable failure where applicable; C8.5+ adds expression context. Jim has its distinct unmatched-bracket/function-tree diagnostics. The exact rows retain these differences, not a uniform diagnostic or successful operand execution.

## Scope

Seven exact ASCII expressions evaluated on five C releases and the recorded Jim provider, with full diagnostic bytes and unchanged x. No BIG-IP observation.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4. Build: Recorded executable SHA-256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5. Build: Recorded executable SHA-256 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6. Build: Recorded executable SHA-256 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0. Build: Recorded executable SHA-256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1. Build: Recorded executable SHA-256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim. Build: Recorded executable SHA-256 d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/manifest.json](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/manifest.json). SHA-256 `c3ae22451edf13521ffec00e8637db10a1ac374fd93f101576f293def191ad2d`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.txt). SHA-256 `09e30af52b75db35986e72a823266cc9f20419bb87c8d5ef9686af792e2c2eb6`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.txt). SHA-256 `50f02ae8538b2fed0f9c8e1c807ba6fcc1f182b666b063c592f2cf4d58e058f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.txt). SHA-256 `50f02ae8538b2fed0f9c8e1c807ba6fcc1f182b666b063c592f2cf4d58e058f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.txt). SHA-256 `50f02ae8538b2fed0f9c8e1c807ba6fcc1f182b666b063c592f2cf4d58e058f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.txt). SHA-256 `50f02ae8538b2fed0f9c8e1c807ba6fcc1f182b666b063c592f2cf4d58e058f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.txt). SHA-256 `caa8f1287ee286487c35e46e3f261e0b854adcc781adfcb6d55ece82b79ed977`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/native.tsv](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/native.tsv). SHA-256 `e6a450f5271507db399f5cf71a6343eaed5fe196393726760eab200ab46fc285`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (input): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/nested-syntax.tcl](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/nested-syntax.tcl). SHA-256 `5f2a6b0e3200794a396f68243f8dfe25083c9607dfa99d2f92cf21f953fab1f3`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `maintained-source-replayer` (implementation): [scripts/dev/replay-grammar-native-source.py](../../../../scripts/dev/replay-grammar-native-source.py). SHA-256 `50e5b50aa05b55c5bfcf5e88981d7ccd8da4a3f418a290f3f457c7734e4764bc`. Maintained source-only replay adapter; no fresh native execution is recorded by this contribution.

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
  "scripts/dev/replay-grammar-native-source.py",
  "--proof",
  "naming.nested-expression-syntax-failure-order",
  "--provider",
  "8.4",
  "--executable",
  "/path/to/exact/recorded/provider",
  "--output",
  "/tmp/name-resolution-source-replay"
]
```

Select each observed provider explicitly. The driver pins retained evidence bytes, checks independently launched patchlevel, preserves original source/wrapper bytes, compares process exit/stdout/stderr and writes a fresh provider receipt outside the repository. The shared two-slot limit and60-second timeout remain unchanged. Guest errors inside a catch are expected data. This driver replays source-only observations; it supplies no private-header or original-object identity evidence.
