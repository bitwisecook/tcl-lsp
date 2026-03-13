#!/usr/bin/env python3
"""Standalone LSP client for the Tcl language server.

Starts the server, sends LSP requests, and prints human-readable results.
Uses only the Python standard library — no external dependencies.

Usage:
    python3 lsp_client.py semantic-tokens <file.tcl>
    python3 lsp_client.py diagnostics <file.tcl>
    python3 lsp_client.py format <file.tcl>
    python3 lsp_client.py hover <file.tcl> <line> <col>
    python3 lsp_client.py completion <file.tcl> <line> <col>
    python3 lsp_client.py definition <file.tcl> <line> <col>
    python3 lsp_client.py references <file.tcl> <line> <col>
    python3 lsp_client.py code-actions <file.tcl> <line> <col> <end_line> <end_col>
    python3 lsp_client.py optimize <file.tcl>
    python3 lsp_client.py symbols <file.tcl>
    python3 lsp_client.py diagram <file.tcl>
    python3 lsp_client.py event-info <EVENT_NAME>
    python3 lsp_client.py command-info <COMMAND_NAME>
    python3 lsp_client.py context <file.tcl>
    python3 lsp_client.py all <file.tcl>
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any

# Constants

SEMANTIC_TOKEN_TYPES = [
    "keyword",  # 0
    "function",  # 1
    "variable",  # 2
    "string",  # 3
    "comment",  # 4
    "number",  # 5
    "operator",  # 6
    "parameter",  # 7
    "namespace",  # 8
    "regexp",  # 9
    "event",  # 10
    "decorator",  # 11
    "escape",  # 12
    "object",  # 13
    "fqdn",  # 14
    "ipAddress",  # 15
    "port",  # 16
    "routeDomain",  # 17
    "partition",  # 18
    "username",  # 19
    "encrypted",  # 20
    "pool",  # 21
    "monitor",  # 22
    "profile",  # 23
    "vlan",  # 24
    "interface",  # 25
    "regexpGroup",  # 26
    "regexpCharClass",  # 27
    "regexpQuantifier",  # 28
    "regexpAnchor",  # 29
    "regexpEscape",  # 30
    "regexpBackref",  # 31
    "regexpAlternation",  # 32
    "binarySpec",  # 33
    "binaryCount",  # 34
    "binaryFlag",  # 35
    "formatPercent",  # 36
    "formatSpec",  # 37
    "formatFlag",  # 38
    "formatWidth",  # 39
    "clockPercent",  # 40
    "clockSpec",  # 41
    "clockModifier",  # 42
]

SEMANTIC_TOKEN_MODIFIERS = [
    "declaration",  # bit 0
    "definition",  # bit 1
    "readonly",  # bit 2
]

LSP_SEVERITY = {1: "ERROR", 2: "WARNING", 3: "INFO", 4: "HINT"}

COMPLETION_KIND = {
    1: "Text",
    2: "Method",
    3: "Function",
    4: "Constructor",
    5: "Field",
    6: "Variable",
    7: "Class",
    8: "Interface",
    9: "Module",
    10: "Property",
    11: "Unit",
    12: "Value",
    13: "Enum",
    14: "Keyword",
    15: "Snippet",
    16: "Color",
    17: "File",
    18: "Reference",
    19: "Folder",
    20: "EnumMember",
    21: "Constant",
    22: "Struct",
    23: "Event",
    24: "Operator",
    25: "TypeParameter",
}

SYMBOL_KIND = {
    1: "File",
    2: "Module",
    3: "Namespace",
    4: "Package",
    5: "Class",
    6: "Method",
    7: "Property",
    8: "Field",
    9: "Constructor",
    10: "Enum",
    11: "Interface",
    12: "Function",
    13: "Variable",
    14: "Constant",
    15: "String",
    16: "Number",
    17: "Boolean",
    18: "Array",
    19: "Object",
    20: "Key",
    21: "Null",
    22: "EnumMember",
    23: "Struct",
    24: "Event",
    25: "Operator",
    26: "TypeParameter",
}


# LspClient — JSON-RPC 2.0 transport over stdio


class LspClient:
    """Manages a language server subprocess and JSON-RPC communication."""

    def __init__(self, server_dir: str) -> None:
        self.server_dir = server_dir
        self.process: subprocess.Popen | None = None
        self._request_id = 0
        self._pending: dict[int, dict] = {}  # id -> {"event": Event, "result": ...}
        self._notifications: list[dict] = []
        self._lock = threading.Lock()
        self._reader_thread: threading.Thread | None = None
        self._running = False

    def start(self) -> None:
        """Spawn the server and start the reader thread."""
        self.process = subprocess.Popen(
            ["uv", "run", "--directory", self.server_dir, "--no-dev", "python", "-m", "lsp"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=self.server_dir,
        )
        self._running = True
        self._reader_thread = threading.Thread(target=self._reader_loop, daemon=True)
        self._reader_thread.start()

    def _send(self, data: dict) -> None:
        """Send a JSON-RPC message with Content-Length framing."""
        body = json.dumps(data).encode("utf-8")
        header = f"Content-Length: {len(body)}\r\n\r\n".encode("utf-8")
        assert self.process and self.process.stdin
        self.process.stdin.write(header + body)
        self.process.stdin.flush()

    def _read_message(self) -> dict | None:
        """Read one Content-Length-framed message from the server's stdout."""
        assert self.process and self.process.stdout
        stdout = self.process.stdout

        # Read headers until empty line
        content_length = 0
        while True:
            line = stdout.readline()
            if not line:
                return None  # EOF
            line_str = line.decode("utf-8").rstrip("\r\n")
            if not line_str:
                break  # End of headers
            if line_str.lower().startswith("content-length:"):
                content_length = int(line_str.split(":", 1)[1].strip())

        if content_length == 0:
            return None

        body = stdout.read(content_length)
        if not body:
            return None
        return json.loads(body.decode("utf-8"))

    def _reader_loop(self) -> None:
        """Background thread: read messages and dispatch."""
        while self._running:
            try:
                msg = self._read_message()
            except Exception:
                break
            if msg is None:
                break

            if "id" in msg and "method" not in msg:
                # Response to a request
                rid = msg["id"]
                with self._lock:
                    if rid in self._pending:
                        entry = self._pending[rid]
                        entry["result"] = msg
                        entry["event"].set()
            elif "method" in msg and "id" not in msg:
                # Notification from server
                with self._lock:
                    self._notifications.append(msg)

    def send_request(self, method: str, params: dict, timeout: float = 30.0) -> Any:
        """Send a request and wait for the response."""
        self._request_id += 1
        rid = self._request_id
        event = threading.Event()
        with self._lock:
            self._pending[rid] = {"event": event, "result": None}

        self._send({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})

        if not event.wait(timeout):
            raise TimeoutError(f"Timeout waiting for response to {method} (id={rid})")

        with self._lock:
            entry = self._pending.pop(rid)
        result = entry["result"]

        if "error" in result:
            err = result["error"]
            raise RuntimeError(f"LSP error {err.get('code')}: {err.get('message')}")

        return result.get("result")

    def send_notification(self, method: str, params: dict | None = None) -> None:
        """Send a notification (no response expected)."""
        msg: dict = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            msg["params"] = params
        self._send(msg)

    def collect_notifications(self, method: str, timeout: float = 3.0) -> list[dict]:
        """Wait up to *timeout* seconds and return all notifications matching *method*."""
        deadline = time.monotonic() + timeout
        # Always wait at least a brief moment for the server to push
        time.sleep(min(0.2, timeout))
        while time.monotonic() < deadline:
            with self._lock:
                matches = [n for n in self._notifications if n.get("method") == method]
            if matches:
                # Give a tiny bit more time for additional notifications
                time.sleep(0.1)
                with self._lock:
                    matches = [n for n in self._notifications if n.get("method") == method]
                return matches
            time.sleep(0.05)
        # Timeout — return whatever we have
        with self._lock:
            return [n for n in self._notifications if n.get("method") == method]

    def shutdown(self) -> None:
        """Cleanly shut down the server."""
        self._running = False
        try:
            if self.process and self.process.poll() is None:
                try:
                    self.send_request("shutdown", {}, timeout=5.0)
                except Exception:
                    pass
                try:
                    self.send_notification("exit")
                except Exception:
                    pass
                try:
                    self.process.wait(timeout=3.0)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=2.0)
        except Exception:
            if self.process:
                try:
                    self.process.kill()
                except Exception:
                    pass


