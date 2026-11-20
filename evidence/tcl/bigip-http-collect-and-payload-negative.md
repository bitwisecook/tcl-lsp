# `HTTP::collect` and `HTTP::payload` with -1

## Question

What do `HTTP::collect -1` and `HTTP::payload -1` do at load and at runtime on a request carrying an 8-byte body?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sghttp`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `curl -s -m 5 -d abcdefgh -w ' http=%{http_code} exit=' http://192.168.9.24:19710/ ; echo $?` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when HTTP_REQUEST {
set c [catch {HTTP::collect -1} v]
log local0. "TCLLSP_OQ|sghttp|collect|$c|$v"
set c [catch {string length [HTTP::payload -1]} v]
log local0. "TCLLSP_OQ|sghttp|payload|$c|$v"
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
LOAD WARN: There were warnings: /Common/tcllsp_oq_sghttp:4: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "invalid argument -1; expected syntax spec: (((LENGTH|OFFSET)?)|length|rechunk|replace|unchunk) "129 16][HTTP::payload -1]
TRAFFIC ok  http=200 exit=0
LOG 1x collect|0|
LOG 1x payload|1| Neither <offset> nor <length> may be negative     invoked from within "HTTP::payload -1"
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
