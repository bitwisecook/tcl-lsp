# var: real C8.6 naming question

Variable lookup: arrays, namespace-qualified and global names, and variable diagnostics.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `var.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/var.test`. SHA-256 `268e9a926ef7eba63d347fa4fdbb013d1d82b2420540d4625be2ed9b1887841a`. [Byte-exact compressed source](../originals/tests/var.test.gz); [source manifest](../provenance.json).

[Measured results](../results/var.md). [Actual C record](../c/var/record.json) and [Native record](../runtime/var/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
