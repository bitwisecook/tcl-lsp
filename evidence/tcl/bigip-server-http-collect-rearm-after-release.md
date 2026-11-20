# Re-arming `HTTP::collect` after `HTTP::release` in `HTTP_RESPONSE_DATA`

## Question

In `HTTP_RESPONSE_DATA`, after `HTTP::release`, does calling `HTTP::collect 100` again collect the rest of the response body, and what does the client receive?

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26 (VE, single NIC, 4 TMMs). The script is the body of `ltm rule tcllsp_oq_scol_http`, attached to a dedicated virtual server with profiles `tcp { } http { }`, SNAT automap, and pool member 192.168.9.77, a scripted backend on the driver host (`scripts/dev/bigip-probes/open-questions/collect_server.py backend`). The backend sends the named pattern with `TCP_NODELAY`, then holds the connection open and closes it. For SSL it presents a throwaway self-signed certificate, which the default `serverssl` profile does not verify. The client names the pattern in its first line (or the request path, for HTTP) and reads until close. The rule and virtual server were created after an absence check and deleted afterwards; nothing was saved. Response bodies (header, server writes as (bytes, gap s), hold s before close): `cl0` = 0, [], 1.0; `cl1` = 1, [(1, 0)], 1.0; `cl4` = 4, [(4, 0)], 1.0; `cl100` = 100, [(100, 0)], 1.0; `cl1500` = 1500, [(1500, 0)], 1.0; `cl3000_2gap` = 3000, [(1500, 0.3), (1500, 0)], 1.0; `cl10000` = 10000, [(10000, 0)], 1.0; `cl70000` = 70000, [(70000, 0)], 1.0; `cl100_short` = 100, [(40, 0)], 1.0; `cl100_short_hold3` = 100, [(40, 0)], 3.0; `chunked_2x100` = chunked, None, 1.0; `close_delimited_1500` = none, [(1500, 0)], 1.0. Times are milliseconds since `CLIENT_ACCEPTED` on the appliance, and since connect on the client; the two clocks are not synchronised.

## Script

```tcl-visible
when CLIENT_ACCEPTED {
set tcllsp_cport [TCP::remote_port]
set tcllsp_n 0
set tcllsp_t0 [clock clicks -milliseconds]
}
when HTTP_RESPONSE {
HTTP::collect 100
}
when HTTP_RESPONSE_DATA {
incr tcllsp_n
log local0. "TCLLSP_OQ|scol|$tcllsp_cport|ev=$tcllsp_n|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|len=[HTTP::payload length]"
HTTP::release
if {$tcllsp_n < 20} { HTTP::collect 100 }
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
cl0: data event(s): none; SERVER_CLOSED at 1014 ms; client received 57 bytes (body 0, HTTP/1.1 200 OK), last at 19 ms
cl1: data event(s): 1 bytes at 12 ms; SERVER_CLOSED at 12 ms; client received 58 bytes (body 1, HTTP/1.1 200 OK), last at 24 ms, ConnectionResetError
cl4: data event(s): 4 bytes at 10 ms; SERVER_CLOSED at 10 ms; client received 61 bytes (body 4, HTTP/1.1 200 OK), last at 29 ms, ConnectionResetError
cl100: data event(s): 100 bytes at 8 ms; SERVER_CLOSED at 8 ms; client received 159 bytes (body 100, HTTP/1.1 200 OK), last at 15 ms, ConnectionResetError
cl1500: data event(s): 1500 bytes at 12 ms; SERVER_CLOSED at 12 ms; client received 1560 bytes (body 1500, HTTP/1.1 200 OK), last at 21 ms, ConnectionResetError
cl3000_2gap: data event(s): 1448 bytes at 11 ms; SERVER_CLOSED at 11 ms; client received 1508 bytes (body 1448, HTTP/1.1 200 OK), last at 95 ms, ConnectionResetError
cl10000: data event(s): 8688 bytes at 9 ms; SERVER_CLOSED at 9 ms; client received 8749 bytes (body 8688, HTTP/1.1 200 OK), last at 17 ms, ConnectionResetError
cl70000: data event(s): 13032 bytes at 9 ms; SERVER_CLOSED at 9 ms; client received 13093 bytes (body 13032, HTTP/1.1 200 OK), last at 20 ms, ConnectionResetError
cl100_short: data event(s): 40 bytes at 1044 ms; SERVER_CLOSED at 1052 ms; client received 99 bytes (body 40, HTTP/1.1 200 OK), last at 1134 ms
cl100_short_hold3: data event(s): 40 bytes at 3013 ms; SERVER_CLOSED at 3018 ms; client received 99 bytes (body 40, HTTP/1.1 200 OK), last at 3022 ms
chunked_2x100: data event(s): 106 bytes at 11 ms; SERVER_CLOSED at 11 ms; client received 172 bytes (body 106, HTTP/1.1 200 OK), last at 56 ms, ConnectionResetError
close_delimited_1500: data event(s): 1500 bytes at 9 ms; SERVER_CLOSED at 9 ms; client received 1538 bytes (body 1500, HTTP/1.1 200 OK), last at 18 ms, ConnectionResetError
TCL error: /Common/tcllsp_oq_scol_http <HTTP_RESPONSE_DATA> - ERR_ARG (line 1)  / "HTTP::collect 100 "
TCL error: /Common/tcllsp_oq_scol_http <HTTP_RESPONSE_DATA> - ERR_ARG (line 4)  / "HTTP::collect 100 "
```
