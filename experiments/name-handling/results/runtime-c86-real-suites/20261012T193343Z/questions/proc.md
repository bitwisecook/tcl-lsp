# proc: actual C8.6 and Native full-file question

Does the frozen current Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unchanged `proc.test`, using matching real init.tcl and Tcltest 2.5.11? Earlier state within the file is part of this question.

Original `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/proc.test`. SHA-256 `d79b4b1ad1a374e8839c46175b4f58daad1dfe4e18389e21af9f13f3cf20ece7`. [Compressed byte-exact source](../originals/tests/proc.test.gz); [all original hashes](../provenance.json). Original copyrights and licence remain unchanged.

Each provider uses a fresh process and direct source of the real harness and complete suite. Real Tcltest configuration is `-singleproc 1 -verbose {body error start skip}`; `interp debug {} -frame 1` follows unchanged [tests/all.tcl](../originals/tests/all.tcl.gz). The captured `NH_CONSTRAINT` line verifies real `singleProcess=1` and `singleTestInterp=1`. No test-ID selection or backend overlay applies.

[Results](../results/proc.md), [C record](../c/proc/record.json), [Native record](../runtime/proc/record.json). Each exact wrapper and raw stream is retained beside its record. A host refusal, started marker or incomplete file provides no completed-suite pass rate.
