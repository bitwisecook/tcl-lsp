# `HTTP::collect 100` against request bodies of different lengths

## Question

With `HTTP::collect 100` in `HTTP_REQUEST`, when does `HTTP_REQUEST_DATA` fire and with how many bytes for a request with no body, a 100-byte and a 1500-byte body, a body shorter than its `Content-Length` (40 of 100) sent immediately or held open, and a chunked body of two 100-byte chunks?

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26 (VE, single NIC, 4 TMMs). The script is the body of `ltm rule tcllsp_oq_col_http`, attached to a dedicated virtual server with profiles `tcp { } http { }`, SNAT automap, and pool member 192.168.9.77:19900, a byte-counting sink on the driver host. The default `clientssl` profile's self-signed certificate was not verified by the client. The rule and virtual server were created after an absence check and deleted afterwards; nothing was saved. The client is `scripts/dev/bigip-probes/open-questions/collect.py`: one TCP connection per pattern with `TCP_NODELAY`, each write followed by its gap, then 1 s idle, then up to 1.5 s reading before closing. Request bodies: `cl0` = Content-Length 0, writes []; `cl100` = Content-Length 100, writes [(100, 0)]; `cl100_short` = Content-Length 100, writes [(40, 0)]; `cl100_hold3` = Content-Length 100, writes [(40, 3.0)]; `cl1500` = Content-Length 1500, writes [(1500, 0)]; `chunked_2x100` = Content-Length chunked, writes None. Times are milliseconds since `CLIENT_ACCEPTED` on the appliance, and since connect on the client; the two clocks are not synchronised. Each connection was served by one TMM.

## Script

```tcl-visible
when CLIENT_ACCEPTED {
set tcllsp_t0 [clock clicks -milliseconds]
}
when HTTP_REQUEST {
set tcllsp_n 0
HTTP::collect 100
}
when HTTP_REQUEST_DATA {
incr tcllsp_n
log local0. "TCLLSP_OQ|col|[TCP::client_port]|ev=$tcllsp_n|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|len=[HTTP::payload length]"
HTTP::respond 200 content "n=$tcllsp_n len=[HTTP::payload length]" Connection close
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
cl0: data event(s): none; closed at 2595 ms, events=0; client closed at 2510 ms; no reply
cl100: data event(s): 100 bytes at 4 ms; closed at 1006 ms, events=1; client closed at 1015 ms; reply starts 'HTTP/1.0 200 OK\r\nServer:'
cl100_short: data event(s): 40 bytes at 2505 ms; closed at 2510 ms, events=1; client closed at 2512 ms; no reply
cl100_hold3: data event(s): 40 bytes at 5532 ms; closed at 5537 ms, events=1; client closed at 5512 ms; no reply
cl1500: data event(s): 1500 bytes at 0 ms; closed at 1044 ms, events=1; client closed at 1053 ms; reply starts 'HTTP/1.0 200 OK\r\nServer:'
chunked_2x100: data event(s): 106 bytes at 0 ms; closed at 1307 ms, events=1; client closed at 1375 ms; reply starts 'HTTP/1.0 200 OK\r\nServer:'
```
