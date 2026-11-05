# BIG-IP evidence fixtures

A fixture is the harness shared by several BIG-IP evidence files: how the
probe is deployed, the script template, and values every case shares. A case
file's `## Script` section links one fixture and gives only what that case
changes, usually `nh_setup`, `nh_operation` and `nh_cleanup` as decoded Tcl.

The template is a `tcl-visible` block with two kinds of slot:

- `⟦HEX:name⟧` takes the lowercase hexadecimal form of the named value. The
  harness passes it through `binary format H*`, so the case shows the decoded
  Tcl.
- `⟦PARAM:name⟧` takes the exact bytes of the named value, such as the run or
  case identifier.

Values come from the fixture's `## Constants` section and the case's own
`### name` blocks or `- name: \`value\`` lines. `extract-tcl-evidence.py`
composes them into the exact bytes that ran:

```sh
python3 scripts/dev/extract-tcl-evidence.py evidence/tcl/bigip-apl-alias-deleted-lookup.md -o -
```

Results report the decoded `NHO|run|case|context|phase|code|unit|invocation|field|…`
rows. Raw rows are omitted when every field is decoded below them. Units or
invocations with identical results share one heading.

All probes ran on BIG-IP 21.1.0.1 unless the case records otherwise. Every
probe creates uniquely named objects, checks they are absent first, removes
them afterwards and never saves the configuration. See
[the appliance notes](../../../scripts/dev/bigip-probes/README.md#appliance-notes)
for the loader limitations behind these steps.

## tmm-traffic-event

Load the rule with `tmsh load sys config merge file <rule>.conf` and attach it
to a virtual server with an HTTP profile. Send one request to trigger
`HTTP_REQUEST`. Collect the `NHO|` lines the rule writes to `/var/log/ltm`.
Each TMM logs its own unit; a request reaches only the TMM that serves it.

## tmm-rule-init

Same as the traffic event, but the probe body runs in `RULE_INIT` when the rule
loads. Every TMM runs it; the log tag (`tmm`, `tmm1`, …) identifies the unit.
Do not infer how many times it ran from the number of log lines: syslog
collapses identical lines.

## tmm-procedure

The probe body is an iRule `proc`. A `RULE_INIT` or `HTTP_REQUEST` event in
the same rule calls it, and that event is named in the recorded channel.

## iapp-implementation

The probe is an iApp template's implementation section. A config merge creates
the application service without running it, so force a run with
`tmsh modify sys application service <name> execute-action definition`.
Results are logged with `tmsh::log err`, because info-level messages never
reach `/var/log/ltm`.

## iapp-apl

The probe runs as an APL presentation callback, a template choice whose
`tcl` body computes the option values. Render the template in the browser to
execute it, which does not execute the implementation. The result is the DOM
option values, one `NHO|` row per value, not process output.

## icall

The probe body is an iCall script run once by a handler. Results are logged
like the iApp implementation.

## tmsh-cli-script

The probe body is a `cli script` run with `tmsh run cli script <name>`.
`tcl_patchLevel` does not exist in this context.

## stock-tclsh

The probe script is sent over SSH to the appliance's `/usr/bin/tclsh8.4` or
`/usr/bin/tclsh8.5` on standard input. Standard error can include SSH transport
messages.

## other-contexts

The case's recorded channel names the context. Deploy as the closest section
above describes.
