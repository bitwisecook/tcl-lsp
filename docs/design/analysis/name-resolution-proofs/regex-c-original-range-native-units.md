# naming.regex.c-original-range-native-units

Kind: `native-observation`

## Problem statement

The original pattern and subject can contain FF, modified zero or a four-byte astral sequence. A matched range is a new native object whose string encoding and residency can differ from the original subject and across releases.

## Question

What original range bytes and storage windows follow matching FF, modified zero and astral native input units?

## Conclusion

All selected controls match. C8.4/8.5 retain FF and the four-byte astral spelling in the reported ranges; C8.6–9.1 report C3BF for FF. C8.6 reports the astral range as EDA0BDEDB880, while C9.0/9.1 report F09F9880. Modified zero is C080 in all captured outputs. Range residency differs independently from these bytes, as the exact rows show; this is not a Unicode-normalization or source-channel theorem.

## Scope

Only the selected cases of this retained public C API probe and the recorded five library/header builds are covered. Original input object construction, reference holders, reporting conversions and callback setup are part of the probe. Private physical fields are sampled before result rendering where specified. A recorded build label is not an independent runtime version query. No new native launch, Rust test result, Jim or BIG-IP behavior is claimed; manually invalidated objects and aborted attempts provide no successful adapter or language completion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":3,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\tff\n","stderr":""},{"case":4,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc080\n","stderr":""},{"case":5,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\tf09f9880\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":3,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\tff\n","stderr":""},{"case":4,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc080\n","stderr":""},{"case":5,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\tf09f9880\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":3,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc3bf\n","stderr":""},{"case":4,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc080\n","stderr":""},{"case":5,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\teda0bdedb880\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":3,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc3bf\n","stderr":""},{"case":4,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc080\n","stderr":""},{"case":5,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tf09f9880\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":3,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc3bf\n","stderr":""},{"case":4,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tc080\n","stderr":""},{"case":5,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\tf09f9880\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Historical references

- `original-capture` (provider): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). Immutable build/header/library/executable identities and complete case statuses/streams.
- `original-probe` (input): [runtime/rust/tests/data/native_regexp_cache/probe.c](../../../../runtime/rust/tests/data/native_regexp_cache/probe.c). Exact retained original object constructors, Tcl_EvalEx/Tcl_EvalObjv or RegExp API calls, reporting order and case selector.
- `tcl8.4-case-3` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/0/captures/3`. Exact case 3 process status, raw stdout and raw stderr.
- `tcl8.4-case-4` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/0/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl8.4-case-5` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/0/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl8.5-case-3` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/1/captures/3`. Exact case 3 process status, raw stdout and raw stderr.
- `tcl8.5-case-4` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/1/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl8.5-case-5` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/1/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl8.6-case-3` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/2/captures/3`. Exact case 3 process status, raw stdout and raw stderr.
- `tcl8.6-case-4` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/2/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl8.6-case-5` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/2/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl9.0-case-3` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/3/captures/3`. Exact case 3 process status, raw stdout and raw stderr.
- `tcl9.0-case-4` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/3/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl9.0-case-5` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/3/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl9.1-case-3` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/4/captures/3`. Exact case 3 process status, raw stdout and raw stderr.
- `tcl9.1-case-4` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/4/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl9.1-case-5` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/4/captures/5`. Exact case 5 process status, raw stdout and raw stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regsub_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::native_regexp_ranges_preserve_selected_original_units_and_storage` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.
