# naming.optimiser.recursive-parent-cell

Kind: `native-observation`

## Problem statement

A recursive activation has a preceding procedure activation as its caller, whereas a loop remains in the initially called activation. A relative caller variable lookup can therefore select a different cell even when a local parameter has the same spelling. The controls retain root n=99 and inspect upvar 1 at the final branch.

## Question

Does replacing the measured recursion with its authored loop select the same upvar 1 parent cell?

## Conclusion

All six tested providers return 1 from the original recursion and 99 from the authored loop. The selected caller cell differs; no display-name comparison can repair this frame difference.

## Scope

Exact original and authored-loop programs in both retained ASCII public C Tcl_Eval/Jim_Eval inputs. These native programs are specification comparisons, not transformations extracted from a passing Rust editor. All provider process exits0 are harness completion; each row retains its independent guest completion and result bytes. No BIG-IP observation, pointer across processes or unknown object representation claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Header SHA 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf, static library SHA 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47, exact compile arguments and executable SHA 330951b1453e3073a80cb55d4b56890d6f354cba43983c2ff247275d8235338b retained. Original captured provider attribution is used; no foreign executable identity is borrowed.. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs.. Dialect: Tcl.

upvar parent original1 versus loop99.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Header SHA c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5, static library SHA 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc, exact compile arguments and executable SHA 93566dcf1f131915471cd1fa8027ee84bf9c9df17a45fb581af923326ddea2cc retained. Original captured provider attribution is used; no foreign executable identity is borrowed.. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs.. Dialect: Tcl.

upvar parent original1 versus loop99.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Header SHA aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245, static library SHA 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb, exact compile arguments and executable SHA 56d6b3bb5256efdf8bc7cbc77d66cc106f79681d85b7f0c179a3c4f4e9fe046e retained. Original captured provider attribution is used; no foreign executable identity is borrowed.. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs.. Dialect: Tcl.

upvar parent original1 versus loop99.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Header SHA eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a, static library SHA dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4, exact compile arguments and executable SHA 0bc6a90e2716f597e4307ace3d92901030ec0b2051811866e39fd409e6849c81 retained. Original captured provider attribution is used; no foreign executable identity is borrowed.. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs.. Dialect: Tcl.

upvar parent original1 versus loop99.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Header SHA 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950, static library SHA 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db, exact compile arguments and executable SHA 61ee9453a760af20c223a2ada142e6de43100319b1e65a2a8b54336af6afddef retained. Original captured provider attribution is used; no foreign executable identity is borrowed.. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs.. Dialect: Tcl.

upvar parent original1 versus loop99.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Header SHA d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d, static library SHA a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da, exact compile arguments and executable SHA 16dbeba75c84039b375397aeaf59464c2a1cfce968da28af62cdbdd477b0c2f6 retained. Original captured provider attribution is used; no foreign executable identity is borrowed.. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs.. Dialect: Jim Tcl.

