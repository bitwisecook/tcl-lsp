# naming.regex.c-compiled-pattern-cache-lifetime

Kind: `native-observation`

## Problem statement

A compiled pattern can survive removal of its resident string when its flags match. Recompiling different flags requires a different path, and a duplicate can retain the compiled artifact. Treating every getter as a string read erases the cache-first distinction.

## Question

How do same flags, changed flags, duplicate patterns and invalidated resident bytes affect the original C RegExp cache?

## Conclusion

All five captured same-flag and duplicated-pattern controls retain the same compiled artifact after string invalidation. A flag change with resident bytes produces a different artifact. Changing flags after invalidating the pattern aborts every recorded process with the missing RegExp string-updater message (statuses -6 or -4); those attempts provide no successful completion.

## Scope

Only the selected cases of this retained public C API probe and the recorded five library/header builds are covered. Original input object construction, reference holders, reporting conversions and callback setup are part of the probe. Private physical fields are sampled before result rendering where specified. A recorded build label is not an independent runtime version query. No new native launch, Rust test result, Jim or BIG-IP behavior is claimed; manually invalidated objects and aborted attempts provide no successful adapter or language completion.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.4 8.4.20 (recorded build label; runtime version query not retained).

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\t61\n","stderr":""},{"case":1,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t0\nafter\tregexp\t1\t1\n","stderr":""},{"case":9,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nduplicate\tregexp\t1\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\t61\n","stderr":""},{"case":10,"exit":-6,"stdout":"","stderr":"UpdateStringProc should not be invoked for type regexp\n"}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.5 8.5.19 (recorded build label; runtime version query not retained).

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\t61\n","stderr":""},{"case":1,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t0\nafter\tregexp\t1\t1\n","stderr":""},{"case":9,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nduplicate\tregexp\t1\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t1\t1\nrangebytes\t61\n","stderr":""},{"case":10,"exit":-6,"stdout":"","stderr":"UpdateStringProc should not be invoked for type regexp\n"}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.6 8.6.18 (recorded build label; runtime version query not retained).

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\t61\n","stderr":""},{"case":1,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t0\nafter\tregexp\t1\t1\n","stderr":""},{"case":9,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nduplicate\tregexp\t1\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\t61\n","stderr":""},{"case":10,"exit":-6,"stdout":"","stderr":"UpdateStringProc should not be invoked for type regexp\n"}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.0 9.0.4 (recorded build label; runtime version query not retained).

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\t61\n","stderr":""},{"case":1,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t0\nafter\tregexp\t1\t1\n","stderr":""},{"case":9,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nduplicate\tregexp\t1\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\t61\n","stderr":""},{"case":10,"exit":-4,"stdout":"","stderr":"UpdateStringProc should not be invoked for type regexp\n"}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.1 9.1.0 (recorded build label; runtime version query not retained).

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\t61\n","stderr":""},{"case":1,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\ncache\t0\nafter\tregexp\t1\t1\n","stderr":""},{"case":9,"exit":0,"stdout":"pattern\tregexp\t1\t1\ncompile\t1\nduplicate\tregexp\t1\t1\ncache\t1\nafter\tregexp\t0\t1\nmatch\t1\nsubject\tstring\t1\t1\nrange\tstring\t0\t1\nrangebytes\t61\n","stderr":""},{"case":10,"exit":-4,"stdout":"","stderr":"UpdateStringProc should not be invoked for type regexp\n"}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regsub_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::native_regexp_cache_hit_precedes_getter_and_duplicate_keeps_artifact` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

## Evidence

- [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json)
- [runtime/rust/tests/data/native_regexp_cache/probe.c](../../../../runtime/rust/tests/data/native_regexp_cache/probe.c)
