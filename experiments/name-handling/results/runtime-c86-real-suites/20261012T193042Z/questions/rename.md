# rename: real C8.6 naming question

Command identity when renaming, namespace-qualified destinations and deletion.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `rename.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/rename.test`. SHA-256 `7404adea7e0f939698c398c2bdc2a7abaaaf854e8086f864ff006ade80c9d081`. [Byte-exact compressed source](../originals/tests/rename.test.gz); [source manifest](../provenance.json).

[Measured results](../results/rename.md). [Actual C record](../c/rename/record.json) and [Native record](../runtime/rename/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
