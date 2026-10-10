# naming.quoted-tmm-operator-value

Kind: `native-observation`

## Problem statement

A source adapter could mistake a final escaped backslash or command-substitution component for the end of an evaluated quoted expression. The original TMM-style join source checks the actual value independently of parser token geometry.

## Question

What value does the original TMM-style quoted operator expression produce on the six retained interpreters?

## Conclusion

Every retained C and Jim run emits the same value bytes for this exact ASCII source. This checks this quoted source and its Tcl substitution result only; it does not establish BIG-IP appliance state or original object identity.

## Scope

Six script evaluations of probe.tcl; ASCII source through the recorded shell providers. Exact C patchlevels are identified by adjacent parse runs; script rows themselves retain engine and executable digests. Jim patchlevel is not recorded here.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4. Build: Recorded executable SHA-256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5. Build: Recorded executable SHA-256 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6. Build: Recorded executable SHA-256 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0. Build: Recorded executable SHA-256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1. Build: Recorded executable SHA-256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `observed`. Version: jim. Build: Recorded executable SHA-256 d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/manifest.json](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/manifest.json). SHA-256 `c1fe3fded0d04889023225990d55c3866b4b93ded9ea55ea2f9515a40ef27646`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.20.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.20.tsv). SHA-256 `8358bbe134ca093e28ab8c34c8f324128ef147d5e5139a0959350885eeddb568`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.value.txt). SHA-256 `f083b5af7c7a0aa9acd9c8b8f6cd7e9ec3f7e8421597130374fa77880acdd99d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.19.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.19.tsv). SHA-256 `8358bbe134ca093e28ab8c34c8f324128ef147d5e5139a0959350885eeddb568`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.value.txt). SHA-256 `f083b5af7c7a0aa9acd9c8b8f6cd7e9ec3f7e8421597130374fa77880acdd99d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.18.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.18.tsv). SHA-256 `8358bbe134ca093e28ab8c34c8f324128ef147d5e5139a0959350885eeddb568`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.value.txt). SHA-256 `f083b5af7c7a0aa9acd9c8b8f6cd7e9ec3f7e8421597130374fa77880acdd99d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.4.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.4.tsv). SHA-256 `8358bbe134ca093e28ab8c34c8f324128ef147d5e5139a0959350885eeddb568`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.value.txt). SHA-256 `f083b5af7c7a0aa9acd9c8b8f6cd7e9ec3f7e8421597130374fa77880acdd99d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.0.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.0.tsv). SHA-256 `8358bbe134ca093e28ab8c34c8f324128ef147d5e5139a0959350885eeddb568`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.value.txt). SHA-256 `f083b5af7c7a0aa9acd9c8b8f6cd7e9ec3f7e8421597130374fa77880acdd99d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/jim.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/jim.value.txt). SHA-256 `f083b5af7c7a0aa9acd9c8b8f6cd7e9ec3f7e8421597130374fa77880acdd99d`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (input): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.c](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.c). SHA-256 `efdbc9c25a4194a4d4af2f4231a4d36cb30ee895be77d739a02ec6f6a58cee93`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (input): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.tcl](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.tcl). SHA-256 `b227935e82bb9f41be8c917a251665e483a0be3c10f19338756198ff9c912f2f`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
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
  "naming.quoted-tmm-operator-value",
  "--provider",
  "8.4",
  "--executable",
  "/path/to/exact/recorded/provider",
  "--output",
  "/tmp/name-resolution-source-replay"
]
```

Select each observed provider explicitly. The driver pins retained evidence bytes, checks independently launched patchlevel, preserves original source/wrapper bytes, compares process exit/stdout/stderr and writes a fresh provider receipt outside the repository. The shared two-slot limit and60-second timeout remain unchanged. Guest errors inside a catch are expected data. This driver replays source-only observations; it supplies no private-header or original-object identity evidence.