# LSP lifecycle helpers


def find_server_dir(override: str | None = None) -> str:
    """Locate the tcl-lsp server directory."""
    if override:
        p = Path(override).resolve()
        if (p / "lsp" / "__main__.py").exists():
            return str(p)
        raise FileNotFoundError(f"No server found at {p}")

    # Walk up from script location: .claude/skills/lsp-client/lsp_client.py
    script_dir = Path(__file__).resolve().parent
    # Try: script_dir -> .claude/skills/lsp-client
    #      project root -> script_dir / ../../..
    #      server -> project_root / tcl-lsp
    project_root = script_dir.parent.parent.parent
    server_dir = project_root / "tcl-lsp"
    if (server_dir / "lsp" / "__main__.py").exists():
        return str(server_dir)

    # Also try: maybe we're already inside tcl-lsp
    cwd = Path.cwd()
    if (cwd / "lsp" / "__main__.py").exists():
        return str(cwd)

    raise FileNotFoundError(
        f"Cannot find tcl-lsp server. Tried {server_dir} and cwd={cwd}. "
        "Use --server-dir to specify the path."
    )


def initialize(client: LspClient) -> dict:
    """Send initialize + initialized."""
    result = client.send_request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": f"file://{client.server_dir}",
            "capabilities": {
                "textDocument": {
                    "semanticTokens": {
                        "dynamicRegistration": False,
                        "requests": {"full": True},
                        "tokenTypes": SEMANTIC_TOKEN_TYPES,
                        "tokenModifiers": SEMANTIC_TOKEN_MODIFIERS,
                        "formats": ["relative"],
                    },
                    "publishDiagnostics": {
                        "relatedInformation": False,
                    },
                },
            },
        },
    )
    client.send_notification("initialized", {})
    return result


