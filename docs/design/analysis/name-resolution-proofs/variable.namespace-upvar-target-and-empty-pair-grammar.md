# naming.variable.namespace-upvar-target-and-empty-pair-grammar

Kind: `native-observation`

## Problem statement

Namespace upvar can validate pair count before selecting a missing namespace, while Jim helper forwarding may pad an odd pair and select flat variable storage. Treating a common displayed namespace or local alias as the same grammar/cell route changes whether a target exists and which error is returned.

## Question

What exact codes/results follow the six retained namespace upvar no-pair, odd-pair, odd-tail, missing-read, missing-write and current-relative scripts on each selected provider?

## Conclusion

The thirty-six exact results preserve provider-specific pair grammar and target selection: C84 lacks the subcommand, C85 rejects no pairs, modern C allows no pairs, and Jim forwards odd pairs and permits the measured missing-write/current-relative paths. These observations do not establish a missing physical namespace token, header, arbitrary alias-chain closure, or normal completion for other operands.

## Scope

Six ASCII scripts per C84-91 and manifested Jim build, public counted evaluation in fresh interpreters. Separate from original argv eighteen-row grammar and original compiler frontier probes; these inputs query execution through the namespace helper and actual target-cell read/write paths.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-odd-pair | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-odd-tail | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-missing-read | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-missing-write | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |
| upvar-current-relative | 1 | "bad option \"upvar\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which" |

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 1 | "wrong # args: should be \"namespace upvar ns otherVar myVar ?otherVar myVar ...?\"" |
| upvar-odd-pair | 1 | "wrong # args: should be \"namespace upvar ns otherVar myVar ?otherVar myVar ...?\"" |
| upvar-odd-tail | 1 | "wrong # args: should be \"namespace upvar ns otherVar myVar ?otherVar myVar ...?\"" |
| upvar-missing-read | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-missing-write | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-current-relative | 1 | "namespace \"missing\" not found in \"::n\"" |

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 0 | "" |
| upvar-odd-pair | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-odd-tail | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-missing-read | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-missing-write | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-current-relative | 1 | "namespace \"missing\" not found in \"::n\"" |

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 0 | "" |
| upvar-odd-pair | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-odd-tail | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-missing-read | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-missing-write | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-current-relative | 1 | "namespace \"missing\" not found in \"::n\"" |

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 0 | "" |
| upvar-odd-pair | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-odd-tail | 1 | "wrong # args: should be \"namespace upvar ns ?otherVar myVar ...?\"" |
| upvar-missing-read | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-missing-write | 1 | "namespace \"missing\" not found in \"::\"" |
| upvar-current-relative | 1 | "namespace \"missing\" not found in \"::n\"" |

### jim

Status: `observed`. Version: not recorded (manifest label Jim). Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Jim Tcl.

Each row records the guest completion before reporter observation.

| case | code | result |
| --- | --- | --- |
| upvar-no-pairs | 1 | "wrong # args: should be \"upvar ?level? otherVar myVar ?otherVar myVar ...?\"" |
| upvar-odd-pair | 0 | "VALUE" |
| upvar-odd-tail | 0 | "VALUE SECOND" |
| upvar-missing-read | 1 | "can't read \"y\": no such variable" |
| upvar-missing-write | 0 | "VALUE" |
| upvar-current-relative | 0 | "VALUE" |

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (provider): [rust/tcl-vm/tests/data/native_namespace_upvar/manifest.json](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/manifest.json). Six exact build/compiler/library/header/binary/log/fixture hashes and six results per provider.
- `e1` (input): [rust/tcl-vm/tests/data/native_namespace_upvar/probe.c](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/probe.c). Retained exact probe.c; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (input): [rust/tcl-vm/tests/data/native_namespace_upvar/cases.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/cases.tsv). Retained exact cases.tsv; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e3` (input): [rust/tcl-vm/tests/data/native_namespace_upvar/README.md](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/README.md). Retained exact README.md; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e4` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.4.20.tsv). Complete retained semantic TSV for 8.4.20; Each row records the guest completion before reporter observation.
- `e5` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.5.19.tsv). Complete retained semantic TSV for 8.5.19; Each row records the guest completion before reporter observation.
- `e6` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.6.18.tsv). Complete retained semantic TSV for 8.6.18; Each row records the guest completion before reporter observation.
- `e7` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/9.0.4.tsv). Complete retained semantic TSV for 9.0.4; Each row records the guest completion before reporter observation.
- `e8` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/9.1.0.tsv). Complete retained semantic TSV for 9.1.0; Each row records the guest completion before reporter observation.
- `e9` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/Jim.tsv). Complete retained semantic TSV for Jim; Each row records the guest completion before reporter observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_namespace.rs](../../../../rust/tcl-vm/src/cmd_namespace.rs), `namespace_upvar_grammar_and_target_cells_match_36_native_results`: Independent current Rust comparison at the stated semantic boundary.
- [runtime/rust/src/cmd_namespace/native_upvar_tests.rs](../../../../runtime/rust/src/cmd_namespace/native_upvar_tests.rs), `namespace_upvar_grammar_and_target_cells_match_36_native_results`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/cmd_namespace.rs](../../../../rust/tcl-vm/src/cmd_namespace.rs), `cmd_namespace::native_upvar_fixture_tests::namespace_upvar_grammar_and_target_cells_match_36_native_results` (linked): Compares all thirty-six original script completions and target-cell results, including unsupported C84.
- [runtime/rust/src/cmd_namespace/native_upvar_tests.rs](../../../../runtime/rust/src/cmd_namespace/native_upvar_tests.rs), `cmd_namespace::native_upvar_tests::namespace_upvar_grammar_and_target_cells_match_36_native_results` (linked): Independently compares the same thirty-six scripts in the Runtime original native core.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-vm/tests/data/native_namespace_upvar/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Replace provider/header/archive paths with the recorded release/build and preserve the manifested original compiler options. Execute /tmp/native-variable-proof in a fresh process; require all stdout bytes to equal that provider's retained transcript, empty stderr and exit 0. The retained probe uses public object/evaluation entry points; a stock shell invocation is not an equivalent replay. Original artifact hashes identify the observed build, not a fresh rerun. No Rust pass is inferred. Jim requires its independent manifested USE_JIM/libjim build. Preserve cases.tsv exact source bytes and guest error tuples; all host processes exit 0.
