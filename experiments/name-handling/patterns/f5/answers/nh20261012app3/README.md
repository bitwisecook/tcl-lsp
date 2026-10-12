# Measured cross-rule application controls: nh20261012app3

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 280 reached
measurements. RULE_INIT and procedure contexts each reach 11 cases on four
units. Traffic reaches 12 cases across 16 requests, with all actual responses
from unit `0:0`. The three initial loads succeeded with warnings. Source,
wrapper and case metadata hashes and materialised operation bytes were
checked.

The provider replacement command succeeded before request 8; subsequent
provider calls return `NEW` rather than the earlier `OLD`. The detachment
command succeeded before request 12; the following calls still return `NEW`.
These are bounded observations after the retained controller commands. No
intermediate applied-object receipt or suspended-frame behaviour is inferred.
The static counter returns 1 through 16 across requests with distinct client
ports, with procedure levels and namespace bytes retained in each result.
This establishes the measured sequence on `0:0`, not connection-local storage.

[reviewed-proofs.json](reviewed-proofs.json) selects 71 records: one per case
and context, separate traffic representatives before replacement, after
replacement and after detachment, and every traffic storage repeat. Phase
command/configuration receipts are attached to their relevant proofs.
[appliance.py](appliance.py) is authored template provenance; actual applied
Tcl bytes remain separate evidence.

Static teardown uses three separate cleanup rules. In
[final-runtime.stdout](final-runtime.stdout), each originating rule has one
setup completion 0 and one lookup completion 0 with result hex `30` on all
four units, checking its own static slot after unset. The shared cleanup
case/invocation names must not be joined across originating rules. The main
records' generic runtime-cleanup field is unobserved; these separate receipts
supply the bounded teardown evidence. Post-delete configuration checks find
the owned rules and virtual absent. No result from app1 or app2 is promoted.
