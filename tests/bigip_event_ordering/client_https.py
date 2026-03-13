#!/usr/bin/env python3
"""Client: HTTPS GET request driver.

Sends one or more HTTPS GET requests to the VIP.  Uses an unverified
SSL context by default (the BIG-IP typically has a self-signed cert).

Usage::

    python client_https.py --host 10.1.1.10 --port 8443 [--count 1] [--verify]
"""

from __future__ import annotations

import argparse
import http.client
import ssl


def main() -> None:
    parser = argparse.ArgumentParser(description="HTTPS client (event-ordering test)")
    parser.add_argument("--host", required=True, help="VIP address")
    parser.add_argument("--port", type=int, required=True, help="VIP port")
    parser.add_argument("--count", type=int, default=1, help="Number of requests to send")
    parser.add_argument(
        "--verify", action="store_true", help="Verify server certificate (off by default)"
    )
    args = parser.parse_args()

    ctx = ssl.create_default_context()
    if not args.verify:
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE

    print(f"[https-client] connecting to {args.host}:{args.port}", flush=True)
    conn = http.client.HTTPSConnection(args.host, args.port, timeout=10, context=ctx)
    try:
        for i in range(args.count):
            conn.request("GET", f"/test?req={i}")
            resp = conn.getresponse()
            body = resp.read()
            print(
                f"[https-client] GET /test?req={i} -> {resp.status} ({len(body)} bytes)", flush=True
            )
    finally:
        conn.close()
    print("[https-client] done", flush=True)


if __name__ == "__main__":
    main()
