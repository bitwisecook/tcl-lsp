# `TCP::collect 100` when fewer or more bytes arrive

## Question

With `TCP::collect 100` in `CLIENT_ACCEPTED` and `TCP::release` in `CLIENT_DATA`, when does `CLIENT_DATA` fire and with how many bytes, if the client sends fewer than 100 bytes and keeps the connection open, three 100-byte writes 300 ms apart, or 1500 bytes at once? This is the question whether an explicit length is a minimum, a maximum, or a timeout, and what happens to data that never reaches it.

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26 (VE, single NIC, 4 TMMs). The script is the body of `ltm rule tcllsp_oq_col_tcp`, attached to a dedicated virtual server with profiles `tcp { }`, SNAT automap, and pool member 192.168.9.77:19900, a byte-counting sink on the driver host. The default `clientssl` profile's self-signed certificate was not verified by the client. The rule and virtual server were created after an absence check and deleted afterwards; nothing was saved. The client is `scripts/dev/bigip-probes/open-questions/collect.py`: one TCP connection per pattern with `TCP_NODELAY`, each write followed by its gap, then 1 s idle, then up to 1.5 s reading before closing. Client write patterns (bytes, then seconds before the next write): `1hold3` = [(1, 3.0)]; `100hold3` = [(100, 3.0)]; `100x3gap` = [(100, 0.3), (100, 0.3), (100, 0)]; `1500` = [(1500, 0)]. Times are milliseconds since `CLIENT_ACCEPTED` on the appliance, and since connect on the client; the two clocks are not synchronised. Each connection was served by one TMM.

## Script

```tcl-visible
when CLIENT_ACCEPTED {
set tcllsp_n 0
set tcllsp_t0 [clock clicks -milliseconds]
TCP::collect 100
}
when CLIENT_DATA {
incr tcllsp_n
log local0. "TCLLSP_OQ|col|[TCP::client_port]|ev=$tcllsp_n|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|len=[TCP::payload length]"
TCP::release
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
1hold3: data event(s): 1 bytes at 5508 ms; closed at 5522 ms, events=1; client closed at 5539 ms; no reply
100hold3: data event(s): 100 bytes at 0 ms; closed at 5520 ms, events=1; client closed at 5517 ms; no reply
100x3gap: data event(s): 100 bytes at 1 ms; closed at 3128 ms, events=1; client closed at 3123 ms; no reply
1500: data event(s): 1500 bytes at 0 ms; closed at 2516 ms, events=1; client closed at 2512 ms; no reply
```
