# Original namespace query and scalar report-address probe

Benign original executable probes on 2026-10-13, C8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1.0 and selected latest Jim checkout reporting `0.84-9-g5bac7c9`. All inputs and outputs are retained here. No Tcltest, Rust test, build, checkout mutation, F5 execution or Native allocation/cell identity is claimed.

| Operation | All five C interpreters | Selected original Jim |
| --- | --- | --- |
| `info vars {::N[0]::*}` | Actual bracket namespace variable `::N[0]::x` | `invalid command name "0"` |
| `info vars {::N\[0\]::*}` | Empty | Sibling `::N0::y` |
| `info commands {::N[0]::*}` | Actual bracket namespace command `::N[0]::p` | Sibling `::N0::q` |
| `info commands {::N\[0\]::*}` | Empty | Actual bracket namespace command `::N[0]::p` |
| `namespace children :: {::N[0]}` | Sibling `::N0` | Unsupported `children` command |
| `namespace children :: {::N\[0\]}` | Actual bracket namespace `::N[0]` | Unsupported `children` command |

Original direct bracket namespace scalar/command calls return the bracket values in every provider. C `info vars`/`info commands` namespace qualifiers are literal for this controlled input; escaping the complete qualified prefix changes the namespace address and is incorrect. The `namespace children` optional full-name pattern is a different operation. Its default enumeration needs no added pattern. Jim's queries have independently observed behavior; it must not receive C's recipe by compatibility inheritance.

All six providers retain two distinct controlled global inputs: original unqualified `:errorInfo` reads `COLON_ERROR_INFO`, while `errorInfo` reads `PLAIN_ERROR_INFO`. `info vars ::*` reports `:::errorInfo` and `::errorInfo`; reusing either complete report as a `set` operand reads `PLAIN_ERROR_INFO`. Trimming all leading colons also maps both reports to `errorInfo`. Therefore neither the report string nor its colon-trim projection selects the separately created colon cell. A snapshot must retain an adequate original operand and scope, or delimit unavailable addresses; a new string normalizer cannot supply cell authority.

The first `input.tcl`/`capture.py` preserves five successful C captures and the initial Jim exit1: the original info-vars call was evaluated before the query helper's catch and halted the script. `continued/input.tcl` moves each query inside the catch. Its six processes exited0, with each query's own code/result recorded separately, including Jim unsupported operations. Successful process exit is not query or conformance success.

Each provider directory contains raw stdout/stderr, exact command/environment/provenance, and executable/linkage/library seals before and after. All executable, linked-file and C library file hashes remained unchanged. Pinned upstream source IDs, Jim source Git HEAD and original C init hashes are in `source-identities.json`; `observations.json` records the measured classifications and limits. No Tcltest copy was used.
