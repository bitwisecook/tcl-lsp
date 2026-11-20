# Binding power of the iRule string word operators

## Question

Where do `contains`, `starts_with`, `ends_with`, `equals`, `matches`, `matches_glob` and `matches_regex` bind relative to `==`, `<`, `+`, `&` and `&&`, and are they left-associative? Each bare expression is followed by its two explicit groupings; the bare result equals exactly one of them where the case discriminates. W154/W155 group these operators at the `==` level, which was unmeasured. Scope: TMM `expr` with braced literal expressions in `RULE_INIT`.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_opbind`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set c [catch {expr {1 & 3 contains 3}} v]
log local0. "TCLLSP_OQ|opbind|c1|$c|$v"
set c [catch {expr {(1 & 3) contains 3}} v]
log local0. "TCLLSP_OQ|opbind|c1a|$c|$v"
set c [catch {expr {1 & (3 contains 3)}} v]
log local0. "TCLLSP_OQ|opbind|c1b|$c|$v"
set c [catch {expr {"ab" contains "a" == 1}} v]
log local0. "TCLLSP_OQ|opbind|c2|$c|$v"
set c [catch {expr {("ab" contains "a") == 1}} v]
log local0. "TCLLSP_OQ|opbind|c2a|$c|$v"
set c [catch {expr {"ab" contains ("a" == 1)}} v]
log local0. "TCLLSP_OQ|opbind|c2b|$c|$v"
set c [catch {expr {1 == 2 contains 0}} v]
log local0. "TCLLSP_OQ|opbind|c3|$c|$v"
set c [catch {expr {(1 == 2) contains 0}} v]
log local0. "TCLLSP_OQ|opbind|c3a|$c|$v"
set c [catch {expr {1 == (2 contains 0)}} v]
log local0. "TCLLSP_OQ|opbind|c3b|$c|$v"
set c [catch {expr {"x0" contains 1 < 2}} v]
log local0. "TCLLSP_OQ|opbind|c4|$c|$v"
set c [catch {expr {("x0" contains 1) < 2}} v]
log local0. "TCLLSP_OQ|opbind|c4a|$c|$v"
set c [catch {expr {"x0" contains (1 < 2)}} v]
log local0. "TCLLSP_OQ|opbind|c4b|$c|$v"
set c [catch {expr {0 && 0 contains 0}} v]
log local0. "TCLLSP_OQ|opbind|c5|$c|$v"
set c [catch {expr {(0 && 0) contains 0}} v]
log local0. "TCLLSP_OQ|opbind|c5a|$c|$v"
set c [catch {expr {0 && (0 contains 0)}} v]
log local0. "TCLLSP_OQ|opbind|c5b|$c|$v"
set c [catch {expr {"12" starts_with 1 + 1}} v]
log local0. "TCLLSP_OQ|opbind|c6|$c|$v"
set c [catch {expr {("12" starts_with 1) + 1}} v]
log local0. "TCLLSP_OQ|opbind|c6a|$c|$v"
set c [catch {expr {"12" starts_with (1 + 1)}} v]
log local0. "TCLLSP_OQ|opbind|c6b|$c|$v"
set c [catch {expr {"21" ends_with 1 + 1}} v]
log local0. "TCLLSP_OQ|opbind|c7|$c|$v"
set c [catch {expr {("21" ends_with 1) + 1}} v]
log local0. "TCLLSP_OQ|opbind|c7a|$c|$v"
set c [catch {expr {"21" ends_with (1 + 1)}} v]
log local0. "TCLLSP_OQ|opbind|c7b|$c|$v"
set c [catch {expr {1 == 2 equals 0}} v]
log local0. "TCLLSP_OQ|opbind|c8|$c|$v"
set c [catch {expr {(1 == 2) equals 0}} v]
log local0. "TCLLSP_OQ|opbind|c8a|$c|$v"
set c [catch {expr {1 == (2 equals 0)}} v]
log local0. "TCLLSP_OQ|opbind|c8b|$c|$v"
set c [catch {expr {1 == 2 matches_glob 0}} v]
log local0. "TCLLSP_OQ|opbind|c9|$c|$v"
set c [catch {expr {(1 == 2) matches_glob 0}} v]
log local0. "TCLLSP_OQ|opbind|c9a|$c|$v"
set c [catch {expr {1 == (2 matches_glob 0)}} v]
log local0. "TCLLSP_OQ|opbind|c9b|$c|$v"
set c [catch {expr {1 == 2 matches_regex 0}} v]
log local0. "TCLLSP_OQ|opbind|c10|$c|$v"
set c [catch {expr {(1 == 2) matches_regex 0}} v]
log local0. "TCLLSP_OQ|opbind|c10a|$c|$v"
set c [catch {expr {1 == (2 matches_regex 0)}} v]
log local0. "TCLLSP_OQ|opbind|c10b|$c|$v"
set c [catch {expr {"abc" contains "b" contains 1}} v]
log local0. "TCLLSP_OQ|opbind|c11|$c|$v"
set c [catch {expr {("abc" contains "b") contains 1}} v]
log local0. "TCLLSP_OQ|opbind|c11a|$c|$v"
set c [catch {expr {"abc" contains ("b" contains 1)}} v]
log local0. "TCLLSP_OQ|opbind|c11b|$c|$v"
set c [catch {expr {1 == 2 matches 0}} v]
log local0. "TCLLSP_OQ|opbind|c12|$c|$v"
set c [catch {expr {(1 == 2) matches 0}} v]
log local0. "TCLLSP_OQ|opbind|c12a|$c|$v"
set c [catch {expr {1 == (2 matches 0)}} v]
log local0. "TCLLSP_OQ|opbind|c12b|$c|$v"
set c [catch {expr {"x0" starts_with 1 < 2}} v]
log local0. "TCLLSP_OQ|opbind|c13|$c|$v"
set c [catch {expr {("x0" starts_with 1) < 2}} v]
log local0. "TCLLSP_OQ|opbind|c13a|$c|$v"
set c [catch {expr {"x0" starts_with (1 < 2)}} v]
log local0. "TCLLSP_OQ|opbind|c13b|$c|$v"
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
LOG 4x c10|0|1
LOG 4x c1|0|1
LOG 4x c10a|0|1
LOG 4x c10b|0|0
LOG 4x c11|0|1
LOG 4x c11a|0|1
LOG 4x c11b|0|0
LOG 4x c12|0|1
LOG 4x c12a|0|1
LOG 4x c12b|0|0
LOG 4x c13|0|0
LOG 4x c13a|0|1
LOG 4x c13b|0|0
LOG 4x c1a|0|0
LOG 4x c1b|0|1
LOG 4x c2|0|1
LOG 4x c2a|0|1
LOG 4x c2b|0|0
LOG 4x c3|0|1
LOG 4x c3a|0|1
LOG 4x c3b|0|0
LOG 4x c4|0|0
LOG 4x c4a|0|1
LOG 4x c4b|0|0
LOG 4x c5|0|0
LOG 4x c5a|0|1
LOG 4x c5b|0|0
LOG 4x c6|0|0
LOG 4x c6a|0|2
LOG 4x c6b|0|0
LOG 4x c7|0|0
LOG 4x c7a|0|2
LOG 4x c7b|0|0
LOG 4x c8|0|1
LOG 4x c8a|0|1
LOG 4x c8b|0|0
LOG 4x c9|0|1
LOG 4x c9a|0|1
LOG 4x c9b|0|0
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
