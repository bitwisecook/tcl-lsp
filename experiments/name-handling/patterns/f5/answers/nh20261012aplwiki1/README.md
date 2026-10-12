# Measured APL Wiki capability boundary: nh20261012aplwiki1

The retained DOM identifies the selected owned template and 31 rendered
controls. Independent projection from [browser-options.json](browser-options.json)
reproduces [invoke.stdout](iapp-apl-0/invoke.stdout) exactly: 243 unique NHO
rows, one per observed DOM option value. The implementation action is empty;
these are actual APL presentation callback captures.

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 30 failed
Wiki suite setups and one reached version control. Every suite setup reports
`can't read "debug": no such variable`; no selected Wiki body or final suite
operation was reached. These errors establish the measured real-tcltest
capability boundary in this callback frame. They do not establish the Wiki
body's Tcl behaviour, semantic unsupported status or a fresh child interpreter.

[reviewed-proofs.json](reviewed-proofs.json) contains only the version
control, which reports `8.4.6 8.4 ::script`.
[interpreter-facts.json](interpreter-facts.json) retains that control's source
digest. Original whole-suite and selected-range hashes, input/wrapper/metadata
hashes and materialised callback source were checked.

The version callback records cleanup completion 0 and result hex `30207b7d`.
The 30 failed suite attempts have no recorded runtime cleanup stage; no
interpreter-state cleanup is inferred for them. Exact absent-before and
post-delete template receipts prove ownership and configuration removal.
`MANIFEST.sha256` seals the retained files without promoting the failed setups.
