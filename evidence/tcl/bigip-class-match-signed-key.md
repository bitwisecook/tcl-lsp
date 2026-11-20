# Integer data-group lookups with a signed key

## Question

Does an integer data group holding 129 match the signed form -127 of the same byte, and how do `class match` and `class lookup` parse a leading negative key?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sgclass`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Supporting objects in the same file: ltm data-group internal tcllsp_oq_sgclass_dg { type integer records { 129 { data unsigned } } }. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `curl -s -m 5 -w ' http=%{http_code} exit=' http://192.168.9.24:19709/ ; echo $?` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when HTTP_REQUEST {
set c [catch {class match -127 equals tcllsp_oq_sgclass_dg} v]
log local0. "TCLLSP_OQ|sgclass|match_neg|$c|$v"
set c [catch {class match -- -127 equals tcllsp_oq_sgclass_dg} v]
log local0. "TCLLSP_OQ|sgclass|match_dd_neg|$c|$v"
set c [catch {class match -- 129 equals tcllsp_oq_sgclass_dg} v]
log local0. "TCLLSP_OQ|sgclass|match_dd_pos|$c|$v"
set c [catch {class match -- [expr {-127 & 0xff}] equals tcllsp_oq_sgclass_dg} v]
log local0. "TCLLSP_OQ|sgclass|match_dd_masked|$c|$v"
set c [catch {class lookup -127 tcllsp_oq_sgclass_dg} v]
log local0. "TCLLSP_OQ|sgclass|lookup_neg|$c|$v"
set c [catch {class lookup 129 tcllsp_oq_sgclass_dg} v]
log local0. "TCLLSP_OQ|sgclass|lookup_pos|$c|$v"
HTTP::respond 200 content "ok\n" Connection close
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
LOAD ACCEPT: (none)
TRAFFIC ok  http=200 exit=0
LOG 1x lookup_neg|0|
LOG 1x lookup_pos|0|unsigned
LOG 1x match_dd_masked|0|1
LOG 1x match_dd_neg|0|0
LOG 1x match_dd_pos|0|1
LOG 1x match_neg|1|bad option "-127": must be -all, -index, -element, -list, -name, or -value
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
