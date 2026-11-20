# `SSL::payload` offsets and lengths and `SSL::release` with -1

## Question

With 8 collected decrypted bytes on a client-SSL virtual server, what do `SSL::payload -1 4`, `SSL::payload 0 -1` and `SSL::release -1` do at load and at runtime, compared with `SSL::payload 0 4`?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sslpayload`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `python3 -I -c 'import socket,ssl;c=ssl.create_default_context();c.check_hostname=False;c.verify_mode=ssl.CERT_NONE;s=c.wrap_socket(socket.create_connection(("192.168.9.24",19716),5));s.settimeout(4);s.sendall(b"abcdefgh");d=b""
try:
 while True:
  b=s.recv(100)
  if not b: break
  d+=b
except Exception as e: print(type(e).__name__, end=" ")
print(s.version(), d)'` from the driver host. The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.

## Script

```tcl-visible
when CLIENTSSL_HANDSHAKE {
SSL::collect 8
}
when CLIENTSSL_DATA {
set c [catch {string length [SSL::payload 0 4]} v]
log local0. "TCLLSP_OQ|sslpayload|ctl_0_4|$c|$v"
set c [catch {string length [SSL::payload -1 4]} v]
log local0. "TCLLSP_OQ|sslpayload|off_neg|$c|$v"
set c [catch {string length [SSL::payload 0 -1]} v]
log local0. "TCLLSP_OQ|sslpayload|len_neg|$c|$v"
set c [catch {SSL::release -1} v]
log local0. "TCLLSP_OQ|sslpayload|release_neg|$c|$v"
log local0. "TCLLSP_OQ|sslpayload|after_release|[SSL::payload length]"
SSL::respond "done\n"
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
LOAD WARN: There were warnings: /Common/tcllsp_oq_sslpayload:7: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "invalid argument -1; expected syntax spec: (((OFFSET? LENGTH?)|length)|replace) "195 17][SSL::payload -1 4] /Common/tcllsp_oq_sslpayload:9: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "-1 is out of expected range(0 - 4294967295)"296 17][SSL::payload 0 -1] /Common/tcllsp_oq_sslpayload:11: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "-1 is out of expected range(0 - 4294967295)"382 15][SSL::release -1]
TRAFFIC ConnectionResetError None b''
LOG 1x after_release|0
LOG 1x ctl_0_4|0|4
LOG 1x len_neg|0|8
LOG 1x off_neg|1|     while executing "SSL::payload -1 4"
LOG 1x release_neg|0|
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