upvar parent original1 versus loop99.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (input): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/probe.c](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/probe.c). SHA-256 `3befb90b4182c246cbcd271688c53c2d8eee5cd75a4137c74757289d18eb5382`. Exact C/Jim public evaluator source, custom object type and conditional trace syntax.
- `e1` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/receipt.json](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/receipt.json). SHA-256 `0928b43ca4e35e7fbc16891576aafd088effbf93a842f420e511902489871b44`. All six exact compile/process receipts, source/header/library/executable hashes, inline guest rows and raw stream hashes.
- `e2` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/8.4.20/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/8.4.20/stdout.tsv). SHA-256 `e3d9fd8ee90d910dd96f7ed3416f7e6d9137831764cc68a8facb51b8fd50feef`. Exact original/loop TSV stream; this question selects case 1 only.
- `e3` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/8.5.19/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/8.5.19/stdout.tsv). SHA-256 `d2e623ee05351968ca36dadfc3416f7a870264241ed84ae1ddda80fbaa1533d2`. Exact original/loop TSV stream; this question selects case 1 only.
- `e4` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/8.6.18/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/8.6.18/stdout.tsv). SHA-256 `d2e623ee05351968ca36dadfc3416f7a870264241ed84ae1ddda80fbaa1533d2`. Exact original/loop TSV stream; this question selects case 1 only.
- `e5` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/9.0.4/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/9.0.4/stdout.tsv). SHA-256 `67ceef7e3ded588dade3bc98ea717eb50cabda90827d0f2d8ad6872789c9ed58`. Exact original/loop TSV stream; this question selects case 1 only.
- `e6` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/9.1.0/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/9.1.0/stdout.tsv). SHA-256 `67ceef7e3ded588dade3bc98ea717eb50cabda90827d0f2d8ad6872789c9ed58`. Exact original/loop TSV stream; this question selects case 1 only.
- `e7` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v1/jim/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1/jim/stdout.tsv). SHA-256 `ea4046c21553702804564b4dd885cda919d2c57986f73e79bf18ab4fb7788186`. Exact original/loop TSV stream; this question selects case 1 only.
- `e8` (input): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/probe.c](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/probe.c). SHA-256 `782a68c3171081d0be4c5e89bb10b4d93f143d52eaaf797b08c7dc1afb457b76`. Exact C/Jim public evaluator source, custom object type and conditional trace syntax.
- `e9` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/receipt.json](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/receipt.json). SHA-256 `f752e39be03907ea1ba8c1da23c4918a4585ea1d7cc38fcbd6f26ff31fc48964`. All six exact compile/process receipts, source/header/library/executable hashes, inline guest rows and raw stream hashes.
- `e10` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/8.4.20/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/8.4.20/stdout.tsv). SHA-256 `d2e623ee05351968ca36dadfc3416f7a870264241ed84ae1ddda80fbaa1533d2`. Exact original/loop TSV stream; this question selects case 1 only.
- `e11` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/8.5.19/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/8.5.19/stdout.tsv). SHA-256 `d2e623ee05351968ca36dadfc3416f7a870264241ed84ae1ddda80fbaa1533d2`. Exact original/loop TSV stream; this question selects case 1 only.
- `e12` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/8.6.18/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/8.6.18/stdout.tsv). SHA-256 `d2e623ee05351968ca36dadfc3416f7a870264241ed84ae1ddda80fbaa1533d2`. Exact original/loop TSV stream; this question selects case 1 only.
- `e13` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/9.0.4/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/9.0.4/stdout.tsv). SHA-256 `d2e623ee05351968ca36dadfc3416f7a870264241ed84ae1ddda80fbaa1533d2`. Exact original/loop TSV stream; this question selects case 1 only.
- `e14` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/9.1.0/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/9.1.0/stdout.tsv). SHA-256 `d2e623ee05351968ca36dadfc3416f7a870264241ed84ae1ddda80fbaa1533d2`. Exact original/loop TSV stream; this question selects case 1 only.
- `e15` (observation): [rust/tcl-compiler/tests/data/native_tail_frame_release/v2/jim/stdout.tsv](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2/jim/stdout.tsv). SHA-256 `ea4046c21553702804564b4dd885cda919d2c57986f73e79bf18ab4fb7788186`. Exact original/loop TSV stream; this question selects case 1 only.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `original_frame_observation_and_release_obligations_withdraw_tail_rewrites` (linked): Require original frame observer/argv object release and producer obligations separately from exact self allocation and formal topology.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-compiler/tests/data/native_tail_frame_release/replay.py",
  "--tcl-source-root",
  "${TCL_SOURCE_ROOT}",
  "--jim-source-root",
  "${JIM_SOURCE_ROOT}",
  "--output",
  "${NEW_OUTPUT}"
]
```

Use matching exact captured header/library hashes, a fresh output directory, the recorded static extension configuration and C compiler. The replayer retains both inputs, checks process exit and raw stdout/stderr exactly, including expected guest failures. Each process has a 60-second budget. Run serially or under the shared native-process limit of two. No Rust pass or actual emitted source transformation is asserted.
