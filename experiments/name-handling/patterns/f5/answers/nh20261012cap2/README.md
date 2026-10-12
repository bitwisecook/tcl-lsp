# Capability controls: nh20261012cap2

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) describe the actual
setup, lookup, unit/invocation, error availability and cleanup evidence.

The capture contains 221 reached control records: 17 each in stock Tcl 8.4,
stock Tcl 8.5, CLI, iApp implementation and iCall, plus 17 on each of four
units in RULE_INIT and rule-procedure contexts. Successful and caught-error
completions remain observations of those particular controls. An absent target
or unsupported subcommand result does not establish a general command-availability
rule without the matching source and context.

The 17 traffic records remain unreached. The readiness/controller boundary is
retained in [controller-failure.md](controller-failure.md), with separate load
and cleanup transcripts. APL was not attempted. The final runtime snapshot is
supplementary retained evidence, not a replacement for original transcripts.

[reviewed-proofs.json](reviewed-proofs.json) contains separately scoped central
record proposals. No C/Jim or exact portable-input answer is replaced by these
controls. `MANIFEST.sha256` seals all retained files except itself.
