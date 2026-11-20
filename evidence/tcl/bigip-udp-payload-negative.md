# `UDP::payload` offsets and lengths with -1

## Question

With an 8-byte datagram, what do `UDP::payload -1 4` and `UDP::payload 0 -1` do at load and at runtime, compared with `UDP::payload 0 4`?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sgudp`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `printf abcdefgh | nc -u -w 1 192.168.9.24 19714; echo " exit=$?"` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when CLIENT_DATA {
set c [catch {string length [UDP::payload 0 4]} v]
log local0. "TCLLSP_OQ|sgudp|ctl_0_4|$c|$v"
set c [catch {string length [UDP::payload -1 4]} v]
log local0. "TCLLSP_OQ|sgudp|off_neg|$c|$v"
set c [catch {string length [UDP::payload 0 -1]} v]
log local0. "TCLLSP_OQ|sgudp|len_neg|$c|$v"
UDP::drop
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
LOAD WARN: There were warnings: /Common/tcllsp_oq_sgudp:4: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "invalid argument -1; expected syntax spec: (((LENGTH|OFFSET)?)|length|replace) "143 17][UDP::payload -1 4] /Common/tcllsp_oq_sgudp:6: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "unexpected token(s): ' -1'"239 17][UDP::payload 0 -1]
TRAFFIC exit=0
LOG 1x ctl_0_4|0|4
LOG 1x len_neg|0|8
LOG 1x off_neg|1|     while executing "UDP::payload -1 4"
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
