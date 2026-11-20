# iRule word operators in a computed `expr` string

## Question

Are the iRule word operators also recognised when the expression is a runtime string rather than a braced literal the rule loader can see, and does the loader comment on it? This decides whether the analyser may treat a computed `expr` string as iRules expression syntax.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_opdyn`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set s [binary format H* 2261622220636f6e7461696e7320226122203d3d2031]
set c [catch {expr $s} v]
log local0. "TCLLSP_OQ|opdyn|contains|$c|$v"
set s [binary format H* 2261626322206d6174636865732022612a22]
set c [catch {expr $s} v]
log local0. "TCLLSP_OQ|opdyn|matches|$c|$v"
set s [binary format H* 31206f72203020616e642030]
set c [catch {expr $s} v]
log local0. "TCLLSP_OQ|opdyn|or_and|$c|$v"
set s [binary format H* 6e6f742032203d3d2031]
set c [catch {expr $s} v]
log local0. "TCLLSP_OQ|opdyn|not|$c|$v"
set s [binary format H* 22313222207374617274735f776974682031202b2031]
set c [catch {expr $s} v]
log local0. "TCLLSP_OQ|opdyn|starts_with_plus|$c|$v"
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
LOAD WARN: There were warnings: /Common/tcllsp_oq_opdyn:3: warning: [use curly braces to avoid double substitution][$s] /Common/tcllsp_oq_opdyn:6: warning: [use curly braces to avoid double substitution][$s] /Common/tcllsp_oq_opdyn:9: warning: [use curly braces to avoid double substitution][$s] /Common/tcllsp_oq_opdyn:12: warning: [use curly braces to avoid double substitution][$s] /Common/tcllsp_oq_opdyn:15: warning: [use curly braces to avoid double substitution][$s]
LOG 4x contains|0|1
LOG 4x matches|0|1
LOG 4x not|0|0
LOG 4x or_and|0|1
LOG 4x starts_with_plus|0|0
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
