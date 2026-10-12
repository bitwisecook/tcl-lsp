# Namespace definition owner control

A newly authored unique-name control measured where `proc` creates a command inside `namespace eval` in each actual wrapper context. BIG-IP 21.1.0.1 build 0.0.26 produced 29 complete measurements with successful ownership setup, lookup and cleanup. The requested namespace and command tail include the run ID; original portable corpus programs remain unchanged.

| Context | Execution namespace before eval | Namespace reported inside eval | Actual command owner |
| --- | --- | --- | --- |
| Stock Tcl 8.4.13 | `::` | Requested unique namespace | Requested namespace |
| Stock Tcl 8.5.13 | `::` | Requested unique namespace | Requested namespace |
| tmsh CLI script | `::script` | Requested unique namespace | Global `::` |
| iApp implementation | `::` | Requested unique namespace | Global `::` |
| iCall | `::` | Requested unique namespace | Global `::` |
| TMM RULE_INIT | `::` | Requested unique namespace | Requested namespace, on each actual unit `0:0`–`0:3` |
| TMM native procedure called from RULE_INIT | `::` | Requested unique namespace | Requested namespace, on each actual unit `0:0`–`0:3` |
| TMM HTTP_REQUEST | `::` | Requested unique namespace | Requested namespace, in all 16 request invocations on actual unit `0:0` |

Each measurement examines three authored candidates: requested-namespace command, caller-namespace command and global command. Where the caller namespace is already global, the latter two rows refer to the same slot. They are repeated observations, not distinct owners. In the CLI wrapper the caller `::script` candidate remains empty.

For the observed owner, `info commands` and `info procs` both return that fully qualified head, its invocation returns code 0 and `TARGET`, and `info body` returns code 0 and exact body `return TARGET`. Every other distinct candidate has empty command/procedure catalogue entries, invocation code 1 and body-query code 1. The outer lookup completion is code 0: inner failure codes are retained list data, not discarded failures.

Ownership setup first proves the requested namespace and every candidate command/procedure absent. Cleanup then removes only the guarded unique candidate commands and requested namespace. Every one of the 29 cleanup measurements returns namespace existence 0, empty command/procedure entries for all candidate rows, and invocation code 1 for each. The four-unit init/procedure results and each traffic invocation retain their actual identities. Named configuration queries after deletion confirm all deployed objects absent.

The TMM native-procedure wrapper here is invoked from RULE_INIT, not HTTP_REQUEST. Only the traffic wrapper has the actual `tcp` and `http` profiles and actual HTTP_REQUEST event. Stock cases run in fresh interpreter processes. This run did not execute APL and supplies no APL ownership answer.

The control makes the requested namespace/current-namespace distinction measurable. It does not establish the implementation mechanism behind the context difference, locate commands created by earlier original programs, or prove those earlier interpreter states were entirely cleaned. No unknown or earlier command name was deleted by this control.

`independent-review.json` records 195 verified file descriptors, 293 verified raw capture rows, materialisation of setup/operation/cleanup, all 29 owner and cleanup identities, and nine exact configuration-absence receipts. `reviewed-proofs.json` contains 14 narrow candidate records: the five stock/hosted receipts, all eight init/procedure unit identities and traffic invocation 0. Every remaining traffic result is retained in `records.json`.

The retained authoring helper is source provenance for this bounded control; it is not an original portable Tcl input. Actual source, case metadata, applied configuration, invocation and raw results remain separate hashed artifacts. Text is decoded as UTF-8 only when the capture marks it UTF-8; readable TMM results use reversible ASCII byte projection with the original `binary-scan-tcl-value` encoding retained. Inner error options are not inferred from result text.
