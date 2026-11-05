# naming.ensemble-worker.original-cmdname-primary

Kind: `native-observation`

## Problem statement

An ensemble mapping prefix can contain an original cmdName object already bound to the a: table. Reconstructing its displayed ::a:::w as a fresh name would lose that native selected command primary.

## Question

Does an explicitly original cached cmdName mapping prefix select the same command as a fresh written ::a:::w?

## Conclusion

C8.6/9.0/9.1 return selected from the original cached mapping and collision from the fresh written command. The original selected command object and equal displayed bytes are distinct input authorities; no source producer or future command lifetime is granted.

## Scope

Original initialized C interpreter, ASCII Tcl_Eval ensemble setup, actual native ENSEMBLE_COMPILE flags, and compiled procedure call. Cached control independently selects the actual a: command table through private native fields and attaches TclSetCmdNameObj before mapping. Result strings are printed after reached calls; no refcounts, pointer values or allocation/lifetime facts are sampled. C8.4 and uncompiled cached branches are explicit no-measurement limits; Jim has a separate setup question.

## Provider answers

### tcl8.4

Status: `not-tested`.

This control is excluded by the probe compile-time release branch. Retained stdout: 'unsupported\tC8.4 has no namespace ensemble compiler'. That is not a measured guest API rejection.

### tcl8.5

Status: `not-tested`.

This control is excluded by the probe compile-time release branch. Retained stdout: 'compiled-collision\tcollision collision ::a:'. That is not a measured guest API rejection.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary. Version: tcl8.6 8.6.18 (capture association); tcl9.0 9.0.4 (capture association); tcl9.1 9.1.0 (capture association).

cached-selected	selected collision

### jim, bigip

Status: `not-tested`.

This exact original selected C ensemble worker control was not run for this provider.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_selected_worker_names/probe.c](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/probe.c)
- [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json)
- [rust/tcl-registry/tests/data/native_selected_worker_names/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.4.20.tsv)
- [rust/tcl-registry/tests/data/native_selected_worker_names/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_selected_worker_names/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_selected_worker_names/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_selected_worker_names/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/9.1.0.tsv)
