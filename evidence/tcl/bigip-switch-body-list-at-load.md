# Literal `switch` pattern/body lists that need the iRule word break

## Question

Does the rule loader parse a literal braced `switch` pattern/body list with stock list rules or with the iRule implicit word break? Each list here is valid only if the word break applies.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_swlit`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
switch a {"a"{set r A}}
switch a {{a}{set r A}}
switch a {{x} {set r X} {default}{set r D}}
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
LOAD REJECT: 01070151:3: Rule [/Common/tcllsp_oq_swlit] error: /Common/tcllsp_oq_swlit:2: error: [invalid list: list element in quotes followed by "{set" instead of space]["] /Common/tcllsp_oq_swlit:3: error: [invalid list: list element in braces followed by "{set" instead of space][{] /Common/tcllsp_oq_swlit:4: error: [invalid list: list element in braces followed by "{set" instead of space][{]  Unexpected Error: Loading configuration process failed.
LOG (no tagged lines)
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
