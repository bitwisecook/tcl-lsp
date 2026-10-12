# oo: real C8.6 naming question

TclOO class/object naming, command forwarding, namespace lifetimes and class/method identities.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `oo.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/oo.test`. SHA-256 `dda92defec55b7c37ae1bcc1f9a199aa3403148341213220e2cb394ef3bb0ceb`. [Byte-exact compressed source](../originals/tests/oo.test.gz); [source manifest](../provenance.json).

[Measured results](../results/oo.md). [Actual C record](../c/oo/record.json) and [Native record](../runtime/oo/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
