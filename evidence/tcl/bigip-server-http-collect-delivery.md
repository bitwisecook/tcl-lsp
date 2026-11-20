# Server-side `HTTP::collect 100`: when `HTTP_RESPONSE_DATA` fires

## Question

With `HTTP::collect 100` in `HTTP_RESPONSE` and `HTTP::release` in `HTTP_RESPONSE_DATA`, when does `HTTP_RESPONSE_DATA` fire and with how many bytes, for responses with no body, bodies shorter and longer than 100 bytes, a body shorter than its `Content-Length`, a chunked body, and a close-delimited body?

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
cl0: data event(s): none; SERVER_CLOSED at 1047 ms; client received 57 bytes (body 0, HTTP/1.1 200 OK), last at 114 ms
cl1: data event(s): 1 bytes at 12 ms; SERVER_CLOSED at 1014 ms; client received 58 bytes (body 1, HTTP/1.1 200 OK), last at 23 ms
cl4: data event(s): 4 bytes at 11 ms; SERVER_CLOSED at 1014 ms; client received 61 bytes (body 4, HTTP/1.1 200 OK), last at 21 ms
cl100: data event(s): 100 bytes at 12 ms; SERVER_CLOSED at 1043 ms; client received 159 bytes (body 100, HTTP/1.1 200 OK), last at 92 ms
cl1500: data event(s): 1500 bytes at 8 ms; SERVER_CLOSED at 1010 ms; client received 1560 bytes (body 1500, HTTP/1.1 200 OK), last at 19 ms
cl3000_2gap: data event(s): 1448 bytes at 10 ms; SERVER_CLOSED at 1318 ms; client received 3060 bytes (body 3000, HTTP/1.1 200 OK), last at 397 ms
cl10000: data event(s): 8688 bytes at 12 ms; SERVER_CLOSED at 1015 ms; client received 10061 bytes (body 10000, HTTP/1.1 200 OK), last at 28 ms
cl70000: data event(s): 1448 bytes at 9 ms; SERVER_CLOSED at 1012 ms; client received 70061 bytes (body 70000, HTTP/1.1 200 OK), last at 36 ms
cl100_short: data event(s): 40 bytes at 1041 ms; SERVER_CLOSED at 1047 ms; client received 99 bytes (body 40, HTTP/1.1 200 OK), last at 1116 ms
cl100_short_hold3: data event(s): 40 bytes at 3018 ms; SERVER_CLOSED at 3024 ms; client received 99 bytes (body 40, HTTP/1.1 200 OK), last at 3029 ms
chunked_2x100: data event(s): 106 bytes at 10 ms; SERVER_CLOSED at 1325 ms; client received 283 bytes (body 217, HTTP/1.1 200 OK), last at 350 ms
close_delimited_1500: data event(s): 1500 bytes at 13 ms; SERVER_CLOSED at 1023 ms; client received 1538 bytes (body 1500, HTTP/1.1 200 OK), last at 24 ms
```
