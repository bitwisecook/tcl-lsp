# Analyser and optimiser at HEAD over the 14 Tcl-area fixtures

The original analyser and optimiser audit used source at `b0311765`.
`analyser-optimiser-head.tsv` preserves that diagnostic and rewrite census.
`probe.rs` is the original temporary analyser probe.

For [plan item 153](../../../plan.md), the permanent
`item_153_audit_rewrites_preserve_file_execution` test in
`tcl-codegen/tests/it_3/release_optimiser_safety.rs` reran all 49 entries in
`rewrites-executed.tsv` on Tcl 8.6.18 and 9.0.4. It runs each version of a
script from the same filename and compares the exit status, stdout and stderr.
All rows matched. The companion tests exercise the affected constructs on
Tcl 8.4–9.1 where supported and load the O116 list fixture as ISO-8859-1 on Tcl 8.6.18.

Jim fixtures are not included: there is no `jim` dialect profile for the analyser (`DialectProfile::find("jim")` is `None`); the Jim page used the server's `jimtcl` dialect.
