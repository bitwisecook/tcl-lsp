# `HTTP::collect` without a length

## Question

When `HTTP::collect` is called with no length in `HTTP_REQUEST` and nothing else calls `HTTP::release`, does `HTTP_REQUEST_DATA` fire once the complete `Content-Length` body has arrived, and what happens to the connection? F5's own command description says it will not fire without an `HTTP::release`. This measures that on a live request.

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26. The script is the body of `ltm rule tcllsp_oq_col_bare`. It was attached to virtual server `tcllsp_oq_col_bare_vs` (192.168.9.24:19990, profiles `tcp { } http { }`, SNAT automap, no pool), created after an absence check and deleted afterwards, and nothing was saved. The client, from the driver host, was `printf 'POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 100\r\n\r\n%0100d' 0 | nc -w 4 192.168.9.24 19990`, a complete 100-byte body. The log and connection table were read 3 s after `nc` exited, and again about 25 s later. The same behaviour appeared for every body shape in the harness run of `scripts/dev/bigip-probes/open-questions/collect.py` (no data event, no reply).

## Script

```tcl-visible
when CLIENT_ACCEPTED {
set tcllsp_t0 [clock clicks -milliseconds]
log local0. "TCLLSP_OQ|bare|[TCP::client_port]|accepted"
}
when HTTP_REQUEST {
set c [catch {HTTP::collect} v]
log local0. "TCLLSP_OQ|bare|[TCP::client_port]|HTTP_REQUEST|collect rc=$c v=$v"
}
when HTTP_REQUEST_DATA {
log local0. "TCLLSP_OQ|bare|[TCP::client_port]|DATA|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|len=[HTTP::payload length]"
HTTP::respond 200 content "len=[HTTP::payload length]" Connection close
}
when CLIENT_CLOSED {
log local0. "TCLLSP_OQ|bare|[TCP::client_port]|closed|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]"
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26, TMM

- Channel: tagged `/var/log/ltm` lines, `tmsh show sys connection cs-server-port 19990`, and the client's own exit status.
- Exit status: Not recorded
- Standard output: Not recorded
- Standard error: Not recorded
- Recorded result:

```text-visible
client: no reply, nc exit 0
TCLLSP_OQ|bare|57937|accepted
TCLLSP_OQ|bare|57937|HTTP_REQUEST|collect rc=0 v=
(no DATA line, no closed line, no TCL error)
after 3 s:  192.168.9.77:57937  192.168.9.24:19990  any6.any  any6.any  tcp  7  (tmm: 0)  none  none
after ~30 s: 192.168.9.77:57937  192.168.9.24:19990  any6.any  any6.any  tcp  40  (tmm: 0)  none  none
```

The connection-table column after `tcp` is the idle time in seconds. The connection was still held after the client had gone, and `CLIENT_CLOSED` had not fired.
