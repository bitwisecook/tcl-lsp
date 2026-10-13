# Evaluated procedure argument oracle observations

Six custom original Tcl programs check name/body substitution, inert body text,
unknown names, replacement of the definer and missing namespace holders. No
Tcltest harness or modified startup script is used. Exact input bytes, binary
hashes, invocation/environment selection, raw stdout/stderr and exit status are
retained in results.json and the provider directories.

| Question | C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 | Identified Jim build |
| --- | --- | --- |
| counted-name | `COUNTED 1 {}` | `COUNTED 1 {}` |
| counted-body | `MATERIALISED 1 1` | `MATERIALISED 1 1` |
| inert-body | `1 0 1` | `1 0 1` |
| replaced-definer | `SHADOW {}` | `SHADOW {}` |
| unknown-name | Error completion1, exact missing-variable message and no `:audit` command | Same measured result |
| missing-holder | Error completion1, no namespace/command created | Original orchestration fails at unavailable `namespace exists`; supplies no complete answer |

All thirty C processes and five completed Jim processes exit zero. The original
Jim missing-holder orchestration exits1 with its raw error retained. A separately
captured direct question uses `namespace which -command` and records C5 rejecting
`proc missing::audit` with the exact unknown-namespace message/no command, while
Jim returns its name and publishes `::missing::audit`. The direct provider
results are independently retained; they do not rewrite the failed orchestration.

The capture identifies the existing Jim executable and checkout. Discovery of
latest upstream is an independent retained root check, not performed by these
probes. Local C executable identities and reported patchlevels are recorded;
this capture does not independently reconstruct build flags or verify upstream
archive integrity. F5 has no result here. Software consumer controls must retain
provider-specific expectations, including Jim's flat missing-holder publication.
