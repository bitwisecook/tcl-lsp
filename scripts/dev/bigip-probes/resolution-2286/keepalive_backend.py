#!/usr/bin/env python3
"""Disposable HTTP/1.1 backend that records peers and permits connection reuse."""

import argparse
import json
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--bind", required=True)
parser.add_argument("--port", type=int, required=True)
args = parser.parse_args()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        value = {
            "backend": self.server.server_address,
            "client": self.client_address,
            "path": self.path,
            "probe_id": self.headers.get("X-R2286-Request"),
            "scope": self.headers.get("X-R2286-Scope"),
            "utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
        body = (json.dumps(value, sort_keys=True) + "\n").encode("utf-8")
        print(json.dumps(value, sort_keys=True), flush=True)
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "keep-alive")
        self.end_headers()
        self.wfile.write(body)


ThreadingHTTPServer((args.bind, args.port), Handler).serve_forever()