def open_document(client: LspClient, file_path: str) -> tuple[str, str]:
    """Read a file, send textDocument/didOpen, return (uri, content)."""
    abs_path = os.path.abspath(file_path)
    if not os.path.isfile(abs_path):
        raise FileNotFoundError(f"File not found: {abs_path}")
    with open(abs_path) as f:
        content = f.read()
    uri = f"file://{abs_path}"
    client.send_notification(
        "textDocument/didOpen",
        {
            "textDocument": {
                "uri": uri,
                "languageId": "tcl",
                "version": 1,
                "text": content,
            },
        },
    )
    return uri, content


# Decode / display helpers


def decode_semantic_tokens(data: list[int], source: str) -> list[dict]:
    """Delta-decode the 5-int encoded tokens into human-readable dicts."""
    lines = source.split("\n")
    tokens = []
    line, col = 0, 0

    for i in range(0, len(data), 5):
        delta_line = data[i]
        delta_char = data[i + 1]
        length = data[i + 2]
        type_idx = data[i + 3]
        mod_bits = data[i + 4]

        if delta_line > 0:
            line += delta_line
            col = delta_char
        else:
            col += delta_char

        # Extract source text
        text = ""
        if line < len(lines) and col < len(lines[line]):
            text = lines[line][col : col + length]

        # Decode modifier bitmask
        mods = []
        for bit, name in enumerate(SEMANTIC_TOKEN_MODIFIERS):
            if mod_bits & (1 << bit):
                mods.append(name)

        type_name = (
            SEMANTIC_TOKEN_TYPES[type_idx]
            if type_idx < len(SEMANTIC_TOKEN_TYPES)
            else f"unknown({type_idx})"
        )

        tokens.append(
            {
                "line": line,
                "col": col,
                "length": length,
                "type": type_name,
                "modifiers": mods,
                "text": text,
            }
        )

    return tokens


def print_semantic_tokens(tokens: list[dict]) -> None:
    """Print decoded semantic tokens in a readable table."""
    print(f"=== Semantic Tokens ({len(tokens)} tokens) ===")
    for tok in tokens:
        mods = " [" + ",".join(tok["modifiers"]) + "]" if tok["modifiers"] else ""
        print(f'  {tok["line"]:3d}:{tok["col"]:<3d}  {tok["type"]:<12s}  "{tok["text"]}"{mods}')


def print_diagnostics(diag_params: list[dict]) -> None:
    """Print diagnostics from publishDiagnostics notification params."""
    all_diags = []
    for params in diag_params:
        for d in params.get("diagnostics", []):
            all_diags.append(d)

    print(f"=== Diagnostics ({len(all_diags)} items) ===")
    if not all_diags:
        print("  (none)")
        return

    for d in all_diags:
        sev = LSP_SEVERITY.get(d.get("severity", 0), "?")
        code = d.get("code", "")
        r = d.get("range", {})
        s = r.get("start", {})
        e = r.get("end", {})
        msg = d.get("message", "")
        print(
            f"  {sev:<8s}  {code:<5s}  "
            f"{s.get('line', 0)}:{s.get('character', 0)}"
            f"-{e.get('line', 0)}:{e.get('character', 0)}"
            f"  {msg}"
        )


