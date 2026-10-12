# upvar: actual C8.6 and Native full-file question

Does the frozen current Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unchanged `upvar.test`, using matching real init.tcl and Tcltest 2.5.11? Earlier state within the file is part of this question.

Original `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/upvar.test`. SHA-256 `715f0e882539601bf8f1bf33fed09cf2b9a5ab66e7f00bc93a27c73eded64458`. [Compressed byte-exact source](../originals/tests/upvar.test.gz); [all original hashes](../provenance.json). Original copyrights and licence remain unchanged.

Each provider uses a fresh process and direct source of the real harness and complete suite. Real Tcltest configuration is `-singleproc 1 -verbose {body error start skip}`; `interp debug {} -frame 1` follows unchanged [tests/all.tcl](../originals/tests/all.tcl.gz). The captured `NH_CONSTRAINT` line verifies real `singleProcess=1` and `singleTestInterp=1`. No test-ID selection or backend overlay applies.

[Results](../results/upvar.md), [C record](../c/upvar/record.json), [Native record](../runtime/upvar/record.json). Each exact wrapper and raw stream is retained beside its record. A host refusal, started marker or incomplete file provides no completed-suite pass rate.
