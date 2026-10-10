# naming.concat-malformed-list-result

Kind: `native-observation`

## Problem statement

A concatenation result may contain an unmatched brace while still being a successful string result. Treating the result as an already parsed List would suppress a later native list error.

## Question

Does concat preserve a malformed list fragment as text, and what happens when llength subsequently parses that result?

## Conclusion

The captured C and Jim source controls keep concat output separate from the subsequent llength completion and exact error string. Successful concatenation does not confer a successful list parse.

## Scope

Exact malformed.tcl ASCII source and six retained shell outputs; source escape evaluation and later list parsing are distinct.

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

Status: `observed`. Version: jim0.84. Build: Recorded executable SHA-256 d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Jim Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-cmd-core/tests/data/native_concat/malformed-manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_concat/malformed-manifest.json). SHA-256 `43170f14fbe327af7859a4d026eb29a26fede8a6b6521bed6eade537db3b015f`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-cmd-core/tests/data/native_concat/8.4.20.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/8.4.20.tsv). SHA-256 `bc561124f4ef020341ee33091ddcf5a0a21bfaa145ab81760e59ecad8eb2b348`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-cmd-core/tests/data/native_concat/8.5.19.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/8.5.19.tsv). SHA-256 `24ec239ec5cda300c610d2ccbdc2da2693925ec76fd4229fb7589cbb1f3dfd97`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-cmd-core/tests/data/native_concat/8.6.18.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/8.6.18.tsv). SHA-256 `cdf24672e3305ed945e87430673c022295d2568312bfb9161020a65d69c25652`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-cmd-core/tests/data/native_concat/9.0.4.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/9.0.4.tsv). SHA-256 `cdf24672e3305ed945e87430673c022295d2568312bfb9161020a65d69c25652`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-cmd-core/tests/data/native_concat/9.1.0.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/9.1.0.tsv). SHA-256 `fb72e12e1728879b28e246258d82a49ea8b94acd89f8c5557153314303f5a927`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.4.20.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.4.20.tsv). SHA-256 `486bce24322d1951a1f235b9e02230d764c44c4b89c3e5c6c1abdefb4fb1fe0a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.5.19.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.5.19.tsv). SHA-256 `486bce24322d1951a1f235b9e02230d764c44c4b89c3e5c6c1abdefb4fb1fe0a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.6.18.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-8.6.18.tsv). SHA-256 `486bce24322d1951a1f235b9e02230d764c44c4b89c3e5c6c1abdefb4fb1fe0a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.0.4.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.0.4.tsv). SHA-256 `9bcad3d1dfeb95d562000b1938c2c90b51eed0169772ac5ca03d36f2097afbf8`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.1.0.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-9.1.0.tsv). SHA-256 `9bcad3d1dfeb95d562000b1938c2c90b51eed0169772ac5ca03d36f2097afbf8`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (input): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.c). SHA-256 `12f526b5dc29dc705afb858bc746bb7c97fa75f663edcd8ce0b36ddeadb5b1ea`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (observation): [rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries-jim.tsv). SHA-256 `486bce24322d1951a1f235b9e02230d764c44c4b89c3e5c6c1abdefb4fb1fe0a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (input): [rust/tcl-cmd-core/tests/data/native_concat/arithseries.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/arithseries.c). SHA-256 `185ac631d6a07bb91eadbea745e6cf341164cbccb2241403c1c3ce6d193a7908`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-14` (input): [rust/tcl-cmd-core/tests/data/native_concat/cases.rs](../../../../rust/tcl-cmd-core/tests/data/native_concat/cases.rs). SHA-256 `0fe7f58facc4d87824cfdfdaa29a10d341dddac93d46274a78ab7640e03eacb3`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-15` (input): [rust/tcl-cmd-core/tests/data/native_concat/jim-probe.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/jim-probe.c). SHA-256 `c2f215041d8fb369ff80f8e867942fe2c8338f8e99b9c21e2efbd378111c101a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-16` (observation): [rust/tcl-cmd-core/tests/data/native_concat/jim.tsv](../../../../rust/tcl-cmd-core/tests/data/native_concat/jim.tsv). SHA-256 `6ba8699df2783a7bc9dc848660eb9b93cbd7e8fa7f40b665cd8bc81b917de0d1`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-17` (input): [rust/tcl-cmd-core/tests/data/native_concat/malformed.tcl](../../../../rust/tcl-cmd-core/tests/data/native_concat/malformed.tcl). SHA-256 `67cc50488bc2d4500e070684cd60730cb5fc57fedf283f0c9f7d2b48cbc50ccc`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-18` (input): [rust/tcl-cmd-core/tests/data/native_concat/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_concat/probe.c). SHA-256 `a26b847b3ed8e3f95e39cb6bc3687874a5439baba0b5b89dd9aefb8465779d37`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
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
  "naming.concat-malformed-list-result",
  "--provider",
  "8.4",
  "--executable",
  "/path/to/exact/recorded/provider",
  "--output",
  "/tmp/name-resolution-source-replay"
]
```

Select each observed provider explicitly. The driver pins retained evidence bytes, checks independently launched patchlevel, preserves original source/wrapper bytes, compares process exit/stdout/stderr and writes a fresh provider receipt outside the repository. The shared two-slot limit and60-second timeout remain unchanged. Guest errors inside a catch are expected data. This driver replays source-only observations; it supplies no private-header or original-object identity evidence.
