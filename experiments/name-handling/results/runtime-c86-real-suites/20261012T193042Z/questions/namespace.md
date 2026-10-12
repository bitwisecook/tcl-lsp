# namespace: real C8.6 naming question

Namespace lookup, imports, rename/forget chains, ensembles, captured bindings and namespace teardown.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `namespace.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/namespace.test`. SHA-256 `5bfb373b86f503ecfe52d1018a5850c5b24ddf1e5e56101304022a1674d82623`. [Byte-exact compressed source](../originals/tests/namespace.test.gz); [source manifest](../provenance.json).

[Measured results](../results/namespace.md). [Actual C record](../c/namespace/record.json) and [Native record](../runtime/namespace/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
