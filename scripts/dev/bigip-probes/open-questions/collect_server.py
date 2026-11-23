#!/usr/bin/env python3
"""Server-side collect: how much data SERVER_DATA, SERVERSSL_DATA and
HTTP_RESPONSE_DATA see for each TCP::collect / SSL::collect / HTTP::collect
size, when the *server* sends known write patterns.

  collect_server.py backend <tcp|tls|http> <port> [cert key]
      The pool member. TCP/TLS: reads one line naming a pattern, then sends
      that pattern's writes with their gaps, holds, and closes. HTTP: reads a
      request for /<pattern> and answers with that body shape.
  collect_server.py run <TCP|SSL|HTTP> <backend-ip> <backend-port> <out.jsonl>
  collect_server.py report <out.jsonl>

Every object is tcllsp_oq_scol_*; virtuals, rules and the pool are deleted
afterwards. Needs BIGIP_SSH (command prefix running a remote command as root).
COL_SIZES / COL_PATTERNS / COL_ONCE_ONLY select subsets as in collect.py.
"""

import json
import os
import socket
import ssl
import subprocess
import sys
import threading
import time

VIP = "192.168.9.24"
PORT = 20050
SSH = os.environ.get("BIGIP_SSH", "").split()

SIZES = [None, 0, 1, 4, 100, 1460, 1500, 3000, 65536]
HTTP_SIZES = [None, 0, 1, 4, 100, 1500, 3000, 65536, 1048576]

# server write patterns: (writes as (bytes, gap)), seconds held open before close
PATTERNS = {
    "1": ([(1, 0)], 1.0),
    "4": ([(4, 0)], 1.0),
    "100": ([(100, 0)], 1.0),
    "1500": ([(1500, 0)], 1.0),
    "100x3": ([(100, 0), (100, 0), (100, 0)], 1.0),
    "100x3gap": ([(100, 0.3), (100, 0.3), (100, 0)], 1.0),
    "1500x2gap": ([(1500, 0.3), (1500, 0)], 1.0),
    "10000": ([(10000, 0)], 1.0),
    "70000": ([(70000, 0)], 1.0),
    "1hold3": ([(1, 0)], 3.0),
    "100hold3": ([(100, 0)], 3.0),
}
# response bodies: header mode, writes, hold before close
HTTP_BODIES = {
    "cl0": ("0", [], 1.0),
    "cl1": ("1", [(1, 0)], 1.0),
    "cl4": ("4", [(4, 0)], 1.0),
    "cl100": ("100", [(100, 0)], 1.0),
    "cl1500": ("1500", [(1500, 0)], 1.0),
    "cl3000_2gap": ("3000", [(1500, 0.3), (1500, 0)], 1.0),
    "cl10000": ("10000", [(10000, 0)], 1.0),
    "cl70000": ("70000", [(70000, 0)], 1.0),
    "cl100_short": ("100", [(40, 0)], 1.0),
    "cl100_short_hold3": ("100", [(40, 0)], 3.0),
    "chunked_2x100": ("chunked", None, 1.0),
    "close_delimited_1500": ("none", [(1500, 0)], 1.0),
}


