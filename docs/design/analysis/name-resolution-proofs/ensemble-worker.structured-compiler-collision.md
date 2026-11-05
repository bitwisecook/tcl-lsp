# naming.ensemble-worker.structured-compiler-collision

Kind: `native-observation`

## Problem statement

A maker namespace component a: and ordinary a display ambiguously when a compiled ensemble maps member to w. Source-selected structured context and reconstructed printed target can select different commands.

## Question

Which command result does compiled E mem VALUE select from a: compared with directly written ::a:::w?

## Conclusion

C8.5–9.1 return collision for both compiled ensemble call and directly written target, while namespace current displays ::a:. This finite compiler-selected mapping does not retain the structured a: worker merely from its presentation.

## Scope

Original initialized C interpreter, ASCII Tcl_Eval ensemble setup, actual native ENSEMBLE_COMPILE flags, and compiled procedure call. Cached control independently selects the actual a: command table through private native fields and attaches TclSetCmdNameObj before mapping. Result strings are printed after reached calls; no refcounts, pointer values or allocation/lifetime facts are sampled. C8.4 and uncompiled cached branches are explicit no-measurement limits; Jim has a separate setup question.

## Provider answers

### tcl8.4

Status: `not-tested`.

This control is excluded by the probe compile-time release branch. Retained stdout: 'unsupported\tC8.4 has no namespace ensemble compiler'. That is not a measured guest API rejection.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary. Version: tcl8.5 8.5.19 (capture association); tcl8.6 8.6.18 (capture association); tcl9.0 9.0.4 (capture association); tcl9.1 9.1.0 (capture association).

compiled-collision	collision collision ::a:

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
