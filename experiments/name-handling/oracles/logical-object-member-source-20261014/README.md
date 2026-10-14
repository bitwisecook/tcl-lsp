# Per-object method name data

The unchanged original TclOO probe checks literal sigil names, distinct bare receiver names, quoted scalar receivers, spaced method names and visibility masks. Tcl 8.6.18, 9.0.4 and 9.1.0 produce the recorded distinctions; Tcl 8.4.20, 8.5.19 and pinned JimTcl report TclOO unavailable. No package substitute or tcltest harness is used. Raw channels, original executable hashes and unchanged before/after receipts are in result.json.

Conformance mapping: `analyser::types::logical_object_members::tests` and `completion::configured_receiver_completion_tests::logical_object_completion_keeps_selected_names_and_visibility_masks`. These observations establish oracle behaviour; current Rust consumer execution remains separately required. f5-questions.md specifies the appliance questions and answer path.
