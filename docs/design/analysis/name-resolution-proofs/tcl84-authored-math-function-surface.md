# naming.tcl84-authored-math-function-surface

Kind: `native-observation`

## Problem statement

An authored Tcl8.4 math simulation must distinguish fixed function validation from runtime operands and wide/numeric conversion. Modern math-command dispatch or a parser capability does not establish the old fixed-function surface.

## Question

What fixed function, arity, numeric-width and operand-effect results do the sixteen original Tcl8.4 math expressions produce?

## Conclusion

The retained Tcl8.4 source/output control defines the tested function and arity/numeric cases only. It supplies a conditional authored provider comparison, not an actual Native function table or BIG-IP appliance seed, state or object header.

## Scope

C8.4-only source.tcl and native84.txt with exact executable/header/implementation digests in the manifest.

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

## Historical references

- `receipt-0` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/manifest.json](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (input): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (input): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (input): [rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl](../../../../rust/tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
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
  "naming.tcl84-authored-math-function-surface",
  "--provider",
  "8.4",
  "--executable",
  "/path/to/exact/recorded/provider",
  "--output",
  "/tmp/name-resolution-source-replay"
]
```

Select each observed provider explicitly. The driver pins retained evidence bytes, checks independently launched patchlevel, preserves original source/wrapper bytes, compares process exit/stdout/stderr and writes a fresh provider receipt outside the repository. The shared two-slot limit and60-second timeout remain unchanged. Guest errors inside a catch are expected data. This driver replays source-only observations; it supplies no private-header or original-object identity evidence.
