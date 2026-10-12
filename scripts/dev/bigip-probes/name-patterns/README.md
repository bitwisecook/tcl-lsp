# Name-handling appliance captures

Run these scripts from a checkout containing the retained questions. Configure
`BIGIP_PROBE_SSH_TARGET` for appliance execution. Optionally set
`BIGIP_PROBE_SSH_IDENTITY` and `BIGIP_PROBE_SSH_CONTROL_PATH`; the default control
socket is `/tmp/tcl_lsp_probe_%C`. A persistent SSH master reduces repeated
authentication. Receipts omit the target, identity and control socket paths.

```sh
python3 scripts/dev/bigip-probes/name-patterns/capture.py --run UNIQUE --contexts tmsh-cli-script
python3 scripts/dev/bigip-probes/name-patterns/campaign.py --run UNIQUE --kind patterns --contexts stock-8.4,stock-8.5
python3 scripts/dev/bigip-probes/name-patterns/campaign.py --run UNIQUE --kind broader --contexts tmm-procedure,tmm-traffic-event
python3 scripts/dev/bigip-probes/name-patterns/literal.py --run UNIQUE
python3 scripts/dev/bigip-probes/name-patterns/appliance.py --run UNIQUE
```

Use a new alphanumeric run ID of 6–24 characters for every capture. A run snapshots
its executed Python sources, Tcl, merged configuration, invocation, provider facts
and raw output. Keep those files unchanged after review and sealing. The parser
retains incomplete chunks, duplicate records and binary representations explicitly;
it does not silently join ambiguous repeated invocations or assume UTF-8.

`campaign.py --resume-from RUN_DIRECTORY` skips only case groups with retained
load receipts and proven object cleanup. It writes the resumed cases into a new
run. Rejected loads and failed setups remain distinguishable from a reached lookup.
Stock processes, CLI scripts, iApp implementations, iCall handlers and TMM events
remain separate providers. Renamed hosted inputs are separate adaptation variants.

The traffic driver owns one temporary HTTP virtual server on port 19912 and checks
for destination conflicts before applying configuration, retaining only the
collision or absence result. Set `BIGIP_PROBE_ADDRESS` to a dedicated IPv4
destination; its default is the documentation address `192.0.2.10`. Saved receipts
substitute that documentation address for the configured destination and identify
these redactions. Input hashes describe the actual executed input. Full appliance
hardware and unrelated virtual inventories are omitted. Run only one traffic
campaign at a time. Every request carries its invocation ID; responses and logs
retain actual TMM units. A response from one unit proves that unit only.
Runtime log receipts retain each complete `NHO|` line without the device log prefix.

APL requires the real Configuration Utility form renderer:

```sh
python3 scripts/dev/bigip-probes/name-patterns/apl.py --run UNIQUE --phase prepare
# Select the owned template in Chrome's Application Services Create form.
# Capture returned choice options, the selected owned template DOM and action metadata.
python3 scripts/dev/bigip-probes/name-patterns/apl.py --run UNIQUE --phase finish
```

APL template creation and iApp `execute-action definition` do not establish APL
callback execution. Leave the prepared template owned until the form has rendered
and its results have been retained, then finish to prove template removal.
`browser_receipt.py` retains probe option rows and the selected template, omits
device chrome and removes the unsealed browser screenshot before reporting.

No probe saves configuration. Normal jobs prove absence before creation, remove
only objects whose absence was established, and retain after-deletion receipts.
An interrupted controller requires an independent recovery check. Remote staging
files must be removed by the owner after local evidence retrieval.

Generate derived reports with `report.py`. Register only independently reviewed,
reached observations through the existing name-resolution proof authority;
`register.py` prepares records without mutating that authority. Run the central
proof gate after registration and report existing failures separately.
