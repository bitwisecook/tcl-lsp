# `TCP::collect -1`

## Question

What does `TCP::collect -1` do at load and at runtime: does the client data event fire, and with how many bytes, when the client sends 8 bytes?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sgtcpcollect`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `printf abcdefgh | nc -w 5 192.168.9.24 19711; echo " exit=$?"` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when CLIENT_ACCEPTED {
set c [catch {TCP::collect -1} v]
log local0. "TCLLSP_OQ|sgtcpcollect|CLIENT_ACCEPTED|$c|$v"
}
when CLIENT_DATA {
log local0. "TCLLSP_OQ|sgtcpcollect|CLIENT_DATA|[TCP::payload length]"
TCP::respond "seen\n"
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
LOAD WARN: There were warnings: /Common/tcllsp_oq_sgtcpcollect:2: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "invalid argument -1; expected syntax spec:"37 15][TCP::collect -1]
TRAFFIC seen  exit=0
LOG 1x CLIENT_ACCEPTED|0|
LOG 1x CLIENT_DATA|8
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
