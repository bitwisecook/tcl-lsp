# proc: real C8.6 naming question

Procedure birth, namespace-qualified and empty command names, parameter binding and command replacement.

Does the frozen Native C8.6 interpreter agree with actual C8.6.18 on the ordered, unmodified full-file question `proc.test`, using unchanged matching init and Tcltest 2.5.11? Earlier state within the file is part of the question. No case selection or backend constraint overlay applies.

Original: `/workspace/tcl-lsp/tmp/tcl8.6.18/tests/proc.test`. SHA-256 `d79b4b1ad1a374e8839c46175b4f58daad1dfe4e18389e21af9f13f3cf20ece7`. [Byte-exact compressed source](../originals/tests/proc.test.gz); [source manifest](../provenance.json).

[Measured results](../results/proc.md). [Actual C record](../c/proc/record.json) and [Native record](../runtime/proc/record.json) retain exact executable, wrapper and stream hashes. Neither host refusal nor an unreached case is a pass or an assertion failure.
