# oo: actual C8.6 and Native full-file question

Does the frozen current Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unchanged `oo.test`, using matching real init.tcl and Tcltest 2.5.11? Earlier state within the file is part of this question.

Original `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/oo.test`. SHA-256 `dda92defec55b7c37ae1bcc1f9a199aa3403148341213220e2cb394ef3bb0ceb`. [Compressed byte-exact source](../originals/tests/oo.test.gz); [all original hashes](../provenance.json). Original copyrights and licence remain unchanged.

Each provider uses a fresh process and direct source of the real harness and complete suite. Real Tcltest configuration is `-singleproc 1 -verbose {body error start skip}`; `interp debug {} -frame 1` follows unchanged [tests/all.tcl](../originals/tests/all.tcl.gz). The captured `NH_CONSTRAINT` line verifies real `singleProcess=1` and `singleTestInterp=1`. No test-ID selection or backend overlay applies.

[Results](../results/oo.md), [C record](../c/oo/record.json), [Native record](../runtime/oo/record.json). Each exact wrapper and raw stream is retained beside its record. A host refusal, started marker or incomplete file provides no completed-suite pass rate.
