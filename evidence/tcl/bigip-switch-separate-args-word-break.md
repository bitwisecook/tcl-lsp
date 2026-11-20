# `switch` patterns and bodies as separate words with the iRule word break

## Question

When the same patterns and bodies are separate `switch` words rather than one list, does the script-level implicit word break (R1/R2) split `"a"{...}`, `{a}{...}` and `{default}{...}` into pattern and body?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_swargs`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set r unset
set c [catch {switch a "a"{set r A}} v]
log local0. "TCLLSP_OQ|swargs|quoted|$c|$v|r=$r"
set r unset
set c [catch {switch a {a}{set r A}} v]
log local0. "TCLLSP_OQ|swargs|braced|$c|$v|r=$r"
set r unset
set c [catch {switch a {x} {set r X} {default}{set r D}} v]
log local0. "TCLLSP_OQ|swargs|braced_default|$c|$v|r=$r"
set r unset
set c [catch {switch a a {set r A}} v]
log local0. "TCLLSP_OQ|swargs|ctl|$c|$v|r=$r"
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
LOG 4x braced|0|A|r=A
LOG 4x braced_default|0|D|r=D
LOG 4x ctl|0|A|r=A
LOG 4x quoted|0|A|r=A
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
