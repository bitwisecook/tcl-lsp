#!/usr/bin/env python3
"""How much data do TCP::collect, SSL::collect and HTTP::collect deliver?

For each collect command, each requested size (none, 0, 1, 4, ... ) and each
variant (fire once, or re-arm after every release), load one rule and send a
fixed set of client write patterns through it. The rule logs every data event
with the client port, the event number and the payload length; the backend
sink (this script, on the driver host) records what reaches the server.

  collect.py sink <port> <jsonl-out>          run the backend sink
  collect.py run <cmd> <sink-ip> <sink-port> <out.jsonl>
                                              drive one command's matrix
  collect.py report <out.jsonl>               print the tables

Needs BIGIP_SSH (command prefix running a remote command as root).
Every object is tcllsp_oq_col_*; the pool and virtuals are deleted at the end.
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
PORT = 19950
SSH = os.environ.get("BIGIP_SSH", "").split()

SIZES = [None, 0, 1, 4, 100, 1460, 1500, 3000, 65536]
HTTP_SIZES = [None, 0, 1, 4, 100, 1500, 3000, 65536, 1048576]

# name -> list of (bytes, gap-seconds-after)
PATTERNS = {
    "1": [(1, 0)],
    "4": [(4, 0)],
    "100": [(100, 0)],
    "1500": [(1500, 0)],
    "100x3": [(100, 0), (100, 0), (100, 0)],
    "100x3gap": [(100, 0.3), (100, 0.3), (100, 0)],
    "1500x2gap": [(1500, 0.3), (1500, 0)],
    "10000": [(10000, 0)],
    "70000": [(70000, 0)],
    "1hold3": [(1, 3.0)],
    "100hold3": [(100, 3.0)],
}
# HTTP request bodies: name -> (content-length header or "chunked", writes)
HTTP_BODIES = {
    "cl0": ("0", []),
    "cl1": ("1", [(1, 0)]),
    "cl4": ("4", [(4, 0)]),
    "cl100": ("100", [(100, 0)]),
    "cl1500": ("1500", [(1500, 0)]),
    "cl3000_2gap": ("3000", [(1500, 0.3), (1500, 0)]),
    "cl10000": ("10000", [(10000, 0)]),
    "cl70000": ("70000", [(70000, 0)]),
    "cl100_short": ("100", [(40, 0)]),  # body shorter than its Content-Length
    "chunked_2x100": ("chunked", None),
    "cl100_hold3": ("100", [(40, 3.0)]),
}


def data(n):
    return (b"abcdefghij" * (n // 10 + 1))[:n]


# ------------------------------------------------------------------ sink
def sink(port, out):
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("0.0.0.0", port))
    srv.listen(64)
    lock = threading.Lock()

    def handle(c, peer):
        t0 = time.time()
        chunks = []
        c.settimeout(10)
        try:
            while True:
                b = c.recv(65536)
                if not b:
                    break
                chunks.append([round(time.time() - t0, 3), len(b)])
        except Exception as e:
            chunks.append(["error", type(e).__name__])
        c.close()
        with lock, open(out, "a") as f:
            f.write(
                json.dumps(
                    {
                        "t": t0,
                        "peer": peer[1],
                        "chunks": chunks,
                        "total": sum(x[1] for x in chunks if isinstance(x[1], int)),
                    }
                )
                + "\n"
            )

    while True:
        c, peer = srv.accept()
        threading.Thread(target=handle, args=(c, peer), daemon=True).start()


# ------------------------------------------------------------------ rules
def size_word(s):
    return "" if s is None else f" {s}"


def rule(cmd, size, rearm):
    sw = size_word(size)
    tag = 'log local0. "TCLLSP_OQ|col|{port}|ev=$tcllsp_n|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|len={len}"'
    if cmd == "TCP":
        rearm_line = f"if {{$tcllsp_n < 20}} {{ TCP::collect{sw} }}\n" if rearm else ""
        return (
            "when CLIENT_ACCEPTED {\nset tcllsp_n 0\nset tcllsp_t0 [clock clicks -milliseconds]\nTCP::collect"
            + sw
            + "\n}\n"
            "when CLIENT_DATA {\nincr tcllsp_n\n"
            + tag.replace("{port}", "[TCP::client_port]").replace(
                "{len}", "[TCP::payload length]"
            )
            + "\n"
            "TCP::release\n" + rearm_line + "}\n"
            'when CLIENT_CLOSED {\nlog local0. "TCLLSP_OQ|col|[TCP::client_port]|closed|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|events=$tcllsp_n"\n}\n'
        )
    if cmd == "SSL":
        rearm_line = f"if {{$tcllsp_n < 20}} {{ SSL::collect{sw} }}\n" if rearm else ""
        return (
            "when CLIENT_ACCEPTED {\nset tcllsp_n 0\nset tcllsp_t0 [clock clicks -milliseconds]\n}\n"
            "when CLIENTSSL_HANDSHAKE {\nSSL::collect" + sw + "\n}\n"
            "when CLIENTSSL_DATA {\nincr tcllsp_n\n"
            + tag.replace("{port}", "[TCP::client_port]").replace(
                "{len}", "[SSL::payload length]"
            )
            + "\n"
            "SSL::release\n" + rearm_line + "}\n"
            'when CLIENT_CLOSED {\nlog local0. "TCLLSP_OQ|col|[TCP::client_port]|closed|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|events=$tcllsp_n"\n}\n'
        )
    if cmd == "HTTP":
        rearm_line = f"if {{$tcllsp_n < 20}} {{ HTTP::collect{sw} }}\n" if rearm else ""
        respond = (
            ""
            if rearm
            else 'HTTP::respond 200 content "n=$tcllsp_n len=[HTTP::payload length]" Connection close\n'
        )
        release = "HTTP::release\n" + rearm_line if rearm else ""
        return (
            "when CLIENT_ACCEPTED {\nset tcllsp_t0 [clock clicks -milliseconds]\n}\nwhen HTTP_REQUEST {\nset tcllsp_n 0\nHTTP::collect"
            + sw
            + "\n}\n"
            "when HTTP_REQUEST_DATA {\nincr tcllsp_n\n"
            + tag.replace("{port}", "[TCP::client_port]").replace(
                "{len}", "[HTTP::payload length]"
            )
            + "\n"
            + respond
            + release
            + "}\n"
            'when CLIENT_CLOSED {\nlog local0. "TCLLSP_OQ|col|[TCP::client_port]|closed|ms=[expr {[clock clicks -milliseconds] - $tcllsp_t0}]|events=$tcllsp_n"\n}\n'
        )
    raise ValueError(cmd)


def profiles(cmd):
    return {
        "TCP": "tcp { }",
        "SSL": "tcp { } clientssl { context clientside }",
        "HTTP": "tcp { } http { }",
    }[cmd]


def ssh(cmd, stdin=None):
    return subprocess.run(
        SSH + [cmd], input=stdin, capture_output=True, text=True
    ).stdout


# ---------------------------------------------------------------- clients
def client(cmd, port, pattern):
    t_conn = time.time()
    raw = socket.create_connection((VIP, port), 5)
    raw.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    local = raw.getsockname()[1]
    s = raw
    if cmd == "SSL":
        ctx = ssl.create_default_context()
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE
        s = ctx.wrap_socket(raw)
    err = ""
    try:
        if cmd == "HTTP":
            cl, writes = HTTP_BODIES[pattern]
            if cl == "chunked":
                s.sendall(
                    b"POST / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n"
                )
                s.sendall(b"64\r\n" + data(100) + b"\r\n")
                time.sleep(0.3)
                s.sendall(b"64\r\n" + data(100) + b"\r\n0\r\n\r\n")
            else:
                s.sendall(
                    f"POST / HTTP/1.1\r\nHost: x\r\nContent-Length: {cl}\r\n\r\n".encode()
                )
                for n, gap in writes:
                    s.sendall(data(n))
                    time.sleep(gap)
        else:
            for n, gap in PATTERNS[pattern]:
                s.sendall(data(n))
                time.sleep(gap)
        time.sleep(1.0)
        s.settimeout(1.5)
        got = b""
        try:
            while True:
                b = s.recv(4096)
                if not b:
                    break
                got += b
        except Exception as e:
            err = type(e).__name__
        reply = got[:60].decode(errors="replace")
    except Exception as e:
        err, reply = type(e).__name__, ""
    t_close = int((time.time() - t_conn) * 1000)
    try:
        s.close()
    except Exception:
        pass
    return local, err, f"[close@{t_close}ms] " + reply


def run(cmd, sink_ip, sink_port, out):
    pool = (
        f"ltm pool tcllsp_oq_col_pool {{ members {{ {sink_ip}:{sink_port} {{ }} }} }}\n"
    )
    print(
        ssh(
            "tmsh list ltm pool tcllsp_oq_col_pool >/dev/null 2>&1 && echo POOL-EXISTS; "
            "cat > /var/tmp/tcllsp_oq_col_pool.conf; tmsh load sys config merge file /var/tmp/tcllsp_oq_col_pool.conf 2>&1 "
            "| grep -v '^Loading\\|^  /'; rm -f /var/tmp/tcllsp_oq_col_pool.conf",
            pool,
        ).strip(),
        file=sys.stderr,
    )
    sizes = HTTP_SIZES if cmd == "HTTP" else SIZES
    pats = list(HTTP_BODIES) if cmd == "HTTP" else list(PATTERNS)
    # optional subsets: COL_SIZES="none,100" COL_PATTERNS="1hold3,100x3gap" COL_ONCE_ONLY=1
    if os.environ.get("COL_SIZES"):
        sizes = [
            None if x == "none" else int(x) for x in os.environ["COL_SIZES"].split(",")
        ]
    if os.environ.get("COL_PATTERNS"):
        pats = [p for p in os.environ["COL_PATTERNS"].split(",") if p in pats]
    port = PORT
    with open(out, "a") as f:
        for size in sizes:
            for rearm in (False,) if os.environ.get("COL_ONCE_ONLY") else (False, True):
                port += 1
                name = f"tcllsp_oq_col_{cmd.lower()}"
                conf = (
                    f"ltm rule {name} {{\n{rule(cmd, size, rearm)}}}\n"
                    f"ltm virtual {name}_vs {{ destination {VIP}:{port} mask 255.255.255.255 ip-protocol tcp "
                    f"profiles {{ {profiles(cmd)} }} pool tcllsp_oq_col_pool "
                    f"source-address-translation {{ type automap }} rules {{ {name} }} }}\n"
                )
                off = ssh("stat -c %s /var/log/ltm").strip()
                load = ssh(
                    f"tmsh list ltm rule {name} >/dev/null 2>&1 && echo EXISTS; "
                    f"cat > /var/tmp/{name}.conf; tmsh load sys config merge file /var/tmp/{name}.conf 2>&1 "
                    f"| grep -v '^Loading\\|^  /'",
                    conf,
                ).strip()
                trials = []
                for p in pats:
                    t0 = time.time()
                    local, err, reply = client(cmd, port, p)
                    trials.append(
                        {
                            "pattern": p,
                            "client_port": local,
                            "err": err,
                            "reply": reply,
                            "t": t0,
                        }
                    )
                    time.sleep(0.5)
                time.sleep(1.5)
                logs = ssh(
                    f"tail -c +$(({off} + 1)) /var/log/ltm | grep -a -F 'TCLLSP_OQ|col|' "
                    f"| sed 's/^.*TCLLSP_OQ|col|//'"
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
                rec = {
                    "cmd": cmd,
                    "size": size,
                    "rearm": rearm,
                    "load": load,
                    "trials": trials,
                }
                f.write(json.dumps(rec) + "\n")
                f.flush()
                print(
                    cmd,
                    size,
                    "rearm" if rearm else "once",
                    "load:",
                    (load[:80] or "ok"),
                    file=sys.stderr,
                )
    ssh("tmsh delete ltm pool tcllsp_oq_col_pool")


def report(path, sink_log=None):
    sinks = [json.loads(l) for l in open(sink_log)] if sink_log else []
    for line in open(path):
        r = json.loads(line)
        print(
            f"\n== {r['cmd']}::collect{size_word(r['size'])} ({'re-arm' if r['rearm'] else 'once'})"
            + (f"  LOAD: {r['load'][:120]}" if r["load"] else "")
        )
        for t in r["trials"]:
            evs = [e for e in t["events"] if e.startswith("ev=")]
            lens = ",".join(
                e.split("len=")[1]
                + ("@" + e.split("ms=")[1].split("|")[0] if "ms=" in e else "")
                for e in evs
            )
            closed = next((e for e in t["events"] if e.startswith("closed")), "")
            # sink connection whose start falls in this trial's window
            sk = [s for s in sinks if t["t"] - 0.2 <= s["t"] <= t["t"] + 3.0]
            skt = "/".join(str(s["total"]) for s in sk) if sk else "-"
            print(
                f"  {t['pattern']:14s} events=[{lens}] {closed:12s} sink={skt:8s} err={t['err'] or '-'} reply={t['reply'][:40]!r}"
            )


if __name__ == "__main__":
    m = sys.argv[1]
    if m == "sink":
        sink(int(sys.argv[2]), sys.argv[3])
    elif m == "run":
        run(sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5])
    else:
        report(sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else None)
