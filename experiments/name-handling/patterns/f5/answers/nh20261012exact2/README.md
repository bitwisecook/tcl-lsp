# Measured original TMM programs: nh20261012exact2

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 1,080
measurements: 180 each in RULE_INIT and procedure, and 720 in HTTP_REQUEST.
Each configuration loads successfully. RULE_INIT and procedure actually
execute on units `0:0`, `0:1`, `0:2` and `0:3`; all traffic responses come
from `0:0`. The roster does not imply traffic execution on the other units.
Original/source/input/wrapper/metadata hashes and all 135 case/context
materialisations were checked. Original corpus operation bytes are unchanged.

The exact final-operation findings are:

| Context | Whole-program completion 0 | Source-qualified final completion 1 |
| --- | --- | --- |
| RULE_INIT | `finite-factory`, `keyed-variable-link` | `factory-body-near-miss`, `namespace-captured-prefix` |
| Procedure | `alias-prefix-lifetime`, `computed-binding-mutation`, `computed-qualified`, `finite-factory`, `keyed-variable-link`, `list-prefix` | `factory-body-near-miss`, `namespace-captured-prefix`, `string-prefix-near-miss` |
| HTTP_REQUEST | Same six procedure cases, on actual traffic unit `0:0` | Same three procedure cases, on actual traffic unit `0:0` |

The factory near miss reaches `::nh::red` and fails reading its uncaptured
`name` variable. The namespace-captured prefix's final operation fails reading
`::nh::prefix`. The string near miss's final `eval` attempts the space-split
command `::nh::two`. These are measured results in the retained frame; they
do not assert an unsupported recogniser, expected setup bindings or dispatch
of an outer command after argument-substitution failure.

[reviewed-reachability.json](reviewed-reachability.json) records all 68
source-qualified error identities, with source/operation hashes and raw stack
row selectors. Its ASCII stack projection is reversible and explicitly
limited to bytes; result and error encodings remain `binary-scan-tcl-value`.
The metadata parser accepts only the complete fixed ASCII presence-marker
schema. [reachability-validation.json](reachability-validation.json) retains
checks rejecting absent/altered projections, unobserved units, changed chunks,
incorrect final lines and malformed presence markers.
[reviewed-proofs.json](reviewed-proofs.json) selects 22 case/context
representatives: 14 whole-program successes and eight source-qualified final
errors. Traffic representatives use request `0`; all captures remain retained.

Ownership guards preserve pre-existing state. In RULE_INIT they reject eight
original cases for existing `description`, `ns`, `target`, `prefix` or
`::forward` names. Procedure and traffic reject both frame-wrapper cases for
existing `::forward`. These collisions leave the original program unreached;
the experiment does not remove or claim ownership of those names. The 720
selected Wiki suite identities fail setup because real `tcltest` is unavailable.
Other whole-program errors in ensemble, OO and tcltest setup do not supply
final-operation answers.

All 1,080 records report runtime cleanup completion 0 and exact owned-object
absence checks. Cleanup of rejected ownership guards acquires no original
state. The later raw TMM snapshot is retained in
[final-runtime.stdout](final-runtime.stdout). No result is borrowed from stock,
CLI, iApp, iCall or APL. `MANIFEST.sha256` seals the retained files.