def print_formatting_edits(edits: list[dict] | None, source: str) -> None:
    """Print formatting edits."""
    if not edits:
        print("=== Formatting ===")
        print("  (no edits needed)")
        return

    print(f"=== Formatting ({len(edits)} edits) ===")
    for i, edit in enumerate(edits):
        r = edit.get("range", {})
        s = r.get("start", {})
        e = r.get("end", {})
        new_text = edit.get("newText", "")
        print(
            f"  Edit {i}: replace "
            f"{s.get('line', 0)}:{s.get('character', 0)}"
            f"-{e.get('line', 0)}:{e.get('character', 0)}"
        )
        # For whole-document replacement, show a summary
        lines = source.split("\n")
        total_lines = len(lines)
        new_lines = new_text.split("\n")
        s_line = s.get("line", 0)
        e_line = e.get("line", 0)
        if s_line == 0 and e_line >= total_lines - 1:
            # Whole document replacement — show the formatted result
            print("  (whole document replacement)")
            print()
            for ln in new_lines:
                print(f"    {ln}")
            print()
        else:
            # Partial edit — show the new text
            preview = new_text[:200]
            if len(new_text) > 200:
                preview += "..."
            print(f"  New text: {preview!r}")


def print_hover(result: dict | None) -> None:
    """Print hover result."""
    print("=== Hover ===")
    if not result:
        print("  (no hover)")
        return

    contents = result.get("contents", "")
    if isinstance(contents, dict):
        # MarkupContent
        value = contents.get("value", "")
        print(f"  {value}")
    elif isinstance(contents, str):
        print(f"  {contents}")
    elif isinstance(contents, list):
        for item in contents:
            if isinstance(item, dict):
                print(f"  {item.get('value', '')}")
            else:
                print(f"  {item}")


def print_completions(items: list[dict] | None) -> None:
    """Print completion items."""
    if items is None:
        items = []
    # Handle both CompletionList and direct array
    if isinstance(items, dict):
        items = items.get("items", [])

    print(f"=== Completions ({len(items)} items) ===")
    # Show first 30 items
    for item in items[:30]:
        label = item.get("label", "?")
        kind_num = item.get("kind", 0)
        kind = COMPLETION_KIND.get(kind_num, f"({kind_num})")
        detail = item.get("detail", "")
        detail_str = f"  -- {detail}" if detail else ""
        print(f"  {label:<30s}  {kind:<12s}{detail_str}")
    if len(items) > 30:
        print(f"  ... and {len(items) - 30} more")


def print_locations(locations: list[dict] | None, label: str) -> None:
    """Print location results (definition, references)."""
    if not locations:
        locations = []
    print(f"=== {label} ({len(locations)} locations) ===")
    if not locations:
        print("  (none)")
        return

    for loc in locations:
        uri = loc.get("uri", "")
        # Shorten the URI for display
        if uri.startswith("file://"):
            path = uri[7:]
            # Show just the filename
            short = os.path.basename(path)
        else:
            short = uri
        r = loc.get("range", {})
        s = r.get("start", {})
        e = r.get("end", {})
        print(
            f"  {short}  "
            f"{s.get('line', 0)}:{s.get('character', 0)}"
            f"-{e.get('line', 0)}:{e.get('character', 0)}"
        )


def print_code_actions(actions: list[dict] | None) -> None:
    """Print code action results."""
    if not actions:
        actions = []
    print(f"=== Code Actions ({len(actions)} actions) ===")
    if not actions:
        print("  (none)")
        return

    for action in actions:
        title = action.get("title", "?")
        kind = action.get("kind", "")
        print(f"  [{kind}] {title}")
        edit = action.get("edit", {})
        changes = edit.get("changes", {})
        for uri, edits in changes.items():
            for e in edits:
                r = e.get("range", {})
                s = r.get("start", {})
                end = r.get("end", {})
                new_text = e.get("newText", "")
                print(
                    f"    Replace "
                    f"{s.get('line', 0)}:{s.get('character', 0)}"
                    f"-{end.get('line', 0)}:{end.get('character', 0)}"
                    f" with: {new_text!r}"
                )
        command = action.get("command")
        if command:
            cmd_name = command.get("command", "?")
            cmd_args = command.get("arguments", [])
            print(f"    Command: {cmd_name} {cmd_args}")


def print_optimizations(result: dict | None, content: str) -> None:
    """Print optimization results from workspace/executeCommand."""
    if not result:
        print("=== Optimizations ===")
        print("  (no optimizations available)")
        return

    opts = result.get("optimizations", [])
    optimized_source = result.get("source", content)

    print(f"=== Optimizations ({len(opts)} items) ===")
    if not opts:
        print("  (none)")
    else:
        for o in opts:
            code = o.get("code", "")
            msg = o.get("message", "")
            sl = o.get("startLine", 0)
            sc = o.get("startCharacter", 0)
            el = o.get("endLine", 0)
            ec = o.get("endCharacter", 0)
            replacement = o.get("replacement", "")
            print(f"  {code:<5s}  {sl}:{sc}-{el}:{ec}  {msg}  \u2192  {replacement!r}")

    if optimized_source != content:
        print()
        print("=== Optimized Source ===")
        for ln in optimized_source.split("\n"):
            print(f"    {ln}")


