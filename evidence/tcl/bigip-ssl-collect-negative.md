# `SSL::collect -1`

## Question

On a client-SSL virtual server, what does `SSL::collect -1` in `CLIENTSSL_HANDSHAKE` do at load and at runtime: does `CLIENTSSL_DATA` fire, and with how many decrypted bytes, when the client sends 8 bytes?

## Assumptions

Run on 2026-11-20 UTC against BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs. The script is the body of an `ltm rule` named `tcllsp_oq_sslcollect`, loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; the configuration was never saved. Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN. Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. Client: `python3 -I -c 'import socket,ssl;c=ssl.create_default_context();c.check_hostname=False;c.verify_mode=ssl.CERT_NONE;s=c.wrap_socket(socket.create_connection(("192.168.9.24",19715),5));s.settimeout(4);s.sendall(b"abcdefgh");d=b""
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
set c [catch {SSL::collect -1} v]
log local0. "TCLLSP_OQ|sslcollect|CLIENTSSL_HANDSHAKE|$c|$v"
}
when CLIENTSSL_DATA {
log local0. "TCLLSP_OQ|sslcollect|CLIENTSSL_DATA|[SSL::payload length]"
SSL::respond "seen\n"
SSL::release
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
LOAD WARN: There were warnings: /Common/tcllsp_oq_sslcollect:2: warning: [The following errors were not caught before. Please correct the script in order to avoid future disruption. "-1 is out of expected range(0 - 4294967295)"41 15][SSL::collect -1]
TRAFFIC TimeoutError TLSv1.3 b''
LOG 1x CLIENTSSL_HANDSHAKE|0|
```

`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. `LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.
