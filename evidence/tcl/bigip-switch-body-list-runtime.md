# Is a `switch` pattern/body list split with the iRule word break at runtime?

## Question

When the `switch` pattern/body list is a runtime value (so the rule loader never sees it), does TMM split it with stock Tcl list rules or with the iRule implicit word break after a closing brace or quote (R1/R2)? `llength` of the same value is the list-parser control.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_swdyn`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set b [binary format H* 617b736574207220417d]
set r unset
set l [catch {llength $b} n]
set c [catch {switch a $b} v]
log local0. "TCLLSP_OQ|swdyn|adjacent|llength=$l:$n|switch=$c:$v|r=$r"
set b [binary format H* 2261227b736574207220417d]
set r unset
set l [catch {llength $b} n]
set c [catch {switch a $b} v]
log local0. "TCLLSP_OQ|swdyn|quoted|llength=$l:$n|switch=$c:$v|r=$r"
set b [binary format H* 7b617d7b736574207220417d]
set r unset
set l [catch {llength $b} n]
set c [catch {switch a $b} v]
log local0. "TCLLSP_OQ|swdyn|braced|llength=$l:$n|switch=$c:$v|r=$r"
set b [binary format H* 7b787d207b736574207220587d207b64656661756c747d7b736574207220447d]
set r unset
set l [catch {llength $b} n]
set c [catch {switch a $b} v]
log local0. "TCLLSP_OQ|swdyn|braced_default|llength=$l:$n|switch=$c:$v|r=$r"
set b [binary format H* 61207b736574207220417d]
set r unset
set l [catch {llength $b} n]
set c [catch {switch a $b} v]
log local0. "TCLLSP_OQ|swdyn|ctl|llength=$l:$n|switch=$c:$v|r=$r"
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
LOG 4x adjacent|llength=0:3|switch=1:extra switch pattern with no body|r=unset
LOG 4x braced_default|llength=1:list element in braces followed by "{set" instead of space|switch=1:list element in braces followed by "{set" instead of space|r=unset
LOG 4x braced|llength=1:list element in braces followed by "{set" instead of space|switch=1:list element in braces followed by "{set" instead of space|r=unset
LOG 4x ctl|llength=0:2|switch=0:A|r=A
LOG 4x quoted|llength=1:list element in quotes followed by "{set" instead of space|switch=1:list element in quotes followed by "{set" instead of space|r=unset
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
