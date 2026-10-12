# Measured pattern follow-up: nh20261012pat3

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 330 scoped
measurements: 201 observed, 86 unreached and 43 infrastructure failures.
The stock runs use `original-names-stage-split-v1`: they retain the original
names and concatenate the original setup and lookup source, but catch those
stages separately in a fresh process. These are explicit adaptations.

The stock interpreters reached 25 final lookups, 12 in 8.4 and 13 in 8.5.
Traffic reached 11 cases across 16 request/unit identities each, producing
176 observations; four cases failed setup across those same identities.
[reviewed-proofs.json](reviewed-proofs.json) selects one reached representative
per case, context and variant: 25 stock records and 11 traffic records.
The retained responses, applied configuration and final runtime snapshots
remain the evidence for their individual execution identities.

The Wiki trace observer failed with `can't read "nh_options_available": no
such variable`. Those observer failures do not establish the selected Wiki
body's completion or Tcl name-resolution behaviour, and none is promoted.
Whole-program or setup failures likewise supply no reached final lookup.
Original source, actual input and wrapper digests were checked against the
retained files. No result is borrowed from another provider or run.
