# naming.regex.c-regsub-primary-path-selection

Kind: `native-observation`

## Problem statement

A backreference substitution uses a compiled RegExp path, a simple literal replacement may use a mapping path, and a no-match call may return the original subject. Equal textual results do not reveal which original object or primary was retained.

## Question

How do RegExp replacement, literal mapping and no-match regsub differ in pattern primary, result residency and subject identity?

## Conclusion

All five captured backreference replacements retain a RegExp pattern and return aXb; literal replacements leave string pattern primary and return Xb. The no-match control returns the same original subject ab with its increased reference count. C8.4/8.5 replacement results already have resident strings; later replacement results are sampled before materialization. These are the fixed original vectors, not all optimization choices.

## Scope

Only the selected cases of this retained public C API probe and the recorded five library/header builds are covered. Original input object construction, reference holders, reporting conversions and callback setup are part of the probe. Private physical fields are sampled before result rendering where specified. A recorded build label is not an independent runtime version query. No new native launch, Rust test result, Jim or BIG-IP behavior is claimed; manually invalidated objects and aborted attempts provide no successful adapter or language completion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded build label; runtime version query not retained). Build: Library SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; probe executable SHA256 254739d0e2469770dc8ff8d0a79abc8aa8f3e0ddc8e0d37d20a22938e8991f43; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded build label; runtime version query not retained). Build: Library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; probe executable SHA256 174b3d5380e50f6a40d6aa40f3c4199d6ab8a04e82a5578c2a8208cd03618b2a; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build label; runtime version query not retained). Build: Library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; probe executable SHA256 4a36829b28f7c202cd55a531978d1963fd4c6618c933ac7d56c8206ef4f52ac0; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build label; runtime version query not retained). Build: Library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; probe executable SHA256 59abc79a4fcdc39cbb69e49cb6226d0de5f14ee5e3a4515ebbbbdd8b4f18e58c; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build label; runtime version query not retained). Build: Library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; probe executable SHA256 2a7482f87de4f8443c1798aed562002ab32848b9382905eae1bbcc32aef51001; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Exact evidence

- `original-capture` (provider): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. Immutable build/header/library/executable identities and complete case statuses/streams.
- `original-probe` (input): [runtime/rust/tests/data/native_regexp_cache/probe.c](../../../../runtime/rust/tests/data/native_regexp_cache/probe.c). SHA-256 `ece4064597072cc821d332f2bc5de3467604edec8d8d153736aaa4b31ce76876`. Exact retained original object constructors, Tcl_EvalEx/Tcl_EvalObjv or RegExp API calls, reporting order and case selector.
- `tcl8.4-case-6` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/0/captures/6`. Exact case 6 process status, raw stdout and raw stderr.
- `tcl8.4-case-7` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/0/captures/7`. Exact case 7 process status, raw stdout and raw stderr.
- `tcl8.4-case-8` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/0/captures/8`. Exact case 8 process status, raw stdout and raw stderr.
- `tcl8.5-case-6` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/1/captures/6`. Exact case 6 process status, raw stdout and raw stderr.
- `tcl8.5-case-7` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/1/captures/7`. Exact case 7 process status, raw stdout and raw stderr.
- `tcl8.5-case-8` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/1/captures/8`. Exact case 8 process status, raw stdout and raw stderr.
- `tcl8.6-case-6` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/2/captures/6`. Exact case 6 process status, raw stdout and raw stderr.
- `tcl8.6-case-7` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/2/captures/7`. Exact case 7 process status, raw stdout and raw stderr.
- `tcl8.6-case-8` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/2/captures/8`. Exact case 8 process status, raw stdout and raw stderr.
- `tcl9.0-case-6` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/3/captures/6`. Exact case 6 process status, raw stdout and raw stderr.
- `tcl9.0-case-7` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/3/captures/7`. Exact case 7 process status, raw stdout and raw stderr.
- `tcl9.0-case-8` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/3/captures/8`. Exact case 8 process status, raw stdout and raw stderr.
- `tcl9.1-case-6` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/4/captures/6`. Exact case 6 process status, raw stdout and raw stderr.
- `tcl9.1-case-7` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/4/captures/7`. Exact case 7 process status, raw stdout and raw stderr.
- `tcl9.1-case-8` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). SHA-256 `27428a28d53c5aa18e9741bddb805f2afb7221cfe3d49feff092cf8a3e163605`. JSON pointer `/4/captures/8`. Exact case 8 process status, raw stdout and raw stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regsub_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::native_regsub_mapping_and_regexp_keep_distinct_original_primaries` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.
