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

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.4 8.4.20 (recorded build label; runtime version query not retained).

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.5 8.5.19 (recorded build label; runtime version query not retained).

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.6 8.6.18 (recorded build label; runtime version query not retained).

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.0 9.0.4 (recorded build label; runtime version query not retained).

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.1 9.1.0 (recorded build label; runtime version query not retained).

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":6,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tregexp\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t615862\n","stderr":""},{"case":7,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t0\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t1\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t0\t1\ncommandbytes\t5862\n","stderr":""},{"case":8,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncommand\t0\t1\ncommandpattern\tstring\t1\t1\ncommandsubject\tstring\t1\t2\ncommandspec\tstring\t1\t1\ncommandresult\tstring\t1\t2\ncommandbytes\t6162\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regsub_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::native_regsub_mapping_and_regexp_keep_distinct_original_primaries` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

## Evidence

- [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json)
- [runtime/rust/tests/data/native_regexp_cache/probe.c](../../../../runtime/rust/tests/data/native_regexp_cache/probe.c)
