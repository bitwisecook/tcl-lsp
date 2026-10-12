# Measured broad follow-up: nh20261012broad2

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 1,107
measurements: 944 observed, 100 unreached and 63 unavailable. Nine procedure
cases were attempted on four units; eight reached their operations and one
failed setup. Traffic reached 57 cases across 16 request/unit identities
each, producing 912 observations; six cases failed setup. The 63 APL cases
have no presentation callback capture.

[reviewed-proofs.json](reviewed-proofs.json) selects eight procedure and 57
traffic representatives. Each selected record has accepted configuration,
setup completion 0 and a complete reached operation result. Runtime cleanup
completion is 0 and the retained post-delete checks find the owned
configuration objects absent. A caught operation error is answered only for
that recorded operation and context.

The source provenance is the retained authored [broader_cases.py](broader_cases.py)
template. It is not an original portable Tcl input. The actual Tcl bytes,
materialised case metadata, final runtime snapshot and responding execution
identities remain separate evidence. Their source and wrapper hashes and
materialised setup/operation bytes were checked. No APL result, other unit
or unexecuted context is inferred.