def _flatten_document_symbols(
    symbols: list[dict],
    into: list[dict],
    depth: int = 0,
) -> None:
    """Recursively flatten nested DocumentSymbol hierarchy."""
    for sym in symbols:
        kind_num = sym.get("kind", 0)
        kind_label = SYMBOL_KIND.get(kind_num, f"({kind_num})")
        name = sym.get("name", "?")
        detail = sym.get("detail", "")
        sel_range = sym.get("selectionRange", sym.get("range", {}))
        start = sel_range.get("start", {})
        line = start.get("line", 0) + 1  # 1-based for display
        into.append(
            {
                "kind": kind_label,
                "name": name,
                "detail": detail,
                "line": line,
                "depth": depth,
            }
        )
        children = sym.get("children") or []
        if children:
            _flatten_document_symbols(children, into, depth + 1)


def print_symbols(symbols: list[dict] | None) -> None:
    """Print document symbols in a readable hierarchy."""
    if not symbols:
        print("=== Symbol Definitions ===")
        print("  (none)")
        return

    flat: list[dict] = []
    _flatten_document_symbols(symbols, flat)
    print(f"=== Symbol Definitions ({len(flat)} symbols) ===")
    for sym in flat:
        indent = "  " * sym["depth"]
        detail = f" {sym['detail']}" if sym["detail"] else ""
        print(f"  {indent}{sym['kind']} {sym['name']}{detail} (line {sym['line']})")


def print_event_info(result: dict | None) -> None:
    """Print iRules event registry metadata."""
    print("=== Event Info ===")
    if not result:
        print("  (no data)")
        return

    event = result.get("event", "?")
    known = result.get("known", False)
    deprecated = result.get("deprecated", False)
    cmd_count = result.get("validCommandCount", 0)
    samples = result.get("sampleCommands", [])

    print(f"  Event: {event}")
    print(f"  Known: {'yes' if known else 'no'}")
    print(f"  Deprecated: {'yes' if deprecated else 'no'}")
    print(f"  Valid commands: {cmd_count}")
    if samples:
        # Show first 20 commands, then summarize
        show = samples[:20]
        print(f"  Sample commands: {', '.join(show)}")
        if len(samples) > 20:
            print(f"    ... and {len(samples) - 20} more")


def print_command_info(result: dict | None) -> None:
    """Print iRules command registry metadata."""
    print("=== Command Info ===")
    if not result:
        print("  (no data)")
        return

    if not result.get("found", False):
        print(f"  Command '{result.get('command', '?')}' not found in registry")
        return

    command = result.get("command", "?")
    summary = result.get("summary", "")
    synopsis = result.get("synopsis", [])
    switches = result.get("switches", [])

    print(f"  Command: {command}")
    if summary:
        print(f"  Summary: {summary}")
    if synopsis:
        for syn in synopsis:
            print(f"  Synopsis: {syn}")
    if switches:
        print(f"  Switches: {', '.join(switches)}")

    valid_events = result.get("validEvents", [])
    any_event = result.get("anyEvent", False)
    if any_event:
        print("  Valid in: any event")
    elif valid_events:
        print(f"  Valid in: {', '.join(valid_events[:15])}")
        if len(valid_events) > 15:
            print(f"    ... and {len(valid_events) - 15} more")


def print_diagram_data(result: dict | None) -> None:
    """Print structured diagram data from compiler IR."""
    print("=== Diagram Data ===")
    if not result:
        print("  (no data)")
        return

    if result.get("error"):
        print(f"  Error: {result['error']}")
        return

    events = result.get("events", [])
    procedures = result.get("procedures", [])

    if events:
        print(f"\n  Events ({len(events)}, in firing order):")
        for evt in events:
            name = evt.get("name", "?")
            pri = evt.get("priority")
            mult = evt.get("multiplicity", "?")
            pri_str = f" priority={pri}" if pri is not None else ""
            flow_count = len(evt.get("flow", []))
            print(f"    {name} ({mult}{pri_str}) — {flow_count} flow nodes")

    if procedures:
        print(f"\n  Procedures ({len(procedures)}):")
        for proc in procedures:
            name = proc.get("name", "?")
            params = proc.get("params", [])
            flow_count = len(proc.get("flow", []))
            print(f"    {name}({', '.join(params)}) — {flow_count} flow nodes")

    # Print the full JSON for downstream consumption
    print("\n  --- Raw JSON ---")
    print(json.dumps(result, indent=2))


