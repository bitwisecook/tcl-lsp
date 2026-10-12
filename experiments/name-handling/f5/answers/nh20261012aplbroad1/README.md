# Measured broad APL callbacks: nh20261012aplbroad1

The retained DOM identifies the selected owned template and 64 rendered APL
controls. Independent projection of one observed option value per NHO row
from [browser-options.json](browser-options.json) reproduces
[invoke.stdout](iapp-apl-0/invoke.stdout) exactly: 329 unique rows, with complete
result chunks. The screenshot retains the visible portion of the long form;
the implementation action is empty.

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 56 reached
operations and eight failed setups. The reached operations return completion
0 in 31 cases and completion 1 in 25.
[reviewed-proofs.json](reviewed-proofs.json) selects all 56 reached results.
The extra version control reports `8.4.6 8.4 ::script`; its source and digest
are retained in [interpreter-facts.json](interpreter-facts.json). Caught
operation errors remain scoped measurements, without automatically labelling
semantic support or substituting another execution context's answer.

The retained [broader_cases.py](broader_cases.py) and capability source supply
authored template provenance, not portable original Tcl input. Actual callback
source and materialised metadata remain separate evidence. Input, wrapper and
metadata hashes and materialised setup/operation/cleanup bytes were checked.
Every callback reports cleanup completion 0, with its exact result retained;
this does not infer unobserved teardown suboperations or interpreter lifetime.
Exact absent-before and post-delete template receipts prove ownership and
configuration-object removal. No iApp implementation result is borrowed.
