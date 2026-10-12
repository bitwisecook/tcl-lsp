# ooNext2: actual C8.6 and Native full-file question

Does the frozen current Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unchanged `ooNext2.test`, using matching real init.tcl and Tcltest 2.5.11? Earlier state within the file is part of this question.

Original `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/ooNext2.test`. SHA-256 `fdb5e18350991760d1504aa4de8340e71817463937d0a9d32fdec48a52d1b2c9`. [Compressed byte-exact source](../originals/tests/ooNext2.test.gz); [all original hashes](../provenance.json). Original copyrights and licence remain unchanged.

Each provider uses a fresh process and direct source of the real harness and complete suite. Real Tcltest configuration is `-singleproc 1 -verbose {body error start skip}`; `interp debug {} -frame 1` follows unchanged [tests/all.tcl](../originals/tests/all.tcl.gz). The captured `NH_CONSTRAINT` line verifies real `singleProcess=1` and `singleTestInterp=1`. No test-ID selection or backend overlay applies.

[Results](../results/ooNext2.md), [C record](../c/ooNext2/record.json), [Native record](../runtime/ooNext2/record.json). Each exact wrapper and raw stream is retained beside its record. A host refusal, started marker or incomplete file provides no completed-suite pass rate.
