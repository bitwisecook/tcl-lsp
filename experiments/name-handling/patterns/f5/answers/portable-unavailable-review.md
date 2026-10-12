# Remaining portable whole-program errors: final-operation reachability

Independent source review of the 23 portable matrix rows previously marked unavailable proves an earlier original-program failure in every pointed capture. None of these captures reaches its authored final operation. The appropriate measured boundary is unreached, with the actual whole-program error retained as an attempt. No canonical final-operation answer is promoted. This does not finish the broader outstanding question task.

Every review checks the original question/source SHA-256, byte-identical operation in case metadata, original operation materialised in actual input, accepted load, successful wrapper guard, complete errorInfo chunks and their raw line coordinates, actual context/unit/invocation and sequential original source order. Wrapper guard completion 0 does not establish original source setup completion. Security and authentication metadata are omitted.

| Original case | Context | Exact measurement pointer | Failure boundary | Authored final operation |
| --- | --- | --- | --- | --- |
| `computed-binding-mutation` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/91` | Original outer eval line 4: definition/mutation prefix failed | Line 6, not entered |
| `computed-binding-mutation` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/1` | Original outer eval line 4: definition/mutation prefix failed | Line 6, not entered |
| `computed-binding-mutation` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/46` | Original outer eval line 4: definition/mutation prefix failed | Line 6, not entered |
| `computed-binding-mutation` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/1` | Original outer eval line 4: definition/mutation prefix failed | Line 6, not entered |
| `ensemble-prefix-map` | `tmm-RULE_INIT` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/12` | Original outer eval line 1: ensemble construction failed | Line 5, not entered |
| `ensemble-prefix-map` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/192` | Original outer eval line 1: ensemble construction failed | Line 5, not entered |
| `ensemble-prefix-map` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/408` | Original outer eval line 1: ensemble construction failed | Line 5, not entered |
| `keyed-variable-link` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/98` | Original outer eval line 8: preceding change invocation failed | Line 9, not entered |
| `keyed-variable-link` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/8` | Original outer eval line 8: preceding change invocation failed | Line 9, not entered |
| `keyed-variable-link` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/53` | Original outer eval line 8: preceding change invocation failed | Line 9, not entered |
| `keyed-variable-link` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/8` | Original outer eval line 8: preceding change invocation failed | Line 9, not entered |
| `oo-prefix-deleted-near-miss` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/224` | Original outer eval line 1: class construction failed | Line 9, not entered |
| `oo-prefix-deleted-near-miss` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/536` | Original outer eval line 1: class construction failed | Line 9, not entered |
| `oo-self-prefix` | `tmm-RULE_INIT` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/48` | Original outer eval line 1: class construction failed | Line 6, not entered |
| `oo-self-prefix` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/228` | Original outer eval line 1: class construction failed | Line 6, not entered |
| `oo-self-prefix` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/552` | Original outer eval line 1: class construction failed | Line 6, not entered |
| `tcltest-shared-lifecycle` | `tmsh-cli-script` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/104` | Original outer eval line 1: package initialisation failed | Line 14, not entered |
| `tcltest-shared-lifecycle` | `iapp-implementation` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/14` | Original outer eval line 1: package initialisation failed | Line 14, not entered |
| `tcltest-shared-lifecycle` | `tmm-RULE_INIT` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/56` | Original outer eval line 1: package initialisation failed | Line 14, not entered |
| `tcltest-shared-lifecycle` | `tmm-procedure` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/236` | Original outer eval line 1: package initialisation failed | Line 14, not entered |
| `tcltest-shared-lifecycle` | `tmm-traffic-event` | [nh20261012exact2/records.json](nh20261012exact2/records.json) `/584` | Original outer eval line 1: package initialisation failed | Line 14, not entered |
| `tcltest-shared-lifecycle` | `icall` | [nh20261012exact1/records.json](nh20261012exact1/records.json) `/59` | Original outer eval line 1: package initialisation failed | Line 14, not entered |
| `tcltest-shared-lifecycle` | `iapp-apl` | [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) `/14` | Original outer eval line 1: package initialisation failed | Line 14, not entered |

Four computed-binding mutation rows fail on `rename ::nh::emit ::nh::old` before the replacement definition and final `$head`. Four keyed variable-link rows fail on the preceding `::nh::change` invocation before the final variable read. Three ensemble rows fail while constructing the ensemble inside the initial namespace-eval command. Five TclOO rows fail on the initial class-definition command. Seven tcltest lifecycle rows fail on initial package require: the hosted/APL package initializer raises its missing `debug` variable, while the three TMM contexts cannot find the package. These are captured setup/prefix failures; generic code 1 does not establish an unsupported final lookup.

[reviewed-unreached.json](nh20261012unreached1/reviewed-unreached.json) retains all 23 exact identities, original/actual source hashes, complete semantic error fields, raw chunk hashes and precise final-operation boundaries. The JSON is deliberately separate from the positive reachability validator's input: none of its reviewed booleans supports promotion. Existing sealed captures, matrix and central manifest were not modified.

| Retained measurement bundle | records.json SHA-256 |
| --- | --- |
| [nh20261012exact1/records.json](nh20261012exact1/records.json) | `7635625d658d6d823bd806fe1a869b65388f1e26f359f3ae4941155e6e24c7da` |
| [nh20261012aplexact1/records.json](nh20261012aplexact1/records.json) | `60464454f8cfb9fa852354e129178c62913d728ec64fb524013281f357c2a413` |
| [nh20261012exact2/records.json](nh20261012exact2/records.json) | `cb2edaf2bdcbb1f4d17571ae54b46a03860fb737e19c2863b49e681ca86f91ba` |
