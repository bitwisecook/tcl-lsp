# rename: actual C8.6 and Native full-file question

Does the frozen current Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unchanged `rename.test`, using matching real init.tcl and Tcltest 2.5.11? Earlier state within the file is part of this question.

Original `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/rename.test`. SHA-256 `7404adea7e0f939698c398c2bdc2a7abaaaf854e8086f864ff006ade80c9d081`. [Compressed byte-exact source](../originals/tests/rename.test.gz); [all original hashes](../provenance.json). Original copyrights and licence remain unchanged.

Each provider uses a fresh process and direct source of the real harness and complete suite. Real Tcltest configuration is `-singleproc 1 -verbose {body error start skip}`; `interp debug {} -frame 1` follows unchanged [tests/all.tcl](../originals/tests/all.tcl.gz). The captured `NH_CONSTRAINT` line verifies real `singleProcess=1` and `singleTestInterp=1`. No test-ID selection or backend overlay applies.

[Results](../results/rename.md), [C record](../c/rename/record.json), [Native record](../runtime/rename/record.json). Each exact wrapper and raw stream is retained beside its record. A host refusal, started marker or incomplete file provides no completed-suite pass rate.