def _detect_events(source: str) -> list[str]:
    """Detect iRule event names from source (when EVENT { ... })."""
    events: list[str] = []
    seen: set[str] = set()
    for match in re.finditer(r"^\s*when\s+([A-Z][A-Z0-9_]{2,})\b", source, re.MULTILINE):
        name = match.group(1)
        if name not in seen:
            seen.add(name)
            events.append(name)
    return events


# Subcommand handlers


def cmd_semantic_tokens(client: LspClient, uri: str, content: str) -> None:
    """Request and display semantic tokens."""
    result = client.send_request(
        "textDocument/semanticTokens/full",
        {
            "textDocument": {"uri": uri},
        },
    )
    data = result.get("data", []) if result else []
    tokens = decode_semantic_tokens(data, content)
    print_semantic_tokens(tokens)


def cmd_diagnostics(client: LspClient, uri: str) -> None:
    """Collect and display pushed diagnostics."""
    notifs = client.collect_notifications("textDocument/publishDiagnostics")
    matching = [n["params"] for n in notifs if n.get("params", {}).get("uri") == uri]
    if not matching:
        # Try with all notifications (some servers may not match URI exactly)
        matching = [n["params"] for n in notifs]
    print_diagnostics(matching)


def cmd_format(client: LspClient, uri: str, content: str) -> None:
    """Request and display formatting edits."""
    result = client.send_request(
        "textDocument/formatting",
        {
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": True},
        },
    )
    print_formatting_edits(result, content)


def cmd_hover(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display hover information."""
    result = client.send_request(
        "textDocument/hover",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
        },
    )
    print_hover(result)


def cmd_completion(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display completions."""
    result = client.send_request(
        "textDocument/completion",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
        },
    )
    print_completions(result)


def cmd_definition(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display definitions."""
    result = client.send_request(
        "textDocument/definition",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
        },
    )
    if isinstance(result, dict):
        result = [result]
    print_locations(result, "Definition")


def cmd_references(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display references."""
    result = client.send_request(
        "textDocument/references",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
            "context": {"includeDeclaration": True},
        },
    )
    if isinstance(result, dict):
        result = [result]
    print_locations(result, "References")


def cmd_code_actions(
    client: LspClient,
    uri: str,
    line: int,
    col: int,
    end_line: int,
    end_col: int,
) -> None:
    """Request and display code actions for a range."""
    # Collect diagnostics that the server pushed after didOpen
    notifs = client.collect_notifications("textDocument/publishDiagnostics")
    diags_in_range = []
    for notif in notifs:
        params = notif.get("params", {})
        if params.get("uri") != uri:
            continue
        for d in params.get("diagnostics", []):
            r = d.get("range", {})
            s = r.get("start", {})
            e = r.get("end", {})
            # Check overlap with requested range
            if s.get("line", 0) <= end_line and e.get("line", 0) >= line:
                diags_in_range.append(d)

    result = client.send_request(
        "textDocument/codeAction",
        {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": line, "character": col},
                "end": {"line": end_line, "character": end_col},
            },
            "context": {"diagnostics": diags_in_range},
        },
    )
    print_code_actions(result)


def cmd_optimize(client: LspClient, uri: str, content: str) -> None:
    """Request and display optimization suggestions via workspace command."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.optimizeDocument",
            "arguments": [uri],
        },
    )
    print_optimizations(result, content)


def cmd_symbols(client: LspClient, uri: str) -> None:
    """Request and display document symbols."""
    result = client.send_request(
        "textDocument/documentSymbol",
        {
            "textDocument": {"uri": uri},
        },
    )
    print_symbols(result)


def cmd_diagram(client: LspClient, content: str) -> None:
    """Request diagram data via workspace command (takes source text)."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.diagramData",
            "arguments": [content],
        },
    )
    print_diagram_data(result)


def cmd_event_info(client: LspClient, event_name: str) -> None:
    """Request iRules event registry metadata."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.describeIruleEvent",
            "arguments": [event_name],
        },
    )
    print_event_info(result)


def cmd_command_info(client: LspClient, command_name: str) -> None:
    """Request iRules command registry metadata."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.describeIruleCommand",
            "arguments": [command_name],
        },
    )
    print_command_info(result)


