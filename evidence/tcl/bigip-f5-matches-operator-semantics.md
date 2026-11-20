# What the iRule `matches` operator compares

## Question

Does the iRule word operator `matches` compare for string equality, glob-match its right operand, or regex-match it, and do `matches_glob` and `matches_regex` behave as their names say? The analyser's authored evaluator treated `matches` as equality, and the measured `"abcd" matches "bc"` = 0 could not tell equality from an anchored glob. This scopes TMM expression semantics only; it does not cover `class match` or LTM policy operators.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_opmatches`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set c [catch {expr {"abc" matches "abc"}} v]
log local0. "TCLLSP_OQ|opmatches|eq|$c|$v"
set c [catch {expr {"abcd" matches "bc"}} v]
log local0. "TCLLSP_OQ|opmatches|substring|$c|$v"
set c [catch {expr {"abcd" matches "ab"}} v]
log local0. "TCLLSP_OQ|opmatches|prefix|$c|$v"
set c [catch {expr {"abc" matches "a*"}} v]
log local0. "TCLLSP_OQ|opmatches|star|$c|$v"
set c [catch {expr {"a*" matches "a*"}} v]
log local0. "TCLLSP_OQ|opmatches|star_literal|$c|$v"
set c [catch {expr {"b" matches "\[abc\]"}} v]
log local0. "TCLLSP_OQ|opmatches|class|$c|$v"
set c [catch {expr {"abc" matches "a.c"}} v]
log local0. "TCLLSP_OQ|opmatches|dot|$c|$v"
set c [catch {expr {"abc" matches "a?c"}} v]
log local0. "TCLLSP_OQ|opmatches|question|$c|$v"
set c [catch {expr {"ABC" matches "a*"}} v]
log local0. "TCLLSP_OQ|opmatches|case|$c|$v"
set c [catch {expr {"abc" matches_glob "a*"}} v]
log local0. "TCLLSP_OQ|opmatches|glob_star|$c|$v"
set c [catch {expr {"abc" matches_glob "abc"}} v]
log local0. "TCLLSP_OQ|opmatches|glob_exact|$c|$v"
set c [catch {expr {"abc" matches_regex {^a.c$}}} v]
log local0. "TCLLSP_OQ|opmatches|regex_anchored|$c|$v"
set c [catch {expr {"xabcx" matches_regex {a.c}}} v]
log local0. "TCLLSP_OQ|opmatches|regex_unanchored|$c|$v"
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
LOG 4x case|0|0
LOG 4x class|0|1
LOG 4x dot|0|0
LOG 4x eq|0|1
LOG 4x glob_exact|0|1
LOG 4x glob_star|0|1
LOG 4x prefix|0|0
LOG 4x question|0|1
LOG 4x regex_anchored|0|1
LOG 4x regex_unanchored|0|1
LOG 4x star|0|1
LOG 4x star_literal|0|1
LOG 4x substring|0|0
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
