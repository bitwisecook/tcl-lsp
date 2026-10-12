# Rejected literal namespace application controls: nh20261012app2

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 34 unreached
case/context records. All three main TMM configurations failed loading with
`command is disabled: "namespace"` on the provider's literal `namespace current`.
No intended runtime operation was reached.

[reviewed-proofs.json](reviewed-proofs.json) is empty. The retained loader
diagnostics describe this literal configuration, not dynamic namespace
invocation or cross-rule runtime behaviour. Separate cleanup rule output
does not supply a missing lookup result. The corrected dynamic application
run is retained separately as app3.