def cmd_context(client: LspClient, uri: str, content: str) -> None:
    """Build a context pack: diagnostics + symbols + event metadata.

    Mirrors the context enrichment from the VS Code extension's contextPack.ts.
    """
    file_path = uri.replace("file://", "")
    basename = os.path.basename(file_path)
    line_count = len(content.split("\n"))

    # Detect dialect from extension
    ext = os.path.splitext(file_path)[1].lower()
    dialect = "f5-irules" if ext in (".irul", ".irule") else "tcl8.6"

    print("=== Context Pack ===")
    print(f"  Dialect: {dialect}")
    print(f"  File: {basename}")
    print(f"  Lines: {line_count}")

    # Diagnostics
    print()
    notifs = client.collect_notifications("textDocument/publishDiagnostics")
    matching = [n["params"] for n in notifs if n.get("params", {}).get("uri") == uri]
    if not matching:
        matching = [n["params"] for n in notifs]

    all_diags = []
    for params in matching:
        for d in params.get("diagnostics", []):
            all_diags.append(d)

    # Filter to actionable (error + warning)
    actionable = [d for d in all_diags if d.get("severity", 0) <= 2]
    actionable.sort(key=lambda d: d.get("range", {}).get("start", {}).get("line", 0))

    if actionable:
        print(f"=== Diagnostics ({len(actionable)}) ===")
        for d in actionable[:12]:
            sev = LSP_SEVERITY.get(d.get("severity", 0), "?")
            code = d.get("code", "")
            r = d.get("range", {})
            s = r.get("start", {})
            msg = d.get("message", "")
            print(f"  {sev} {code} line {s.get('line', 0) + 1}: {msg}")
        if len(actionable) > 12:
            print(f"  ... and {len(actionable) - 12} more")
    else:
        print("=== Diagnostics ===")
        print("  (no errors or warnings)")

    # Document symbols
    print()
    try:
        symbols_result = client.send_request(
            "textDocument/documentSymbol",
            {"textDocument": {"uri": uri}},
        )
        flat: list[dict] = []
        if symbols_result:
            _flatten_document_symbols(symbols_result, flat)
        if flat:
            print(f"=== Symbol Definitions ({len(flat)}) ===")
            for sym in flat[:15]:
                indent = "  " * sym["depth"]
                detail = f" {sym['detail']}" if sym["detail"] else ""
                print(f"  {indent}{sym['kind']} {sym['name']}{detail} (line {sym['line']})")
            if len(flat) > 15:
                print(f"  ... and {len(flat) - 15} more")
        else:
            print("=== Symbol Definitions ===")
            print("  (none)")
    except Exception:
        print("=== Symbol Definitions ===")
        print("  (unavailable)")

    # Event metadata (for iRules files)
    events = _detect_events(content)
    if events:
        print()
        print(f"=== Event Metadata ({len(events)} events, in source order) ===")
        for event_name in events[:8]:
            try:
                info = client.send_request(
                    "workspace/executeCommand",
                    {
                        "command": "tcl-lsp.describeIruleEvent",
                        "arguments": [event_name],
                    },
                )
                if info:
                    known = "yes" if info.get("known") else "no"
                    deprecated = "yes" if info.get("deprecated") else "no"
                    cmd_count = info.get("validCommandCount", 0)
                    samples = info.get("sampleCommands", [])[:8]
                    print(
                        f"  {event_name}: known={known}, deprecated={deprecated}, "
                        f"validCommands={cmd_count}"
                    )
                    if samples:
                        print(f"    sample: {', '.join(samples)}")
                else:
                    print(f"  {event_name}: metadata unavailable")
            except Exception:
                print(f"  {event_name}: metadata unavailable")
        if len(events) > 8:
            print(f"  ... and {len(events) - 8} more events")


def cmd_all(client: LspClient, uri: str, content: str) -> None:
    """Run semantic-tokens + diagnostics + symbols + format + optimize in one session."""
    cmd_semantic_tokens(client, uri, content)
    print()
    cmd_diagnostics(client, uri)
    print()
    cmd_symbols(client, uri)
    print()
    cmd_format(client, uri, content)
    print()
    cmd_optimize(client, uri, content)


# CLI


