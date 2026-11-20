# Server-side `SSL::collect 100`: when `SERVERSSL_DATA` fires

## Question

With `SSL::collect 100` in `SERVERSSL_HANDSHAKE` and `SSL::release` in `SERVERSSL_DATA` behind a server-SSL profile, when does `SERVERSSL_DATA` fire and with how many decrypted bytes, and what reaches the client when the server sends fewer than 100 bytes?

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26 (VE, single NIC, 4 TMMs). The script is the body of `ltm rule tcllsp_oq_scol_ssl`, attached to a dedicated virtual server with profiles `tcp { } serverssl { context serverside }`, SNAT automap, and pool member 192.168.9.77, a scripted backend on the driver host (`scripts/dev/bigip-probes/open-questions/collect_server.py backend`). The backend sends the named pattern with `TCP_NODELAY`, then holds the connection open and closes it. For SSL it presents a throwaway self-signed certificate, which the default `serverssl` profile does not verify. The client names the pattern in its first line (or the request path, for HTTP) and reads until close. The rule and virtual server were created after an absence check and deleted afterwards; nothing was saved. Server write patterns (writes as (bytes, gap s), hold s before close): `1` = [(1, 0)], 1.0; `4` = [(4, 0)], 1.0; `100` = [(100, 0)], 1.0; `1500` = [(1500, 0)], 1.0; `100x3` = [(100, 0), (100, 0), (100, 0)], 1.0; `100x3gap` = [(100, 0.3), (100, 0.3), (100, 0)], 1.0; `1500x2gap` = [(1500, 0.3), (1500, 0)], 1.0; `10000` = [(10000, 0)], 1.0; `70000` = [(70000, 0)], 1.0; `1hold3` = [(1, 0)], 3.0; `100hold3` = [(100, 0)], 3.0. Times are milliseconds since `CLIENT_ACCEPTED` on the appliance, and since connect on the client; the two clocks are not synchronised.

## Script

```tcl-visible
when CLIENT_ACCEPTED {
set tcllsp_cport [TCP::remote_port]
set tcllsp_n 0
set tcllsp_t0 [clock clicks -milliseconds]
}
when SERVERSSL_HANDSHAKE {
SSL::collect 100
}
when SERVERSSL_DATA {
incr tcllsp_n
log local0. "TCLLSP_OQ|scol|$tcllsp_cport|ev=$tcllsp_n|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|len=[SSL::payload length]"
SSL::release
}
when SERVER_CLOSED {
log local0. "TCLLSP_OQ|scol|$tcllsp_cport|closed|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|events=$tcllsp_n"
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26, TMM

- Channel: traffic from the driver host; tagged `/var/log/ltm` lines matched to each connection by client port; client-side byte counts.
- Exit status: Not recorded
- Standard output: Not recorded
- Standard error: Not recorded
- Recorded result:

```text-visible
LOAD ACCEPT
1: data event(s): none; SERVER_CLOSED at 1032 ms; client received 1 bytes, last at 1031 ms
4: data event(s): none; SERVER_CLOSED at 1035 ms; client received 4 bytes, last at 1035 ms
100: data event(s): 100 bytes at 30 ms; SERVER_CLOSED at 1042 ms; client received 100 bytes, last at 39 ms
1500: data event(s): 1500 bytes at 14 ms; SERVER_CLOSED at 1030 ms; client received 1500 bytes, last at 21 ms
100x3: data event(s): 100 bytes at 112 ms; SERVER_CLOSED at 1148 ms; client received 300 bytes, last at 124 ms
100x3gap: data event(s): 100 bytes at 15 ms; SERVER_CLOSED at 1628 ms; client received 300 bytes, last at 625 ms
1500x2gap: data event(s): 1500 bytes at 16 ms; SERVER_CLOSED at 1426 ms; client received 3000 bytes, last at 386 ms
10000: data event(s): 10000 bytes at 24 ms; SERVER_CLOSED at 1033 ms; client received 10000 bytes, last at 37 ms
70000: data event(s): 16384 bytes at 19 ms; SERVER_CLOSED at 1056 ms; client received 70000 bytes, last at 62 ms
1hold3: data event(s): none; SERVER_CLOSED at 3022 ms; client received 1 bytes, last at 3024 ms
100hold3: data event(s): 100 bytes at 14 ms; SERVER_CLOSED at 3026 ms; client received 100 bytes, last at 50 ms
```
