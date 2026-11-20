# `node` and `pool ... member` with a negative port at rule load

## Question

Does the rule loader accept a literal negative port for `node` and `pool <pool> member`? A negative port is what a signed 16-bit scan of a port above 32767 yields.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sgport`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Supporting objects in the same file: ltm pool tcllsp_oq_sgport_pool { members { 10.0.0.1:80 { } } }. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when HTTP_REQUEST {
node 10.0.0.1 -1
pool tcllsp_oq_sgport_pool member 10.0.0.1 -1
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
LOAD REJECT: 01070151:3: Rule [/Common/tcllsp_oq_sgport] error: /Common/tcllsp_oq_sgport:2: error: ["invalid IP port:-1"][node 10.0.0.1 -1] /Common/tcllsp_oq_sgport:3: error: ["unexpected token(s): ' -1'"][pool tcllsp_oq_sgport_pool member 10.0.0.1 -1]  Unexpected Error: Loading configuration process failed.
LOG (no tagged lines)
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
