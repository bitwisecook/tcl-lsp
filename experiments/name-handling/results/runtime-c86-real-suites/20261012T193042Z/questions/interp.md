# interp: real C8.6 naming question

Child interpreter command paths, aliases, command hiding and interpreter lookup.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `interp.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/interp.test`. SHA-256 `1a34ca7db200ce0f8936ef537217625f01b525a2eb1fa207c967c71f78b0040b`. [Byte-exact compressed source](../originals/tests/interp.test.gz); [source manifest](../provenance.json).

[Measured results](../results/interp.md). [Actual C record](../c/interp/record.json) and [Native record](../runtime/interp/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
