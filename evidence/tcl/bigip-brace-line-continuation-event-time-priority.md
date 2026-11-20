# Brace-line continuation (N1) at event time in a prioritised rule

## Question

Does the brace-line continuation (N1) still hold when the code runs at event time on a live request, inside a rule with a rule-level `priority` and a `when ... priority` handler, and inside a proc called from it?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_n1event`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `curl -s -m 5 -w ' http=%{http_code} exit=' http://192.168.9.24:19719/ ; echo $?` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
priority 100
proc tcllsp_oq_p {a}
{
return "p-$a"
}
when HTTP_REQUEST priority 900 {
set n 0
while {$n < 30}
{
set n [expr {$n + 30}]
}
set l [list a b
{c}]
set p [call tcllsp_oq_p x]
log local0. "TCLLSP_OQ|n1event|HTTP_REQUEST|n=$n|l=$l|p=$p"
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
LOG 1x HTTP_REQUEST|n=30|l=a b c|p=p-x
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
