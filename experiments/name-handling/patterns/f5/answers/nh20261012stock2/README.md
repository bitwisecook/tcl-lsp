# Stock Tcl and traced Wiki measurements: nh20261012stock2

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) contain 90 records:
73 observed completions and 17 constraint-skipped operations. Both stock
interpreter contexts use a fresh process for each selected input.

The 30 corpus records capture whole-program completions. Of those, 21 return
code 0; the nine errors require their explicit whole-program/final-command
reachability limits. They must not automatically answer successful semantic
setup followed by a final lookup.

The 43 reached Wiki records capture actual `isolated` leave completion/result
through an execution-trace observer. This instrumentation is an adaptation,
despite retaining the complete original suite's source bytes. Child inner
error options remain unavailable. Constraint SKIPs supply no body answer.
Raw oracle lines identify Tcl 8.4.13/tcltest 2.2.8 and Tcl 8.5.13/tcltest 2.3.4.
These stock interpreter observations establish no TMM/CLI/iApp/iCall result.

[reviewed-proofs.json](reviewed-proofs.json) contains narrowly scoped proposals;
original C/Jim evidence remains independent. `MANIFEST.sha256` seals all retained
files except the manifest itself.
