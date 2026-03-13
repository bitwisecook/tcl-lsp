#!/usr/bin/env python3
"""Client: HTTPS POST request driver (triggers HTTP::collect events).

Sends one or more HTTPS POST requests with a JSON body to the VIP.
The request body triggers HTTP_REQUEST_DATA on the BIG-IP when the
iRule calls HTTP::collect.

Usage::

    python client_https_post.py --host 10.1.1.10 --port 8445 [--count 1]
"""

from __future__ import annotations

import argparse
import http.client
import json
import ssl


def main() -> None:
    parser = argparse.ArgumentParser(
        description="HTTPS POST client (event-ordering test, triggers collect)"
    )
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

    print(f"[https-post-client] connecting to {args.host}:{args.port}", flush=True)
    conn = http.client.HTTPSConnection(args.host, args.port, timeout=10, context=ctx)
    try:
        for i in range(args.count):
            body = json.dumps({"request": i, "test": "event-ordering"}).encode()
            conn.request(
                "POST",
                f"/test?req={i}",
                body=body,
                headers={
                    "Content-Type": "application/json",
                    "Content-Length": str(len(body)),
                },
            )
            resp = conn.getresponse()
            resp_body = resp.read()
            print(
                f"[https-post-client] POST /test?req={i} -> {resp.status} ({len(resp_body)} bytes)",
                flush=True,
            )
    finally:
        conn.close()
    print("[https-post-client] done", flush=True)


if __name__ == "__main__":
    main()
