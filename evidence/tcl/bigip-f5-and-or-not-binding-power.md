# Binding power and short-circuit of iRule `and`, `or` and `not`

## Question

Do the iRule word operators `and`, `or` and `not` bind exactly like `&&`, `||` and `!` (including mixed with the symbolic forms and with `?:`), and do `and`/`or` short-circuit? Each bare expression is followed by its two explicit groupings. Scope: TMM `expr` with braced literal expressions in `RULE_INIT`.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_oplogic`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set c [catch {expr {1 or 0 and 0}} v]
log local0. "TCLLSP_OQ|oplogic|o1|$c|$v"
set c [catch {expr {(1 or 0) and 0}} v]
log local0. "TCLLSP_OQ|oplogic|o1a|$c|$v"
set c [catch {expr {1 or (0 and 0)}} v]
log local0. "TCLLSP_OQ|oplogic|o1b|$c|$v"
set c [catch {expr {0 and 0 == 0}} v]
log local0. "TCLLSP_OQ|oplogic|o2|$c|$v"
set c [catch {expr {(0 and 0) == 0}} v]
log local0. "TCLLSP_OQ|oplogic|o2a|$c|$v"
set c [catch {expr {0 and (0 == 0)}} v]
log local0. "TCLLSP_OQ|oplogic|o2b|$c|$v"
set c [catch {expr {1 or 0 && 0}} v]
log local0. "TCLLSP_OQ|oplogic|o3|$c|$v"
set c [catch {expr {(1 or 0) && 0}} v]
log local0. "TCLLSP_OQ|oplogic|o3a|$c|$v"
set c [catch {expr {1 or (0 && 0)}} v]
log local0. "TCLLSP_OQ|oplogic|o3b|$c|$v"
set c [catch {expr {1 || 0 and 0}} v]
log local0. "TCLLSP_OQ|oplogic|o4|$c|$v"
set c [catch {expr {(1 || 0) and 0}} v]
log local0. "TCLLSP_OQ|oplogic|o4a|$c|$v"
set c [catch {expr {1 || (0 and 0)}} v]
log local0. "TCLLSP_OQ|oplogic|o4b|$c|$v"
set c [catch {expr {0 or 1 ? 5 : 6}} v]
log local0. "TCLLSP_OQ|oplogic|o5|$c|$v"
set c [catch {expr {(0 or 1) ? 5 : 6}} v]
log local0. "TCLLSP_OQ|oplogic|o5a|$c|$v"
set c [catch {expr {0 or (1 ? 5 : 6)}} v]
log local0. "TCLLSP_OQ|oplogic|o5b|$c|$v"
set c [catch {expr {0 or 2}} v]
log local0. "TCLLSP_OQ|oplogic|o7|$c|$v"
set c [catch {expr {0 and [error side]}} v]
log local0. "TCLLSP_OQ|oplogic|o8|$c|$v"
set c [catch {expr {1 or [error side]}} v]
log local0. "TCLLSP_OQ|oplogic|o9|$c|$v"
set c [catch {expr {not 2 == 1}} v]
log local0. "TCLLSP_OQ|oplogic|n1|$c|$v"
set c [catch {expr {(not 2) == 1}} v]
log local0. "TCLLSP_OQ|oplogic|n1a|$c|$v"
set c [catch {expr {not (2 == 1)}} v]
log local0. "TCLLSP_OQ|oplogic|n1b|$c|$v"
set c [catch {expr {not 0 + 1}} v]
log local0. "TCLLSP_OQ|oplogic|n2|$c|$v"
set c [catch {expr {(not 0) + 1}} v]
log local0. "TCLLSP_OQ|oplogic|n2a|$c|$v"
set c [catch {expr {not (0 + 1)}} v]
log local0. "TCLLSP_OQ|oplogic|n2b|$c|$v"
set c [catch {expr {not 10 contains 0}} v]
log local0. "TCLLSP_OQ|oplogic|n3|$c|$v"
set c [catch {expr {(not 10) contains 0}} v]
log local0. "TCLLSP_OQ|oplogic|n3a|$c|$v"
set c [catch {expr {not (10 contains 0)}} v]
log local0. "TCLLSP_OQ|oplogic|n3b|$c|$v"
set c [catch {expr {!2 == 1}} v]
log local0. "TCLLSP_OQ|oplogic|n4|$c|$v"
set c [catch {expr {not 1 and 0}} v]
log local0. "TCLLSP_OQ|oplogic|n5|$c|$v"
set c [catch {expr {(not 1) and 0}} v]
log local0. "TCLLSP_OQ|oplogic|n5a|$c|$v"
set c [catch {expr {not (1 and 0)}} v]
log local0. "TCLLSP_OQ|oplogic|n5b|$c|$v"
set c [catch {expr {not 5}} v]
log local0. "TCLLSP_OQ|oplogic|n6|$c|$v"
set c [catch {expr {not "abc"}} v]
log local0. "TCLLSP_OQ|oplogic|n7|$c|$v"
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
LOG 4x n1|0|0
LOG 4x n1a|0|0
LOG 4x n1b|0|1
LOG 4x n2|0|2
LOG 4x n2a|0|2
LOG 4x n2b|0|0
LOG 4x n3|0|1
LOG 4x n3a|0|1
LOG 4x n3b|0|0
LOG 4x n4|0|0
LOG 4x n5|0|0
LOG 4x n5a|0|0
LOG 4x n5b|0|1
LOG 4x n6|0|0
LOG 4x n7|1|can't use non-numeric string as operand of "!"
LOG 4x o1|0|1
LOG 4x o1a|0|0
LOG 4x o1b|0|1
LOG 4x o2|0|0
LOG 4x o2a|0|1
LOG 4x o2b|0|0
LOG 4x o3|0|1
LOG 4x o3a|0|0
LOG 4x o3b|0|1
LOG 4x o4|0|1
LOG 4x o4a|0|0
LOG 4x o4b|0|1
LOG 4x o5|0|5
LOG 4x o5a|0|5
LOG 4x o5b|0|1
LOG 4x o7|0|1
LOG 4x o8|0|0
LOG 4x o9|0|1
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
