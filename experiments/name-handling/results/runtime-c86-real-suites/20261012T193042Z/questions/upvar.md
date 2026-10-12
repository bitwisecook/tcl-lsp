# upvar: real C8.6 naming question

Physical variable aliases across call frames and namespaces.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `upvar.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/upvar.test`. SHA-256 `715f0e882539601bf8f1bf33fed09cf2b9a5ab66e7f00bc93a27c73eded64458`. [Byte-exact compressed source](../originals/tests/upvar.test.gz); [source manifest](../provenance.json).

[Measured results](../results/upvar.md). [Actual C record](../c/upvar/record.json) and [Native record](../runtime/upvar/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
