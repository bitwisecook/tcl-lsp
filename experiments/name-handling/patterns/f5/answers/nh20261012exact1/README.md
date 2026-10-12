# Measured original hosted programs: nh20261012exact1

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 135
measurements across CLI, iApp implementation and iCall. Each context evaluates
15 byte-identical original corpus programs separately under ownership guards
and attempts 30 selected Wiki suites. All configurations load successfully.
Original/source/input/wrapper/metadata hashes and materialised operation bytes
were checked. The actual host namespace and frame remain part of each result.

Each context returns whole-program completion 0 for `frame-wrapper`,
`frame-wrapper-near-miss`, `list-prefix` and `namespace-captured-prefix`.
The remaining 11 corpus programs return completion 1. Whole-program errors
do not automatically answer a canonical final-operation question.

[reviewed-reachability.json](reviewed-reachability.json) anchors independent
source review for five errors per context: `alias-prefix-lifetime`,
`computed-qualified`, `factory-body-near-miss`, `finite-factory` and
`string-prefix-near-miss`. Their complete error stacks locate the failing
command in the authored final operation. Sequential evaluation reaching that
source line establishes completion of the preceding original prefix. This
does not establish that setup created every expected binding or that an outer
command dispatched after argument-substitution failure. Exact source and
operation hashes, metadata pointers and raw capture row selectors retain the
review's provenance.

[reviewed-proofs.json](reviewed-proofs.json) selects 27 original final-operation
results: 12 whole-program successes and the 15 source-qualified final errors.
[reachability-validation.json](reachability-validation.json) retains fixed
validation checks. Valid reviews were accepted; wrong final ranges, an earlier
keyed-variable failure, duplicate identities, altered raw bytes/digests and
missing raw error-stack chunks were rejected.

`computed-binding-mutation`, `ensemble-prefix-map`, both OO cases and
`tcltest-shared-lifecycle` fail before their final operations.
`keyed-variable-link` fails its earlier `change` call before the final `set`.
The 90 selected Wiki suite attempts fail setup with the missing `debug`
variable; none supplies a selected body result or unsupported semantic answer.

Each record reports runtime cleanup completion 0, with owned-object absence
receipts retained. The later scriptd snapshot comes from `/var/tmp/scriptd.out`.
The unavailable `/var/log/scriptd` attempt is preserved separately and supplies
no missing runtime result. No result is borrowed from stock, TMM or APL.
`MANIFEST.sha256` seals the retained files.
