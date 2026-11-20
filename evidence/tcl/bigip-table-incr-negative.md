# `table incr` with a negative increment

## Question

What does `table incr` store and return for a negative increment, compared with its positive twin? A counter fed a signed scanned value would take this path.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sgtable`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `curl -s -m 5 -w ' http=%{http_code} exit=' http://192.168.9.24:19708/ ; echo $?` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when HTTP_REQUEST {
set c [catch {table set tcllsp_oq_sgtable_k 0 indef indef; set r [table incr tcllsp_oq_sgtable_k -32768]; lappend r [table lookup tcllsp_oq_sgtable_k]; table delete tcllsp_oq_sgtable_k; set r} v]
log local0. "TCLLSP_OQ|sgtable|neg|$c|$v"
set c [catch {table set tcllsp_oq_sgtable_k 0 indef indef; set r [table incr tcllsp_oq_sgtable_k 32768]; lappend r [table lookup tcllsp_oq_sgtable_k]; table delete tcllsp_oq_sgtable_k; set r} v]
log local0. "TCLLSP_OQ|sgtable|pos|$c|$v"
set c [catch {table set tcllsp_oq_sgtable_k 5 indef indef; set r [table incr tcllsp_oq_sgtable_k -1]; table delete tcllsp_oq_sgtable_k; set r} v]
log local0. "TCLLSP_OQ|sgtable|dec_from_5|$c|$v"
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
LOG 1x dec_from_5|0|4
LOG 1x neg|0|18446744073709518848 18446744073709518848
LOG 1x pos|0|32768 32768
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
