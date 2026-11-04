# naming.expression-result-normalisation-effects

Kind: `native-observation`

## Problem statement

An algebraic expression rewrite may produce the same number but run a custom operand twice instead of once. Numeric equivalence does not establish effect equivalence.

## Question

Can rewriting the original repeated operand expression preserve its values yet change a custom operand effect count?

## Conclusion

For the captured ordinary numeric/error sources the compared results agree; the custom original emits6 with count2 while rewritten emits6 with count1 on every retained provider. A normal numeric result alone cannot justify eliminating evaluation.

## Scope

Six retained ASCII normalisation.tcl shell runs with exact native outputs and executable digests; no original object-header or BIG-IP claim.

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

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-manifest.json](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/8.4.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/8.4.log). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/8.5.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/8.5.log). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/8.6.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/8.6.log). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/9.0.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/9.0.log). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/9.1.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/9.1.log). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/jim.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/jim.log). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.4.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.4.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.5.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.5.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.6.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.6.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.0.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.0.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.1.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.1.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-jim.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-jim.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/math_binding.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math_binding.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/normal_results.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normal_results.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-15` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.4.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.4.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-16` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.5.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.5.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-17` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.6.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.6.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-18` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.0.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.0.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-19` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.1.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.1.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-20` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-jim.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-jim.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-21` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-22` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/operators_and_shared_pool.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/operators_and_shared_pool.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
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
  "naming.expression-result-normalisation-effects",
  "--provider",
  "8.4",
  "--executable",
  "/path/to/exact/recorded/provider",
  "--output",
  "/tmp/name-resolution-source-replay"
]
```

Select each observed provider explicitly. The driver pins retained evidence bytes, checks independently launched patchlevel, preserves original source/wrapper bytes, compares process exit/stdout/stderr and writes a fresh provider receipt outside the repository. The shared two-slot limit and60-second timeout remain unchanged. Guest errors inside a catch are expected data. This driver replays source-only observations; it supplies no private-header or original-object identity evidence.
