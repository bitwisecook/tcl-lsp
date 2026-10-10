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

- `receipt-0` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-manifest.json](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-manifest.json). SHA-256 `a4f68dc180d4bb579465bbda92bdd19490a979a900638f1a820440e0e56de6e9`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/8.4.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/8.4.log). SHA-256 `f8edc8c6779929831bf6ac0e84faa9394ffe67131cf8d6b2cb97451c242ed685`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/8.5.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/8.5.log). SHA-256 `f8edc8c6779929831bf6ac0e84faa9394ffe67131cf8d6b2cb97451c242ed685`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/8.6.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/8.6.log). SHA-256 `87a83a6343e33aa9e3f4ed699b29b3e7dd363dc40435a35abf032e5829b4a712`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/9.0.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/9.0.log). SHA-256 `0d819d1d446a4bc80764b5db948e4f6028c8615cbba6e62d1de8ac235bfc97c8`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/9.1.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/9.1.log). SHA-256 `9f566177b04a75fdfa896d86aba729dfc49608316fe5b4e713e62503aa684a7a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/jim.log](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/jim.log). SHA-256 `f8edc8c6779929831bf6ac0e84faa9394ffe67131cf8d6b2cb97451c242ed685`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.4.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.4.txt). SHA-256 `1a6f1dca53b7656cb383df0a7819f3a26e88a243870b87849f7edbccef7d8a97`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.5.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.5.txt). SHA-256 `d07627ee0cdb8aaf173c5f71f4d3e0e0c8c98d65c18cd4328ae7406fed5ba6f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.6.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-8.6.txt). SHA-256 `d07627ee0cdb8aaf173c5f71f4d3e0e0c8c98d65c18cd4328ae7406fed5ba6f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.0.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.0.txt). SHA-256 `d07627ee0cdb8aaf173c5f71f4d3e0e0c8c98d65c18cd4328ae7406fed5ba6f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.1.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-9.1.txt). SHA-256 `d07627ee0cdb8aaf173c5f71f4d3e0e0c8c98d65c18cd4328ae7406fed5ba6f4`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-jim.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math-binding-jim.txt). SHA-256 `1a6f1dca53b7656cb383df0a7819f3a26e88a243870b87849f7edbccef7d8a97`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/math_binding.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/math_binding.tcl). SHA-256 `efc5aa09e9540f2f6fff429de61de530637fa181f41ca3761fed18b66656b7e2`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/normal_results.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normal_results.tcl). SHA-256 `b0d3f83302c0f9a5f4e8a7775b64c9056adc88bd9b0ec8dea22eea6e5e1354bd`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-15` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.4.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.4.txt). SHA-256 `2c3cce3c64aaf1f65cc4434fd98217386a1252c81492fb4acc622c685a29ba4f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-16` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.5.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.5.txt). SHA-256 `2c3cce3c64aaf1f65cc4434fd98217386a1252c81492fb4acc622c685a29ba4f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-17` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.6.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-8.6.txt). SHA-256 `2c3cce3c64aaf1f65cc4434fd98217386a1252c81492fb4acc622c685a29ba4f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-18` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.0.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.0.txt). SHA-256 `cb5a8ec03f78b0f04fe90bd3d65956cc80c941f5977511f5eeed2b3dad076808`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-19` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.1.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-9.1.txt). SHA-256 `cb5a8ec03f78b0f04fe90bd3d65956cc80c941f5977511f5eeed2b3dad076808`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-20` (observation): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-jim.txt](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation-jim.txt). SHA-256 `05a4699ddf1b0156fd2d87e63b52be2a03fecf98f8736ec402366b6b3d18ed51`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-21` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/normalisation.tcl). SHA-256 `a03361c917d57f420c603d10b0456bb1844e477be6bed133422a893b49aade07`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-22` (input): [rust/tcl-registry/tests/data/native_conditional_expression_results/operators_and_shared_pool.tcl](../../../../rust/tcl-registry/tests/data/native_conditional_expression_results/operators_and_shared_pool.tcl). SHA-256 `0a14244ea067af858bede507927cdd62d3e77e2bdc133bc51bbce6881291e616`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
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
