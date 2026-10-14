# Dictionary names and missing input

The retained original Tcl script probes `dict with` with a missing empty-name variable and a missing literal `$missing` variable. C Tcl 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9 return error code 1, do not execute the body and leave the binding absent. C Tcl 8.4.20 reports that `dict` is unavailable. All captures exit 0 and preserve their recorded script, executable and original release-library hashes before and after execution. The capture uses no tcltest and records no F5 result.

A source MAY name footprint can retain original possible-name hazards while withdrawing uncertain contents. It cannot activate a dictionary scope, publish successful stores, complete the body or establish Normal execution. The physical dictionary-entry consumer retains its independent missing-input `Invalid` result.

Run `capture.py --repo REPOSITORY --out NEW_RESULT_DIRECTORY` beside `oracles-isolated/`. Each provider uses the retained source-built executable under the repository's `tmp` directory and its matching original Tcl release library. Loader closure is outside these captures.

The conformance control is `dictionary_bindings::tests::source_may_names_do_not_activate_a_known_missing_dictionary`. The source-phase effect control is `resolved_invocation::tests::dictionary_source_phase_keeps_generic_unknown_and_additional_effects`; the original consumer positive remains `ssa::tests::empty_dictionary_scope_source_phases_keep_original_names_and_unknown_alias_contents`.
