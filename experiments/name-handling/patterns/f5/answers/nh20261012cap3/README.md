# Measured traffic capabilities: nh20261012cap3

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 272
observations: 17 cases across 16 requests each. Every actual response comes
from unit `0:0`; the declared four-unit roster does not establish traffic
execution elsewhere. Each case has setup completion 0 and complete operation
capture. Nine cases return completion 0 and eight completion 1. These are
dynamic commands evaluated inside HTTP_REQUEST.

[reviewed-proofs.json](reviewed-proofs.json) selects one representative per
case. All runtime cleanup captures return completion 0, and exact owned-rule
and virtual post-delete checks find those objects absent. Source, input,
wrapper and case-metadata hashes and materialised setup/operation/cleanup
bytes were checked. Actual response markers and request identities remain
retained beside their drivers and final runtime snapshot.

[runner.py](runner.py) is authored capability template provenance, not a
portable original Tcl input. Caught errors are observed operation outcomes;
they do not imply literal loader acceptance or semantic unsupported status.
No result is transferred to RULE_INIT, procedure, CLI, iApp, iCall, APL or
stock interpreter contexts.
