# Corrected stock measurements: nh20261012stock3

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 90
measurements: 68 observed and 22 unreached. Stock Tcl 8.4 reaches 12 staged
corpus operations and 21 selected Wiki body returns; stock Tcl 8.5 reaches
13 staged corpus operations and 22 selected Wiki body returns. The corrected
trace observer initialises its capture variables before evaluating the
retained suite.

[reviewed-proofs.json](reviewed-proofs.json) selects all 68 reached results.
The 25 corpus records use original names and concatenated source bytes but
split setup and final operation into separate catches in fresh processes.
The 43 Wiki records use an execution trace to capture isolated body leave
completion and result; child inner error options remain unavailable. Both
are explicit adaptations. Failed setup and real tcltest skips remain
unreached.

Original, actual input and wrapper hashes were checked, as were the original
corpus source concatenations and materialised setup/operation bytes. These
records do not fill exact-original-input answers. The earlier stock2 run's
21 successful whole-portable-program records remain the conservative exact
stock candidates; its whole-program errors do not establish their final
intended lookups.
