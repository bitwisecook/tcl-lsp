# ooNext2: real C8.6 naming question

TclOO nextto method lookup, mixin call-chain order and next/tailcall command lifetimes.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `ooNext2.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/ooNext2.test`. SHA-256 `fdb5e18350991760d1504aa4de8340e71817463937d0a9d32fdec48a52d1b2c9`. [Byte-exact compressed source](../originals/tests/ooNext2.test.gz); [source manifest](../provenance.json).

[Measured results](../results/ooNext2.md). [Actual C record](../c/ooNext2/record.json) and [Native record](../runtime/ooNext2/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