def main() -> None:
    parser = argparse.ArgumentParser(
        description="LSP client for the Tcl language server",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""\
examples:
  %(prog)s semantic-tokens samples/for_screenshots/03-completions.tcl
  %(prog)s diagnostics editors/vscode/testFixture/diagnostics.tcl
  %(prog)s hover editors/vscode/testFixture/procs.tcl 1 6
  %(prog)s all samples/for_screenshots/03-completions.tcl
""",
    )
    parser.add_argument("--server-dir", help="Path to tcl-lsp directory (auto-detected by default)")

    sub = parser.add_subparsers(dest="command", required=True)

    # semantic-tokens
    p = sub.add_parser("semantic-tokens", help="Decode and display semantic tokens")
    p.add_argument("file", help="Tcl file to analyze")

    # diagnostics
    p = sub.add_parser("diagnostics", help="Show diagnostics")
    p.add_argument("file", help="Tcl file to analyze")

    # format
    p = sub.add_parser("format", help="Show formatting edits")
    p.add_argument("file", help="Tcl file to format")

    # hover
    p = sub.add_parser("hover", help="Show hover info at position")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # completion
    p = sub.add_parser("completion", help="Show completions at position")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # definition
    p = sub.add_parser("definition", help="Show definition locations")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # references
    p = sub.add_parser("references", help="Show reference locations")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # code-actions
    p = sub.add_parser("code-actions", help="Show code actions in a range")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Start line (0-based)")
    p.add_argument("col", type=int, help="Start column (0-based)")
    p.add_argument("end_line", type=int, help="End line (0-based)")
    p.add_argument("end_col", type=int, help="End column (0-based)")

    # optimize
    p = sub.add_parser("optimize", help="Show optimization suggestions and rewritten source")
    p.add_argument("file", help="Tcl file to optimize")

    # symbols
    p = sub.add_parser("symbols", help="Show document symbol hierarchy")
    p.add_argument("file", help="Tcl file to analyze")

    # diagram
    p = sub.add_parser("diagram", help="Extract control flow diagram data from compiler IR")
    p.add_argument("file", help="Tcl/iRule file to analyze")

    # event-info (no file needed)
    p = sub.add_parser("event-info", help="Show iRules event registry metadata")
    p.add_argument("event", help="iRules event name (e.g. HTTP_REQUEST)")

    # command-info (no file needed)
    p = sub.add_parser("command-info", help="Show iRules command registry metadata")
    p.add_argument("name", help="iRules command name (e.g. HTTP::uri)")

    # context
    p = sub.add_parser("context", help="Build context pack: diagnostics + symbols + event metadata")
    p.add_argument("file", help="Tcl file to analyze")

    # all
    p = sub.add_parser(
        "all", help="Run semantic-tokens + diagnostics + symbols + format + optimize"
    )
    p.add_argument("file", help="Tcl file to analyze")

    args = parser.parse_args()

    # Find server
    try:
        server_dir = find_server_dir(args.server_dir)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    # Create client and run
    client = LspClient(server_dir)
    try:
        client.start()
        initialize(client)

        # Commands that don't require a file
        if args.command == "event-info":
            # Set dialect to f5-irules for event registry access
            client.send_notification(
                "workspace/didChangeConfiguration",
                {"settings": {"tclLsp": {"dialect": "f5-irules"}}},
            )
            time.sleep(0.2)
            cmd_event_info(client, args.event)
        elif args.command == "command-info":
            client.send_notification(
                "workspace/didChangeConfiguration",
                {"settings": {"tclLsp": {"dialect": "f5-irules"}}},
            )
            time.sleep(0.2)
            cmd_command_info(client, args.name)
        else:
            # Commands that require a file
            ext = os.path.splitext(args.file)[1].lower()
            if ext in (".irul", ".irule"):
                dialect = "f5-irules"
            elif ext in (".iapp", ".iappimpl", ".impl"):
                dialect = "f5-iapps"
            else:
                dialect = None
            if dialect:
                client.send_notification(
                    "workspace/didChangeConfiguration",
                    {"settings": {"tclLsp": {"dialect": dialect}}},
                )

            uri, content = open_document(client, args.file)

            # Give server a moment to process didOpen and push diagnostics
            time.sleep(0.3)

            match args.command:
                case "semantic-tokens":
                    cmd_semantic_tokens(client, uri, content)
                case "diagnostics":
                    cmd_diagnostics(client, uri)
                case "format":
                    cmd_format(client, uri, content)
                case "hover":
                    cmd_hover(client, uri, args.line, args.col)
                case "completion":
                    cmd_completion(client, uri, args.line, args.col)
                case "definition":
                    cmd_definition(client, uri, args.line, args.col)
                case "references":
                    cmd_references(client, uri, args.line, args.col)
                case "code-actions":
                    cmd_code_actions(client, uri, args.line, args.col, args.end_line, args.end_col)
                case "optimize":
                    cmd_optimize(client, uri, content)
                case "symbols":
                    cmd_symbols(client, uri)
                case "diagram":
                    cmd_diagram(client, content)
                case "context":
                    cmd_context(client, uri, content)
                case "all":
                    cmd_all(client, uri, content)

    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except TimeoutError as e:
        print(f"Timeout: {e}", file=sys.stderr)
        sys.exit(1)
    except RuntimeError as e:
        print(f"LSP error: {e}", file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        pass
    finally:
        client.shutdown()


if __name__ == "__main__":
    main()
