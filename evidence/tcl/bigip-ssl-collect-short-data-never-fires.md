# `SSL::collect 100` when fewer or more bytes arrive

## Question

With `SSL::collect 100` in `CLIENTSSL_HANDSHAKE` and `SSL::release` in `CLIENTSSL_DATA` on a client-SSL virtual server, when does `CLIENTSSL_DATA` fire and with how many decrypted bytes, for the same client write patterns as the `TCP::collect` case? In particular, does it fire at connection close when fewer than 100 bytes arrive, as `TCP::collect` does?

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26 (VE, single NIC, 4 TMMs). The script is the body of `ltm rule tcllsp_oq_col_ssl`, attached to a dedicated virtual server with profiles `tcp { } clientssl { context clientside }`, SNAT automap, and pool member 192.168.9.77:19900, a byte-counting sink on the driver host. The default `clientssl` profile's self-signed certificate was not verified by the client. The rule and virtual server were created after an absence check and deleted afterwards; nothing was saved. The client is `scripts/dev/bigip-probes/open-questions/collect.py`: one TCP connection per pattern with `TCP_NODELAY`, each write followed by its gap, then 1 s idle, then up to 1.5 s reading before closing. Client write patterns (bytes, then seconds before the next write): `1hold3` = [(1, 3.0)]; `100hold3` = [(100, 3.0)]; `100x3gap` = [(100, 0.3), (100, 0.3), (100, 0)]; `1500` = [(1500, 0)]. Times are milliseconds since `CLIENT_ACCEPTED` on the appliance, and since connect on the client; the two clocks are not synchronised. Each connection was served by one TMM.

## Script

```tcl-visible
when CLIENT_ACCEPTED {
set tcllsp_n 0
set tcllsp_t0 [clock clicks -milliseconds]
}
when CLIENTSSL_HANDSHAKE {
SSL::collect 100
}
when CLIENTSSL_DATA {
incr tcllsp_n
log local0. "TCLLSP_OQ|col|[TCP::client_port]|ev=$tcllsp_n|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|len=[SSL::payload length]"
SSL::release
}
when CLIENT_CLOSED {
log local0. "TCLLSP_OQ|col|[TCP::client_port]|closed|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|events=$tcllsp_n"
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26, TMM

- Channel: traffic from the driver host; tagged `/var/log/ltm` lines matched to each connection by client port.
- Exit status: Not recorded
- Standard output: Not recorded
- Standard error: Not recorded
- Recorded result:

```text-visible
LOAD ACCEPT
1hold3: data event(s): none; closed at 5553 ms, events=0; client closed at 5546 ms; no reply
100hold3: data event(s): 100 bytes at 26 ms; closed at 5547 ms, events=1; client closed at 5543 ms; no reply
100x3gap: data event(s): 100 bytes at 20 ms; closed at 3147 ms, events=1; client closed at 3139 ms; no reply
1500: data event(s): 1500 bytes at 32 ms; closed at 2548 ms, events=1; client closed at 2542 ms; no reply
```
