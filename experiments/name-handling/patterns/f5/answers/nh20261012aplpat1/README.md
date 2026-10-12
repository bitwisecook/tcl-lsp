# Measured APL pattern callbacks: nh20261012aplpat1

The retained DOM and screenshot identify the selected owned template and
15 rendered APL controls. Each observed browser option value is one NHO row.
Independent projection from [browser-options.json](browser-options.json)
reproduces [invoke.stdout](iapp-apl-0/invoke.stdout) exactly: 87 unique rows,
with complete result chunks. The implementation action is empty.

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain nine reached
operations and six failed setups. Of the reached operations, four return
completion 0 and five completion 1. The renamed namespace and separated
setup/final catches are explicit adaptations. Several normally positive
patterns return missing-command errors here after setup completion 0; these
are measured callback outcomes, not unsupported recogniser classifications
or conclusions about the portable originals.

[reviewed-proofs.json](reviewed-proofs.json) selects the nine reached
operations and attaches raw browser evidence. This run has no independent
interpreter-version control and borrows no patchlevel or namespace fact from
another APL run. Original, actual input, wrapper and case metadata hashes and
materialised callback source were checked.

Every callback reports cleanup completion 0. Two cleanup results contain
caught `unknown command` text; completion 0 does not establish that every
internal teardown action succeeded or that all interpreter names are absent.
The template's own absent-before and post-delete receipts independently
establish ownership and exact configuration-object removal. No other
execution context or APL interpreter lifetime is inferred.
