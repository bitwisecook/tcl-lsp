#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Best-effort synthetic-fixture reply capture after a failed native test.

Only definition and hover replies are persisted; request bodies, the fixture,
and unrelated messages stay out of the artifact. This is diagnostic only.
"""

import json
import os
import queue
import subprocess
import sys
import threading
import time
from pathlib import Path

root = Path(__file__).resolve().parents[2]
fixture = root / "rust/tcl-lsp-server-wasi/test/fixture/app.tcl"
server = root / "target/debug/tcl-lsp-server"
out = Path(sys.argv[1])
if not server.is_file():
    raise SystemExit(
        "native diagnostic unavailable: test build did not produce the server"
    )
source = fixture.read_text(encoding="utf-8")
uri = fixture.resolve().as_uri()
process = subprocess.Popen(
    [str(server)],
    cwd=fixture.parent,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
)
messages = queue.Queue()
write_lock = threading.Lock()


def send(value):
    assert process.stdin is not None
    body = json.dumps(value).encode()
    with write_lock:
        process.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
        process.stdin.flush()


def read():
    assert process.stdout is not None
    try:
        while True:
            headers = {}
            while True:
                line = process.stdout.readline()
                if not line:
                    return
                if line in (b"\r\n", b"\n"):
                    break
                key, value = line.decode().split(":", 1)
                headers[key.lower()] = value.strip()
            raw = process.stdout.read(int(headers["content-length"]))
            message = json.loads(raw)
            if "method" in message and "id" in message:
                response = (
                    [
                        (
                            "tcl8.6"
                            if item.get("section") == "tclLsp.dialect"
                            else {"dialect": "tcl8.6", "libraryPaths": []}
                        )
                        for item in message.get("params", {}).get("items", [])
                    ]
                    if message["method"] == "workspace/configuration"
                    else None
                )
                send({"jsonrpc": "2.0", "id": message["id"], "result": response})
            else:
                messages.put((message, raw))
    except (OSError, ValueError, KeyError, json.JSONDecodeError):
        return


threading.Thread(target=read, daemon=True).start()
serial = 0


def request(method, params):
    global serial
    serial += 1
    identifier = serial
    send({"jsonrpc": "2.0", "id": identifier, "method": method, "params": params})
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        try:
            message, raw = messages.get(timeout=max(0.01, deadline - time.monotonic()))
        except queue.Empty:
            break
        if message.get("id") == identifier and "method" not in message:
            if method in ("textDocument/definition", "textDocument/hover"):
                with out.open("ab") as stream:
                    stream.write(raw + b"\n")
            return message
    raise TimeoutError(method)


try:
    request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": None,
            "workspaceFolders": [],
            "capabilities": {
                "workspace": {"configuration": True},
                "textDocument": {"hover": {"contentFormat": ["markdown"]}},
            },
        },
    )
    send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
    send(
        {
            "jsonrpc": "2.0",
            "method": "workspace/didChangeConfiguration",
            "params": {"settings": {"tclLsp": {"dialect": "tcl8.6"}}},
        }
    )
    send(
        {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "tcl",
                    "version": 1,
                    "text": source,
                }
            },
        }
    )
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        report = request(
            "workspace/executeCommand",
            {
                "command": "tcl-lsp.currentDiagnostics",
                "arguments": [{"uri": uri, "version": 1, "source": source}],
            },
        )
        if report.get("result", {}).get("status") == "complete":
            break
        time.sleep(0.05)
    else:
        raise TimeoutError("currentDiagnostics")
    params = {"textDocument": {"uri": uri}, "position": {"line": 9, "character": 13}}
    request("textDocument/definition", params)
    request("textDocument/hover", params)
    request("shutdown", {})
    send({"jsonrpc": "2.0", "method": "exit", "params": {}})
finally:
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()
