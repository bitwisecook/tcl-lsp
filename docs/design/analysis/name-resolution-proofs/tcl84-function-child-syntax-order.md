# naming.tcl84-function-child-syntax-order

Kind: `native-observation`

## Problem statement

An invalid nested command in a function argument may be rejected before the function or argument is entered. Inventing child effects from recovered syntax would alter the function evaluation order.

## Question

How does Tcl8.4 reject the original nested child syntax around fixed math function validation?

## Conclusion

The exact child-syntax.tcl / child-syntax84.txt capture retains Tcl8.4 delimiter/function outcomes and operand marker state. It does not authenticate private child compiler entry or another release.

## Scope

Single explicitly pinned C8.4 provider; original ASCII child-syntax source and complete captured diagnostic/value output.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4 (retained corpus; patchlevel not recorded in this row). Build: Recorded executable SHA-256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4. Channel: Original ASCII script passed to the recorded native shell.. Dialect: Tcl.

The complete original source/output is retained with process exit 0. Its exact function/diagnostic/result rows answer the scoped question; no other release is inferred.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Exact evidence

- `receipt-0` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax-manifest.json](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax-manifest.json). SHA-256 `e433b830aba1e2f3f28e18e4e2a389d15968f72d30c9e71b2b92391dec0ddf48`. Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (input): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl). SHA-256 `eb4287197135aaa2da91716194cbb4dee9aab60783c0b72ce10550f0279ae967`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt). SHA-256 `1e8bde2d28b996f81f88f1517d6019f89def1ee44aca181f16847d62d2d04eba`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (input): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl). SHA-256 `567621c9f1705a5c87ae046ab0e1080b89ed8469fc6638372fcc63be6a221987`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt). SHA-256 `456cc116d041ff444c175d9da1eaad587f61edb92c6485bca542669e9b0ebcf2`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt). SHA-256 `a6c618ff9a8a4205ed5a56396eab8c1396f552e8bfbfdce5ebe7e6e3b7f44107`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl). SHA-256 `7d8fe3c2e0765f78cb6d1b5eabbc35b3888b8ac2dc9b6196437bc573ab1a787a`. Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
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
  "naming.tcl84-function-child-syntax-order",
  "--provider",
  "8.4",
  "--executable",
  "/path/to/exact/recorded/provider",
  "--output",
  "/tmp/name-resolution-source-replay"
]
```

Select each observed provider explicitly. The driver pins retained evidence bytes, checks independently launched patchlevel, preserves original source/wrapper bytes, compares process exit/stdout/stderr and writes a fresh provider receipt outside the repository. The shared two-slot limit and60-second timeout remain unchanged. Guest errors inside a catch are expected data. This driver replays source-only observations; it supplies no private-header or original-object identity evidence.
