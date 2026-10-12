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

Status: `unsupported`. Version: 8.4.20. Build: binary_sha256=8dfe5d4897f9d3e276c16c61deb8b0731472b887953bc8ae72a8c37d4fc4d272; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

Status: `observed`. Version: 8.5.19. Build: binary_sha256=6ef810ade5e0bb6a9dbcfbf97ff1607ddc9071ea303d9c0d16a0eed7b3b13924; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

Status: `observed`. Version: 8.6.18. Build: binary_sha256=e106bc2ce492cbe28868923dd9803a5e8f5ee7f4a81678c809ee6a8f360de397; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

Status: `observed`. Version: 9.0.4. Build: binary_sha256=b29b2e7ef461b1110a68cb5e3a3bd2aa1b8bad0765d7d8e5381b3ff8060fe86f; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

Status: `observed`. Version: 9.1.0. Build: binary_sha256=5dac7c540c46994653e9610f5ee8f9e93921e34c6e8219ccc9e6227397f8a273; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

Status: `observed`. Version: not recorded (manifest label Jim). Build: binary_sha256=49357ed6e96d9e9d07caecf9e3b6a654c096ae077f2abf5a39a7a33b1ea7a5f8; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Jim Tcl.

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

## Exact evidence

- `e0` (provider): [rust/tcl-vm/tests/data/native_namespace_upvar/manifest.json](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/manifest.json). SHA-256 `1d0bdc8ff5aba585626735bffdc3b16095d0e97224e78c6ddc42593dc44dda9d`. Six exact build/compiler/library/header/binary/log/fixture hashes and six results per provider.
- `e1` (input): [rust/tcl-vm/tests/data/native_namespace_upvar/probe.c](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/probe.c). SHA-256 `c58c9ab4519085344d5a67fbdc0092565339478a08e7b6fbb11a3e18d9627c60`. Retained exact probe.c; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (input): [rust/tcl-vm/tests/data/native_namespace_upvar/cases.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/cases.tsv). SHA-256 `8f24208998d02706a7df324a1defabf7f94e07e1f786c2f6b7a15b038d552922`. Retained exact cases.tsv; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e3` (input): [rust/tcl-vm/tests/data/native_namespace_upvar/README.md](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/README.md). SHA-256 `6809b7220ad5f8f76976c2b3a98b59aeda72b880c4838ebf8b31caf448a37c0d`. Retained exact README.md; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e4` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.4.20.tsv). SHA-256 `b28dfa7d4825778106abe45d69e8f2d89e07169259a32731c11440ab7d3557d1`. Complete retained semantic TSV for 8.4.20; Each row records the guest completion before reporter observation.
- `e5` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.5.19.tsv). SHA-256 `1d9f1029e55aedc3c238ab87c380f630c2b5182bf43ddc939bb131a3ed46c429`. Complete retained semantic TSV for 8.5.19; Each row records the guest completion before reporter observation.
- `e6` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/8.6.18.tsv). SHA-256 `49073af9c720ad2ef5fcd302c0bc21823bb434917ac8d474912c4d88e09bdc37`. Complete retained semantic TSV for 8.6.18; Each row records the guest completion before reporter observation.
- `e7` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/9.0.4.tsv). SHA-256 `49073af9c720ad2ef5fcd302c0bc21823bb434917ac8d474912c4d88e09bdc37`. Complete retained semantic TSV for 9.0.4; Each row records the guest completion before reporter observation.
- `e8` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/9.1.0.tsv). SHA-256 `49073af9c720ad2ef5fcd302c0bc21823bb434917ac8d474912c4d88e09bdc37`. Complete retained semantic TSV for 9.1.0; Each row records the guest completion before reporter observation.
- `e9` (observation): [rust/tcl-vm/tests/data/native_namespace_upvar/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_upvar/Jim.tsv). SHA-256 `3b5b861480267eac11ce991ffd2fcd1bd1e582fd5d66279cee29202da3dcb156`. Complete retained semantic TSV for Jim; Each row records the guest completion before reporter observation.

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
