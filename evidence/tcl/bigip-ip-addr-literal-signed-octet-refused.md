# `IP::addr` with a literal negative octet at rule load

## Question

Does the rule loader accept `IP::addr` given a literal dotted quad with negative octets, as a signed `binary scan` value formatted with `%d` would produce? The runtime behaviour with a computed address is in the TMM signed-value case; this one records the load-time verdict only.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sgipaddr`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set r1 [IP::addr 192.-64.-1.1 equals 192.192.255.1]
set r2 [IP::addr 192.-64.-1.1 mask 255.255.255.0]
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
LOAD REJECT: 01070151:3: Rule [/Common/tcllsp_oq_sgipaddr] error: /Common/tcllsp_oq_sgipaddr:2: error: ["invalid argument 192.-64.-1.1; expected syntax spec: (IP_ADDR|parse) "][IP::addr 192.-64.-1.1 equals 192.192.255.1] /Common/tcllsp_oq_sgipaddr:3: error: ["invalid argument 192.-64.-1.1; expected syntax spec: (IP_ADDR|parse) "][IP::addr 192.-64.-1.1 mask 255.255.255.0]  Unexpected Error: Loading configuration process failed.
LOG (no tagged lines)
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
