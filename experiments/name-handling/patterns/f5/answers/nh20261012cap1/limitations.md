# Capability capture limits: nh20261012cap1

These retained inputs are capability controls, not answers to the portable
corpus/Wiki questions. Contexts, setup and lookup remain separate. A successful
configuration load or driver exit does not establish complete guest execution.

The following counts include only `NHO` rows whose context field exactly matches
the directory. The `runtime-logs.stdout` files were selected by run tag and can
also contain earlier contexts; those other rows supply no answer here.

| Context | Exact-context marker rows | Measured capture boundary |
| --- | ---: | --- |
| `stock-8.4` | 106 | All 17 setup and lookup controls appear in `runtime.stdout`; this is an appliance stock interpreter control. |
| `stock-8.5` | 102 | All 17 setup and lookup controls appear in `runtime.stdout`; this is an appliance stock interpreter control. |
| `tmsh-cli-script` | 110 | All 17 setup and lookup controls appear in `invoke.stdout`. |
| `icall` | 110 | All 17 setup and lookup controls appear in `runtime-logs.stdout`. |
| `iapp-implementation` | 5 | Only the version control's three setup fields and lookup result/errorcode were captured. The lookup errorinfo and subsequent controls are absent. |
| `tmm-RULE_INIT` | 162 | Units `0:0`, `0:1`, `0:2`, `0:3` appear, but later case/field/chunk coverage is incomplete. |
| `tmm-procedure` | 181 | Units `0:0`, `0:1`, `0:2`, `0:3` appear, but case/field/chunk coverage is incomplete. The procedure was invoked from `RULE_INIT`. |
| `tmm-traffic-event` | 13,568 | 128 retained response bodies identify only unit `0:0`; this is not four-unit coverage. |
| `iapp-apl` | 0 | Template/service load and implementation-action invocation returned zero, but no presentation callback was captured. APL is unreached. |

Incomplete logging cannot distinguish unrecorded execution from an operation
that never ran. These filtered transcripts do not establish the cause of the
missing rows. An absent row must not be promoted to an unsupported capability or
successful lookup. Each multi-chunk field requires its complete indexed set.

The runner uses one run-scoped dynamic namespace and reuses command names across
controls. It records no per-case runtime teardown. Repeated traffic may observe
prior interpreter state. Configuration-object deletion and absence checks do
not establish cleanup of dynamically created namespaces, commands, aliases or
traces. The first capture's markers also lack a distinct invocation identifier;
repeat reconstruction must not invent one.

For an error, missing `::errorCode`/`::errorInfo` slots are emitted as empty
fields by this wrapper. Their availability is therefore not established by an
empty field. Complete return options were not captured. Results identify either
UTF-8 conversion or `binary-scan-tcl-value`; those encodings must remain distinct.

The applied `input.tcl`, `applied.conf`, `invocation.sh`, command exit files,
raw output and `after-*.stdout`/`after-*.stderr` remain the evidence. The
`save-status.txt` files record that configuration was not saved. Appliance
product/build/hotfix and complete runtime cleanup require their independent
retained receipts; the Tcl version control and these limitations supply neither.
