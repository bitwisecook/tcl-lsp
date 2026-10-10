# naming.variable.counted-formal-foreach-slot-execution-frontier

Kind: `native-observation`

## Problem statement

Two original formal objects k\x00a and k\x00b have equal lengths and CString prefixes. Their formal-list storage and invocation arity can prevent the foreach body from running at all; observing a failed call must not be used to infer which loop variable cell was selected.

## Question

When a procedure is defined with those two counted original formal objects, does invocation with ONE TWO reach the body, and if it does, what static/dynamic foreach and later write results are produced?

## Conclusion

All five C builds accept definition but reject the two-argument call with wrong # args for p k, so these controls establish no C foreach-body cell correspondence. The manifested Jim build reaches the body and returns {ONE CHANGED ONE CHANGED} ONE DYNAMIC ONE DYNAMIC. Original formal installation, body execution and source/static/runtime cell ownership remain independent.

## Scope

Two retained windows per six manifested native builds: original definition via object argv and one original invocation. Formal objects are counted k00a and k00b, exact body bytes in body.hex/probe.c. Jim revision/version is not recorded beyond build hashes. No C loop success, compiler LVT or observer/release capability is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: binary_sha256=9443e048358aadd49092ae92a23c62c649a23487cc034757a3b9c858d3c6074b; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 1 | "wrong # args: should be \"p k\"" |

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: binary_sha256=087bb004cb153775723633b2f6c5d883470988bb4285419ee41cbe5179dd9847; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 1 | "wrong # args: should be \"p k\"" |

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: binary_sha256=60140e5b7ea8c6b472c489c77c9b24f26fd2e163d87bd80acf70c5684ca56636; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{CHANGED CHANGED CHANGED TWO} CHANGED CHANGED CHANGED DYNAMIC" |

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: binary_sha256=7237fa59b7fe340e49c1796d9801bcb61786629c9fbc280d3db7f30903fd9605; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{CHANGED CHANGED CHANGED TWO} CHANGED CHANGED CHANGED DYNAMIC" |

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: binary_sha256=274a0a5ffeedfa3d429976195ba41b38b4b159919ad7bb827e9b71daea176964; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{CHANGED CHANGED CHANGED TWO} CHANGED CHANGED CHANGED DYNAMIC" |

### jim

Status: `observed`. Version: not recorded (manifest label Jim). Build: binary_sha256=a1b8ca904a80a3df74f53d90655a9a8a91c935cc1581093b1130b16ca6cbb5eb; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Jim Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "p" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{ONE CHANGED ONE CHANGED} ONE DYNAMIC ONE DYNAMIC" |

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-vm/tests/data/native_foreach_slots/manifest.json](../../../../rust/tcl-vm/tests/data/native_foreach_slots/manifest.json). SHA-256 `d870c524a716f40c096056926d120625c39ca97d40062920a9d26b4c35fbb2f1`. Six original native builds with compile/run statuses, two-window log hashes, header/library/binary hashes.
- `e1` (input): [rust/tcl-vm/tests/data/native_foreach_slots/probe.c](../../../../rust/tcl-vm/tests/data/native_foreach_slots/probe.c). SHA-256 `15d84213e888a8a47a9de0694cd64e561992f64032288bba273c88ec8d353ce0`. Retained exact probe.c; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (input): [rust/tcl-vm/tests/data/native_foreach_slots/body.hex](../../../../rust/tcl-vm/tests/data/native_foreach_slots/body.hex). SHA-256 `12cffd66658d3fda03801a145812b1bbc3f91cd60b8769a130b045d4a6a5eb1b`. Retained exact body.hex; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e3` (input): [rust/tcl-vm/tests/data/native_foreach_slots/README.md](../../../../rust/tcl-vm/tests/data/native_foreach_slots/README.md). SHA-256 `72fa05057bc1caf0d384afb911193baf7db34cc18b5212aecc8b9cdce4b5d7d7`. Retained exact README.md; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e4` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/8.4.20.tsv). SHA-256 `7d317a9eb12619e46e3ee92bbbc3d1a5046ebd1364f57b470050b356295a052e`. Complete retained semantic TSV for 8.4.20; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e5` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/8.5.19.tsv). SHA-256 `7d317a9eb12619e46e3ee92bbbc3d1a5046ebd1364f57b470050b356295a052e`. Complete retained semantic TSV for 8.5.19; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e6` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/8.6.18.tsv). SHA-256 `795a6a97c0b9f5f5a111b1ffb1d54493ecfcca6c5c745a095dd34fcfe426d3ea`. Complete retained semantic TSV for 8.6.18; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e7` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/9.0.4.tsv). SHA-256 `795a6a97c0b9f5f5a111b1ffb1d54493ecfcca6c5c745a095dd34fcfe426d3ea`. Complete retained semantic TSV for 9.0.4; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e8` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/9.1.0.tsv). SHA-256 `795a6a97c0b9f5f5a111b1ffb1d54493ecfcca6c5c745a095dd34fcfe426d3ea`. Complete retained semantic TSV for 9.1.0; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e9` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/Jim.tsv). SHA-256 `03022c30ce7a3cc105b8b15400c569354780731153f9a7d758e455da78c62576`. Complete retained semantic TSV for Jim; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_foreach_matches_twelve_original_native_slot_records`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::compiled_foreach_matches_twelve_original_native_slot_records` (linked): Compares all twelve original definition/invocation tuples, including every C arity failure; it does not claim twelve successful loop bodies.

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
  "rust/tcl-vm/tests/data/native_foreach_slots/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Compile retained probe.c using each exact manifest command, substituting only authentic source/header/archive/output paths; Jim independently requires USE_JIM and its manifested libjim/link flags. Execute a fresh process per provider, require exit 0 and empty stderr, and project the retained definition/invocation code/result fields to the two corresponding TSV rows. The original raw JSON log is referenced by SHA256 but is not retained as complete content, so a full raw-output comparison requires recovery of that log. All C invocation arity errors and the exact Jim successful body result are part of the pass criterion; do not require universal guest code 0. No Rust execution is inferred.
