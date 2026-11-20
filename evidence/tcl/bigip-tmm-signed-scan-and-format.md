# TMM `binary scan`, `scan` and `format` with signed values

## Question

Does TMM's `binary scan`, `scan` and `format` treat signed values the way stock Tcl 8.4 does: signed byte and short fields, the `u` flag, `%x`/`%lx` of `ffffffff`, hex, unsigned and character formatting of -127, and the integer width that `<<` and `+` overflow at? This is the TMM half of the W156 question; F5 command arguments are separate cases.

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sginit`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through `RULE_INIT`, run by every TMM when the rule loads. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when RULE_INIT {
set c [catch {binary scan \x81\x00\x80 cS b s; list $b $s} v]
log local0. "TCLLSP_OQ|sginit|bscan_cS|$c|$v"
set c [catch {binary scan \x81\x00\x80 cs b s; list $b $s} v]
log local0. "TCLLSP_OQ|sginit|bscan_cs_le|$c|$v"
set c [catch {binary scan \x80\x00 S s; set s} v]
log local0. "TCLLSP_OQ|sginit|bscan_S_8000|$c|$v"
set c [catch {binary scan \x81\x00\x80 cS b s; list [expr {$b & 0xff}] [expr {$s & 0xffff}]} v]
log local0. "TCLLSP_OQ|sginit|bscan_masked|$c|$v"
set c [catch {binary scan \x81 cu b; set b} v]
log local0. "TCLLSP_OQ|sginit|bscan_cu|$c|$v"
set c [catch {scan ffffffff %x v; set v} v]
log local0. "TCLLSP_OQ|sginit|scan_x|$c|$v"
set c [catch {scan ffffffff %lx v; set v} v]
log local0. "TCLLSP_OQ|sginit|scan_lx|$c|$v"
set c [catch {format %02x -127} v]
log local0. "TCLLSP_OQ|sginit|fmt_hex|$c|$v"
set c [catch {format %02x [expr {-127 & 0xff}]} v]
log local0. "TCLLSP_OQ|sginit|fmt_hex_masked|$c|$v"
set c [catch {format %u -127} v]
log local0. "TCLLSP_OQ|sginit|fmt_u|$c|$v"
set c [catch {binary scan [format %c -127] H* h; set h} v]
log local0. "TCLLSP_OQ|sginit|fmt_c_hex|$c|$v"
set c [catch {string length [format %c -127]} v]
log local0. "TCLLSP_OQ|sginit|fmt_c_len|$c|$v"
set c [catch {list [expr {1 << 31}] [expr {1 << 32}] [expr {0x7fffffff + 1}] [expr {-1 >> 1}] [expr {1 << 63}] [expr {1 << 64}]} v]
log local0. "TCLLSP_OQ|sginit|intwidth|$c|$v"
set c [catch {IP::addr 192.192.255.1 equals 192.192.255.1} v]
log local0. "TCLLSP_OQ|sginit|ipaddr_ctl_eq|$c|$v"
set c [catch {IP::addr 192.192.255.1 mask 255.255.255.0} v]
log local0. "TCLLSP_OQ|sginit|ipaddr_ctl_mask|$c|$v"
set c [catch {set a 192.[expr {-64}].[expr {-1}].1; IP::addr $a equals 192.192.255.1} v]
log local0. "TCLLSP_OQ|sginit|ipaddr_dyn_eq|$c|$v"
set c [catch {set a 192.[expr {-64}].[expr {-1}].1; IP::addr $a mask 255.255.255.0} v]
log local0. "TCLLSP_OQ|sginit|ipaddr_dyn_mask|$c|$v"
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
LOG 4x bscan_cS|0|-127 128
LOG 4x bscan_cs_le|0|-127 -32768
LOG 4x bscan_cu|1|bad field specifier "u"
LOG 4x bscan_masked|0|129 128
LOG 4x bscan_S_8000|0|-32768
LOG 4x fmt_c_hex|0|fe81
LOG 4x fmt_c_len|0|2
LOG 4x fmt_hex|0|ffffffffffffff81
LOG 4x fmt_hex_masked|0|81
LOG 4x fmt_u|0|18446744073709551489
LOG 4x intwidth|0|2147483648 4294967296 2147483648 -1 -9223372036854775808 1
LOG 4x ipaddr_ctl_eq|0|1
LOG 4x ipaddr_ctl_mask|0|192.192.255.0
LOG 4x ipaddr_dyn_eq|1|bad IP network address format (line 1)     invoked from within "IP::addr $a equals 192.192.255.1"
LOG 4x ipaddr_dyn_mask|1|bad IP address format (line 1)     invoked from within "IP::addr $a mask 255.255.255.0"
LOG 4x scan_lx|0|-1
LOG 4x scan_x|0|-1
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
