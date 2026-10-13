# Literal command names

Question `command-info-literal-name`: does command lookup treat case and
leading/trailing whitespace as part of a name, and can those spellings hold
distinct commands?

[probe.tcl](probe.tcl) checks `puts`, `::puts`, `Puts`, `PUTS`, four space
variants, a leading tab and a trailing newline in a fresh interpreter. It
first observes `namespace which -command`, then defines each initially missing
spelling and invokes that exact list-built command. No Tcltest harness is used.

All five C references (8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0) and the identified
existing Jim build report `::puts` for the first two spellings and no command
for the other eight before registration. Each of those eight names then
resolves to its own command and returns its distinct marker with completion
code zero. Every interpreter process exits zero with empty stderr.

[empty-name.tcl](empty-name.tcl) separately checks an empty literal name. All
six providers report no such command before registration, `::` after defining
the empty name, and `empty-target` from its exact list-built invocation with
completion code zero. Raw transcripts and exact input/executable provenance
are in [empty-name-results.json](empty-name-results.json).

[oracle-results.json](oracle-results.json) records the exact script, executable
and naming-source hashes, argv, environment selection and raw output paths.
The Jim observation is pinned to the recorded existing checkout; discovery of
the latest upstream revision was not performed by this probe. The C source
hashes identify inspected local inputs; this capture does not independently
verify their upstream archive integrity or reconstruct their build flags.

The CLI integration test `command_info_preserves_exact_command_names` uses
these measured distinctions, including an explicitly supplied empty string.
CLI metadata availability is a separate question:
`ResolvedContext::resolve_spec` over the selected registry answers it, and no
live interpreter binding is inferred from that answer. No F5 execution was
performed; this probe provides no F5 oracle answer.
