# Flow commands executed in `RULE_INIT`

## Question

The 2026-08-26 matrix recorded `HTTP::uri`, `HTTP::status`, `HTTP::respond`, `HTTP::collect`, `TCP::collect` and `TCP::payload` as accepted in `RULE_INIT` at load. What does the loader say about them now, and what happens when each actually runs there: error, empty result, or effect? Does the handler continue after a caught failure?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_riflow`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set c [catch {HTTP::uri} v]
log local0. "TCLLSP_OQ|riflow|HTTP::uri|$c|$v"
set c [catch {HTTP::status} v]
log local0. "TCLLSP_OQ|riflow|HTTP::status|$c|$v"
set c [catch {HTTP::respond 200 content "x"} v]
log local0. "TCLLSP_OQ|riflow|HTTP::respond|$c|$v"
set c [catch {HTTP::collect 1} v]
log local0. "TCLLSP_OQ|riflow|HTTP::collect|$c|$v"
set c [catch {TCP::collect 1} v]
log local0. "TCLLSP_OQ|riflow|TCP::collect|$c|$v"
set c [catch {TCP::payload} v]
log local0. "TCLLSP_OQ|riflow|TCP::payload|$c|$v"
log local0. "TCLLSP_OQ|riflow|reached-end"
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26, TMM

- Channel: rule load (mcpd) and TMM log lines.
- Exit status: Not recorded
- Standard output: Not recorded
- Standard error: Not recorded
- Recorded result:

```text-visible
LOAD WARN: There were warnings: /Common/tcllsp_oq_riflow:8: warning: [command is not valid in RULE_INIT (RULE_INIT)][HTTP::collect 1] /Common/tcllsp_oq_riflow:6: warning: [command is not valid in RULE_INIT (RULE_INIT)][HTTP::respond 200 content "x"] /Common/tcllsp_oq_riflow:4: warning: [command is not valid in RULE_INIT (RULE_INIT)][HTTP::status] /Common/tcllsp_oq_riflow:2: warning: [command is not valid in RULE_INIT (RULE_INIT)][HTTP::uri]
LOG 4x HTTP::collect|1|Invalid context (line 1)     invoked from within "HTTP::collect 1"
LOG 4x HTTP::respond|1|Operation not supported (line 1)     invoked from within "HTTP::respond 200 content "x""
LOG 4x HTTP::status|1|Invalid context (line 1)     invoked from within "HTTP::status"
LOG 4x HTTP::uri|1|Invalid context (line 1)     invoked from within "HTTP::uri"
LOG 4x reached-end
LOG 4x TCP::collect|1|Operation not supported (line 1)     invoked from within "TCP::collect 1"
LOG 4x TCP::payload|1|Operation not supported (line 1)     invoked from within "TCP::payload"
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