def data(n):
    return (b"abcdefghij" * (n // 10 + 1))[:n]


# --------------------------------------------------------------- backend
def backend(mode, port, cert=None, key=None):
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("0.0.0.0", port))
    srv.listen(64)
    ctx = None
    if mode == "tls":
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(cert, key)

    def readline(c):
        buf = b""
        while not buf.endswith(b"\n") and len(buf) < 4096:
            b = c.recv(1)
            if not b:
                break
            buf += b
        return buf.decode(errors="replace").strip()

    def handle(c):
        try:
            c.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            if ctx:
                c = ctx.wrap_socket(c, server_side=True)
            if mode == "http":
                line = readline(c)
                while readline(c):
                    pass  # headers
                name = line.split()[1].lstrip("/") if len(line.split()) > 1 else ""
                cl, writes, hold = HTTP_BODIES.get(name, ("0", [], 0))
                if cl == "chunked":
                    c.sendall(
                        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                    )
                    c.sendall(b"64\r\n" + data(100) + b"\r\n")
                    time.sleep(0.3)
                    c.sendall(b"64\r\n" + data(100) + b"\r\n0\r\n\r\n")
                else:
                    head = "HTTP/1.1 200 OK\r\nConnection: close\r\n"
                    if cl != "none":
                        head += f"Content-Length: {cl}\r\n"
                    c.sendall((head + "\r\n").encode())
                    for n, gap in writes:
                        c.sendall(data(n))
                        time.sleep(gap)
            else:
                name = readline(c)
                writes, hold = PATTERNS.get(name, ([], 0))
                for n, gap in writes:
                    c.sendall(data(n))
                    time.sleep(gap)
            time.sleep(hold)
        except Exception:
            pass
        try:
            c.close()
        except Exception:
            pass

    while True:
        c, _ = srv.accept()
        threading.Thread(target=handle, args=(c,), daemon=True).start()


# ----------------------------------------------------------------- rules
def size_word(s):
    return "" if s is None else f" {s}"


T = "[expr {[clock clicks -milliseconds] - $tcllsp_t0}]"


def rule(cmd, size, rearm):
    sw = size_word(size)
    head = "when CLIENT_ACCEPTED {\nset tcllsp_cport [TCP::remote_port]\nset tcllsp_n 0\nset tcllsp_t0 [clock clicks -milliseconds]\n}\n"
    log = 'log local0. "TCLLSP_OQ|scol|$tcllsp_cport|ev=$tcllsp_n|ms=' + T + '|len=%s"'
    closed = (
        'when SERVER_CLOSED {\nlog local0. "TCLLSP_OQ|scol|$tcllsp_cport|closed|ms='
        + T
        + '|events=$tcllsp_n"\n}\n'
    )
    if cmd == "TCP":
        ra = f"if {{$tcllsp_n < 20}} {{ TCP::collect{sw} }}\n" if rearm else ""
        return (
            head + "when SERVER_CONNECTED {\nTCP::collect" + sw + "\n}\n"
            "when SERVER_DATA {\nincr tcllsp_n\n"
            + log % "[TCP::payload length]"
            + "\nTCP::release\n"
            + ra
            + "}\n"
            + closed
        )
    if cmd == "SSL":
        ra = f"if {{$tcllsp_n < 20}} {{ SSL::collect{sw} }}\n" if rearm else ""
        return (
            head + "when SERVERSSL_HANDSHAKE {\nSSL::collect" + sw + "\n}\n"
            "when SERVERSSL_DATA {\nincr tcllsp_n\n"
            + log % "[SSL::payload length]"
            + "\nSSL::release\n"
            + ra
            + "}\n"
            + closed
        )
    if cmd == "HTTP":
        ra = f"if {{$tcllsp_n < 20}} {{ HTTP::collect{sw} }}\n" if rearm else ""
        return (
            head + "when HTTP_RESPONSE {\nHTTP::collect" + sw + "\n}\n"
            "when HTTP_RESPONSE_DATA {\nincr tcllsp_n\n"
            + log % "[HTTP::payload length]"
            + "\nHTTP::release\n"
            + ra
            + "}\n"
            + closed
        )
    raise ValueError(cmd)


def profiles(cmd):
    return {
        "TCP": "tcp { }",
        "SSL": "tcp { } serverssl { context serverside }",
        "HTTP": "tcp { } http { }",
    }[cmd]


def ssh(cmd, stdin=None):
    return subprocess.run(
        SSH + [cmd], input=stdin, capture_output=True, text=True
    ).stdout


# ----------------------------------------------------------------- client
def client(cmd, port, pattern):
    t0 = time.time()
    s = socket.create_connection((VIP, port), 5)
    s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    local = s.getsockname()[1]
    if cmd == "HTTP":
        s.sendall(
            f"GET /{pattern} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n".encode()
        )
    else:
        s.sendall(f"{pattern}\n".encode())
    s.settimeout(8)
    got, err, arrivals = b"", "", []
    try:
        while True:
            b = s.recv(65536)
            if not b:
                break
            got += b
            arrivals.append(int((time.time() - t0) * 1000))
    except Exception as e:
        err = type(e).__name__
    s.close()
    if cmd == "HTTP":
        body = got.split(b"\r\n\r\n", 1)[1] if b"\r\n\r\n" in got else b""
        status = got.split(b"\r\n", 1)[0].decode(errors="replace")
        return (
            local,
            err,
            {
                "received": len(got),
                "body": len(body),
                "status": status,
                "last_ms": arrivals[-1] if arrivals else None,
            },
        )
    return (
        local,
        err,
        {"received": len(got), "last_ms": arrivals[-1] if arrivals else None},
    )


def run(cmd, ip, bport, out):
    ssh(
        "tmsh list ltm pool tcllsp_oq_scol_pool >/dev/null 2>&1 && tmsh delete ltm pool tcllsp_oq_scol_pool"
    )
    print(
        ssh(
            "cat > /var/tmp/tcllsp_oq_scol_pool.conf; tmsh load sys config merge file /var/tmp/tcllsp_oq_scol_pool.conf 2>&1 "
            "| grep -v '^Loading\\|^  /'; rm -f /var/tmp/tcllsp_oq_scol_pool.conf",
            f"ltm pool tcllsp_oq_scol_pool {{ members {{ {ip}:{bport} {{ }} }} }}\n",
        ).strip(),
        file=sys.stderr,
    )
    sizes = HTTP_SIZES if cmd == "HTTP" else SIZES
    pats = list(HTTP_BODIES) if cmd == "HTTP" else list(PATTERNS)
    if os.environ.get("COL_SIZES"):
        sizes = [
            None if x == "none" else int(x) for x in os.environ["COL_SIZES"].split(",")
        ]
    if os.environ.get("COL_PATTERNS"):
        pats = [p for p in os.environ["COL_PATTERNS"].split(",") if p in pats]
    port = PORT + {"TCP": 0, "SSL": 100, "HTTP": 200}[cmd]
    with open(out, "a") as f:
        for size in sizes:
            for rearm in (False,) if os.environ.get("COL_ONCE_ONLY") else (False, True):
                port += 1
                name = f"tcllsp_oq_scol_{cmd.lower()}"
                conf = (
                    f"ltm rule {name} {{\n{rule(cmd, size, rearm)}}}\n"
                    f"ltm virtual {name}_vs {{ destination {VIP}:{port} mask 255.255.255.255 ip-protocol tcp "
                    f"profiles {{ {profiles(cmd)} }} pool tcllsp_oq_scol_pool "
                    f"source-address-translation {{ type automap }} rules {{ {name} }} }}\n"
                )
                off = ssh("stat -c %s /var/log/ltm").strip()
                load = ssh(
                    f"cat > /var/tmp/{name}.conf; tmsh load sys config merge file /var/tmp/{name}.conf 2>&1 "
                    f"| grep -v '^Loading\\|^  /'",
                    conf,
                ).strip()
                trials = []
                for p in pats:
                    local, err, got = client(cmd, port, p)
                    trials.append(
                        {"pattern": p, "client_port": local, "err": err, **got}
                    )
                    time.sleep(0.3)
                time.sleep(1.5)
                logs = ssh(
                    f"tail -c +$(({off} + 1)) /var/log/ltm | grep -a -F 'TCLLSP_OQ|scol|' | sed 's/^.*TCLLSP_OQ|scol|//'; "
                    f"tail -c +$(({off} + 1)) /var/log/ltm | grep -a 'TCL error' | grep -a {name} | sed 's/^/TCLERR|/' | head -3"
                )
                ssh(
                    f"tmsh delete ltm virtual {name}_vs; tmsh delete ltm rule {name}; rm -f /var/tmp/{name}.conf"
                )
                for t in trials:
                    t["events"] = [
                        l.split("|", 1)[1]
                        for l in logs.splitlines()
                        if l.split("|", 1)[0] == str(t["client_port"])
                    ]
                errs = [l for l in logs.splitlines() if l.startswith("TCLERR|")]
                f.write(
                    json.dumps(
                        {
                            "cmd": cmd,
                            "size": size,
                            "rearm": rearm,
                            "load": load,
                            "tcl_errors": errs,
                            "trials": trials,
                        }
                    )
                    + "\n"
                )
                f.flush()
                print(
                    cmd,
                    size,
                    "rearm" if rearm else "once",
                    "load:",
                    load[:80] or "ok",
                    "tclerr" if errs else "",
                    file=sys.stderr,
                )
    ssh("tmsh delete ltm pool tcllsp_oq_scol_pool")


def report(path):
    for line in open(path):
        r = json.loads(line)
        print(
            f"\n== {r['cmd']}::collect{size_word(r['size'])} server-side ({'re-arm' if r['rearm'] else 'once'})"
            + (f"  LOAD: {r['load'][:100]}" if r["load"] else "")
            + (f"  TCLERR: {r['tcl_errors'][0][:120]}" if r["tcl_errors"] else "")
        )
        for t in r["trials"]:
            evs = [e for e in t["events"] if e.startswith("ev=")]
            lens = ",".join(
                e.split("len=")[1] + "@" + e.split("ms=")[1].split("|")[0] for e in evs
            )
            closed = next(
                (
                    e.replace("closed|ms=", "srvclosed@")
                    for e in t["events"]
                    if e.startswith("closed")
                ),
                "",
            )
            extra = f" body={t['body']} {t['status'][:15]}" if "body" in t else ""
            print(
                f"  {t['pattern']:20s} events=[{lens}] {closed:24s} client_got={t['received']}{extra} "
                f"last@{t['last_ms']} err={t['err'] or '-'}"
            )


if __name__ == "__main__":
    m = sys.argv[1]
    if m == "backend":
        backend(sys.argv[2], int(sys.argv[3]), *(sys.argv[4:6]))
    elif m == "run":
        run(sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5])
    else:
        report(sys.argv[2])
