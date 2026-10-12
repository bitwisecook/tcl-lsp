# Rejected appliance-specific configuration: nh20261012app1

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 34 unreached
rule-path, computed-target, argument, lifetime and storage-context cases.
The three main configurations all failed load with the captured diagnostic
`"ltm" unknown property` at line 6. Their complete generated configurations
and separate load stdout/stderr/exit files identify the reached boundary.

No actual case lookup is accepted from this run. The later cleanup-rule
markers are separate controls; they cannot supply the rejected application
cases. Post-delete absence transcripts remain attached. An implementation
or transport/configuration error supplies no semantic unsupported answer.

[reviewed-proofs.json](reviewed-proofs.json) is deliberately an empty array.
`MANIFEST.sha256` seals the rejected inputs, raw results and derived records.
