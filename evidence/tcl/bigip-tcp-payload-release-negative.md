# `TCP::payload` offsets and lengths and `TCP::release` with -1

## Question

With 8 collected bytes, what do `TCP::payload -1 4`, `TCP::payload 0 -1` and `TCP::release -1` do at load and at runtime, compared with `TCP::payload 0 4`?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sgtcppayload`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `printf abcdefgh | nc -w 5 192.168.9.24 19712; echo " exit=$?"` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when CLIENT_ACCEPTED {
TCP::collect 8
}
when CLIENT_DATA {
set c [catch {string length [TCP::payload 0 4]} v]
log local0. "TCLLSP_OQ|sgtcppayload|ctl_0_4|$c|$v"
set c [catch {string length [TCP::payload -1 4]} v]
log local0. "TCLLSP_OQ|sgtcppayload|off_neg|$c|$v"
set c [catch {string length [TCP::payload 0 -1]} v]
log local0. "TCLLSP_OQ|sgtcppayload|len_neg|$c|$v"
set c [catch {TCP::release -1} v]
log local0. "TCLLSP_OQ|sgtcppayload|release_neg|$c|$v"
log local0. "TCLLSP_OQ|sgtcppayload|after_release|[TCP::payload length]"
TCP::respond "done\n"
TCP::close
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
LOAD WARN: There were warnings: /Common/tcllsp_oq_sgtcppayload:7: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "invalid argument -1; expected syntax spec: (((LENGTH|OFFSET)?)|length|replace) "190 17][TCP::payload -1 4] /Common/tcllsp_oq_sgtcppayload:9: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "unexpected token(s): ' -1'"293 17][TCP::payload 0 -1] /Common/tcllsp_oq_sgtcppayload:11: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "invalid argument -1; expected syntax spec:"381 15][TCP::release -1]
TRAFFIC done  exit=0
LOG 1x after_release|0
LOG 1x ctl_0_4|0|4
LOG 1x len_neg|0|8
LOG 1x off_neg|1|     while executing "TCP::payload -1 4"
LOG 1x release_neg|0|8
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
