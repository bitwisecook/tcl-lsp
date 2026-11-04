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

Status: `observed`. Version: 8.4. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/manifest.json](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.stderr). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.4.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.stderr). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.5.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.stderr). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/8.6.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.stderr). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.0.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.stderr). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/9.1.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.stderr](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.stderr). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.txt](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/jim.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (observation): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/native.tsv](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/native.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (input): [rust/tcl-syntax/tests/data/native_nested_expression_syntax/nested-syntax.tcl](../../../../rust/tcl-syntax/tests/data/native_nested_expression_syntax/nested-syntax.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `maintained-source-replayer` (implementation): [scripts/dev/replay-grammar-native-source.py](../../../../scripts/dev/replay-grammar-native-source.py). Maintained source-only replay adapter; no fresh native execution is recorded by this contribution.

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
