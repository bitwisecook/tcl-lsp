#!/usr/bin/env python3
"""Compare the Python and Rust LSP backends across real Tcl projects.

Drives both backends over JSON-RPC (reusing the lsp-client skill's
``LspClient``) and measures five dimensions on real code from
``tmp/tcllib-2.0`` and ``tmp/tcl9.0.3``:

  1. time to semantic tokens (per large file)
  2. heavy edits of a large file (re-analysis latency per incremental edit)
  3. time to full diagnostics (per large file)
  4. time to full optimisation (per large file)
  5. all tools on a large multi-file codebase (open/index + per-feature)

Usage:
    python scripts/dev/bench_lsp_backends.py [--json out.json]

The Python backend is the shipped zipapp (``build/tcl-lsp-server-*.pyz``,
built on demand); the Rust backend is ``target/release/tcl-lsp-server``.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / ".claude" / "skills" / "lsp-client"))

from lsp_client import LspClient, initialize  # noqa: E402

TIMEOUT = 90.0


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def _uri(path: Path) -> str:
    return f"file://{path.resolve()}"


def did_open(client: LspClient, path: Path, text: str, version: int = 1) -> str:
    uri = _uri(path)
    client.send_notification(
        "textDocument/didOpen",
        {"textDocument": {"uri": uri, "languageId": "tcl", "version": version, "text": text}},
    )
    return uri


def did_close(client: LspClient, uri: str) -> None:
    client.send_notification("textDocument/didClose", {"textDocument": {"uri": uri}})


def wait_push_diagnostics(client: LspClient, uri: str, start: float, timeout: float = TIMEOUT):
    """Push mode: wait (from *start*) for publishDiagnostics for *uri*.

    Returns (elapsed_ms, count).  Returns the first non-empty push; if only
    an empty push arrives it keeps waiting briefly for a real one, then
    reports the empty result.  (None, None) if nothing arrives.
    """
    deadline = start + timeout
    empty_at = None
    while time.perf_counter() < deadline:
        with client._lock:  # noqa: SLF001 — reusing the skill client's buffer
            diags = [
                n
                for n in client._notifications
                if n.get("method") == "textDocument/publishDiagnostics"
                and n.get("params", {}).get("uri") == uri
            ]
        if diags:
            last = diags[-1]["params"]["diagnostics"]
            now = time.perf_counter()
            if last:
                return (now - start) * 1000.0, len(last)
            # empty push — give the real pipeline a little longer
            if empty_at is None:
                empty_at = now
            elif now - empty_at >= 2.0:
                return (empty_at - start) * 1000.0, 0
        time.sleep(0.01)
    return (None, None)


def clear_notifications(client: LspClient) -> None:
    with client._lock:  # noqa: SLF001
        client._notifications.clear()


def timed(fn, *args, **kw):
    t0 = time.perf_counter()
    res = fn(*args, **kw)
    return (time.perf_counter() - t0) * 1000.0, res


def median_ms(samples: list[float]) -> float:
    return statistics.median(samples) if samples else float("nan")


# Measurement phases


def bench_semantic_tokens(kind: str, pyz: Path, path: Path, text: str):
    """Cold open-to-tokens: fresh server, didOpen, time the first response."""
    client = make_client(kind, pyz)
    try:
        uri = did_open(client, path, text, version=1)
        try:
            ms, res = timed(
                client.send_request,
                "textDocument/semanticTokens/full",
                {"textDocument": {"uri": uri}},
                TIMEOUT,
            )
        except TimeoutError:
            return {"ms": None, "tokens": None}
        ntok = len(res.get("data", [])) // 5 if res else 0
        return {"ms": ms, "tokens": ntok}
    finally:
        client.shutdown()


def bench_heavy_edits(kind: str, pyz: Path, path: Path, text: str, cycles: int = 8):
    """Incremental insert at 0:0 then re-request tokens; time each re-analysis."""
    client = make_client(kind, pyz)
    try:
        uri = did_open(client, path, text, version=1)
        # warm the document once
        try:
            client.send_request(
                "textDocument/semanticTokens/full", {"textDocument": {"uri": uri}}, TIMEOUT
            )
        except TimeoutError:
            return {"median_ms": None, "cycles": 0}
        samples = []
        version = 100
        for i in range(cycles):
            version += 1
            client.send_notification(
                "textDocument/didChange",
                {
                    "textDocument": {"uri": uri, "version": version},
                    "contentChanges": [
                        {
                            "range": {
                                "start": {"line": 0, "character": 0},
                                "end": {"line": 0, "character": 0},
                            },
                            "text": f"# benchmark edit {i}\n",
                        }
                    ],
                },
            )
            try:
                ms, _ = timed(
                    client.send_request,
                    "textDocument/semanticTokens/full",
                    {"textDocument": {"uri": uri}},
                    TIMEOUT,
                )
            except TimeoutError:
                samples.append(TIMEOUT * 1000.0)
                break
            samples.append(ms)
        return {
            "median_ms": median_ms(samples),
            "min_ms": min(samples) if samples else None,
            "max_ms": max(samples) if samples else None,
            "cycles": len(samples),
        }
    finally:
        client.shutdown()


def bench_diagnostics(kind: str, pyz: Path, path: Path, text: str):
    """Cold time-to-full-diagnostics from a fresh server."""
    client = make_client(kind, pyz)
    try:
        if kind == "rust":
            uri = did_open(client, path, text, version=1)
            try:
                ms, res = timed(
                    client.send_request,
                    "textDocument/diagnostic",
                    {"textDocument": {"uri": uri}},
                    TIMEOUT,
                )
            except TimeoutError:
                return {"ms": None, "count": None}
            items = res.get("items", []) if isinstance(res, dict) else (res or [])
            return {"ms": ms, "count": len(items)}
        # python push mode: time from didOpen to the publishDiagnostics push
        clear_notifications(client)
        start = time.perf_counter()
        uri = did_open(client, path, text, version=1)
        ms, count = wait_push_diagnostics(client, uri, start)
        return {"ms": ms, "count": count}
    finally:
        client.shutdown()


def bench_optimise(kind: str, pyz: Path, path: Path, text: str):
    client = make_client(kind, pyz)
    try:
        uri = did_open(client, path, text, version=1)
        try:
            ms, res = timed(
                client.send_request,
                "workspace/executeCommand",
                {"command": "tcl-lsp.optimiseDocument", "arguments": [uri, "full"]},
                TIMEOUT,
            )
        except Exception as exc:  # noqa: BLE001
            return {"ms": None, "error": str(exc)[:80]}
        n = None
        if isinstance(res, dict):
            for key in ("optimizations", "suggestions", "edits", "changes", "diagnostics"):
                if isinstance(res.get(key), list):
                    n = len(res[key])
                    break
        return {"ms": ms, "suggestions": n}
    finally:
        client.shutdown()


def bench_multifile(client: LspClient, files: list[Path], kind: str):
    texts = {p: _read_text(p) for p in files}
    uris = {}
    # 1. open + index all files
    clear_notifications(client)
    t0 = time.perf_counter()
    for p, txt in texts.items():
        uris[p] = did_open(client, p, txt, version=1)
    open_ms = (time.perf_counter() - t0) * 1000.0

    # 2. diagnostics across the whole set.  Rust pull-mode: sum the
    #    per-file diagnostic requests.  Python push-mode is async and
    #    ~10-22 s *per file* (see single-file numbers); waiting on 100+
    #    files would dwarf the run, so it is reported per-file instead.
    if kind == "rust":
        dt0 = time.perf_counter()
        for p in files:
            try:
                client.send_request(
                    "textDocument/diagnostic", {"textDocument": {"uri": uris[p]}}, TIMEOUT
                )
            except Exception:  # noqa: BLE001
                pass
        diag_ms = (time.perf_counter() - dt0) * 1000.0
    else:
        diag_ms = None

    # 3. per-feature totals across all files (no position needed)
    feature_totals: dict[str, float] = {}

    def run_all(method: str, params_for):
        total = 0.0
        ok = 0
        for p in files:
            try:
                ms, _ = timed(client.send_request, method, params_for(uris[p]), TIMEOUT)
                total += ms
                ok += 1
            except Exception:  # noqa: BLE001
                pass
        return total, ok

    feature_totals["documentSymbol"], _ = run_all(
        "textDocument/documentSymbol", lambda u: {"textDocument": {"uri": u}}
    )
    feature_totals["foldingRange"], _ = run_all(
        "textDocument/foldingRange", lambda u: {"textDocument": {"uri": u}}
    )
    feature_totals["semanticTokens"], _ = run_all(
        "textDocument/semanticTokens/full", lambda u: {"textDocument": {"uri": u}}
    )
    feature_totals["formatting"], _ = run_all(
        "textDocument/formatting",
        lambda u: {"textDocument": {"uri": u}, "options": {"tabSize": 4, "insertSpaces": True}},
    )

    # 4. position features on a sampled subset (every 5th file), at a proc
    sample = files[::5] or files
    pos_methods = {
        "hover": "textDocument/hover",
        "completion": "textDocument/completion",
        "definition": "textDocument/definition",
        "references": "textDocument/references",
    }
    pos_totals = {k: 0.0 for k in pos_methods}
    for p in sample:
        line, char = _first_code_position(texts[p])
        for label, method in pos_methods.items():
            params = {
                "textDocument": {"uri": uris[p]},
                "position": {"line": line, "character": char},
            }
            if label == "references":
                params["context"] = {"includeDeclaration": True}
            try:
                ms, _ = timed(client.send_request, method, params, TIMEOUT)
                pos_totals[label] += ms
            except Exception:  # noqa: BLE001
                pass

    for p in files:
        did_close(client, uris[p])

    return {
        "files": len(files),
        "open_ms": open_ms,
        "diag_ms": diag_ms,
        "feature_totals_ms": feature_totals,
        "pos_sample": len(sample),
        "pos_totals_ms": pos_totals,
    }


def _first_code_position(text: str) -> tuple[int, int]:
    for i, line in enumerate(text.split("\n")):
        s = line.strip()
        if s and not s.startswith("#"):
            # point at the first word's interior
            col = len(line) - len(line.lstrip())
            return i, col + 1
    return 0, 0


def make_client(kind: str, pyz: Path) -> LspClient:
    if kind == "rust":
        binary = str(ROOT / "target" / "release" / "tcl-lsp-server")
        client = LspClient(str(ROOT), launch_cmd=[binary])
    else:
        client = LspClient(str(ROOT), launch_cmd=[sys.executable, str(pyz)])
    client.start()
    initialize(client)
    return client


def _fmt(v):
    return f"{v:,.0f}" if isinstance(v, (int, float)) else "—"


def run_backend(kind: str, pyz: Path, large_files: list[Path], corpus: list[Path]) -> dict:
    print(f"\n{'=' * 60}\n  Backend: {kind}\n{'=' * 60}", flush=True)
    out: dict = {"large_files": {}, "multifile": {}}
    for path in large_files:
        text = _read_text(path)
        name = path.name
        print(f"  [{kind}] {name} ({len(text.splitlines())} lines)…", flush=True)
        st = bench_semantic_tokens(kind, pyz, path, text)
        dg = bench_diagnostics(kind, pyz, path, text)
        op = bench_optimise(kind, pyz, path, text)
        ed = bench_heavy_edits(kind, pyz, path, text)
        out["large_files"][name] = {
            "lines": len(text.splitlines()),
            "semantic_tokens": st,
            "heavy_edits": ed,
            "diagnostics": dg,
            "optimise": op,
        }
        print(
            f"      tokens={_fmt(st['ms'])}ms({st['tokens']})  "
            f"diag={_fmt(dg['ms'])}ms({dg['count']})  "
            f"opt={_fmt(op['ms'])}ms  "
            f"edit={_fmt(ed['median_ms'])}ms",
            flush=True,
        )
    print(f"  [{kind}] multi-file corpus ({len(corpus)} files)…", flush=True)
    client = make_client(kind, pyz)
    try:
        out["multifile"] = bench_multifile(client, corpus, kind)
    finally:
        client.shutdown()
    mf = out["multifile"]
    print(
        f"      open+index={_fmt(mf['open_ms'])}ms  features={ {k: round(v) for k, v in mf['feature_totals_ms'].items()} }",
        flush=True,
    )
    return out


def gather_corpus(max_files: int = 120, max_bytes: int = 180_000) -> list[Path]:
    roots = [
        ROOT / "tmp/tcllib-2.0/modules/struct",
        ROOT / "tmp/tcllib-2.0/modules/math",
        ROOT / "tmp/tcllib-2.0/modules/snit",
        ROOT / "tmp/tcllib-2.0/modules/textutil",
        ROOT / "tmp/tcllib-2.0/modules/fileutil",
        ROOT / "tmp/tcllib-2.0/modules/json",
        ROOT / "tmp/tcllib-2.0/modules/csv",
    ]
    files: list[Path] = []
    for r in roots:
        if not r.is_dir():
            continue
        for p in sorted(r.rglob("*.tcl")):
            if "/build/" in str(p):
                continue
            if 0 < p.stat().st_size <= max_bytes:
                files.append(p)
    return files[:max_files]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--json", help="Write raw results to this JSON file")
    args = ap.parse_args()

    pyz = next((ROOT / "build").glob("tcl-lsp-server-*.pyz"), None)
    if pyz is None:
        print("ERROR: no zipapp under build/ — run `make zipapp-lsp` first.", file=sys.stderr)
        return 1
    binary = ROOT / "target" / "release" / "tcl-lsp-server"
    if not binary.exists():
        print("ERROR: no rust binary — run `make rust-server`.", file=sys.stderr)
        return 1

    large_files = [
        ROOT / "tmp/tcllib-2.0/modules/practcl/practcl.tcl",
        ROOT / "tmp/tcl9.0.3/library/http/http.tcl",
        ROOT / "tmp/tcllib-2.0/modules/tepam/tepam.tcl",
    ]
    large_files = [p for p in large_files if p.exists()]
    corpus = gather_corpus()
    print(f"Large files: {[p.name for p in large_files]}")
    print(f"Corpus: {len(corpus)} files, {sum(p.stat().st_size for p in corpus) // 1024} KB total")

    results = {}
    for kind in ("python", "rust"):
        results[kind] = run_backend(kind, pyz, large_files, corpus)

    if args.json:
        Path(args.json).write_text(json.dumps(results, indent=2), encoding="utf-8")
        print(f"\nWrote {args.json}")

    print_summary(results)
    return 0


def print_summary(results: dict) -> None:
    py, rs = results.get("python", {}), results.get("rust", {})

    def ratio(a, b):
        if a and b and b != 0:
            return f"{a / b:.1f}x"
        return "—"

    print("\n\n" + "=" * 78)
    print("  PERFORMANCE COMPARISON — Python vs Rust LSP backend")
    print("=" * 78)
    for name in py.get("large_files", {}):
        p = py["large_files"][name]
        r = rs.get("large_files", {}).get(name, {})
        print(f"\n  {name}  ({p['lines']} lines)")
        print(f"    {'metric':<24}{'python':>14}{'rust':>14}{'speedup':>12}")
        rows = [
            (
                "time→semantic tokens",
                p["semantic_tokens"]["ms"],
                r.get("semantic_tokens", {}).get("ms"),
            ),
            (
                "heavy edit (re-analyse)",
                p["heavy_edits"]["median_ms"],
                r.get("heavy_edits", {}).get("median_ms"),
            ),
            ("time→full diagnostics", p["diagnostics"]["ms"], r.get("diagnostics", {}).get("ms")),
            ("time→full optimisation", p["optimise"]["ms"], r.get("optimise", {}).get("ms")),
        ]
        for label, pv, rv in rows:
            ps = f"{pv:,.0f} ms" if isinstance(pv, (int, float)) else "—"
            rsv = f"{rv:,.0f} ms" if isinstance(rv, (int, float)) else "—"
            print(f"    {label:<24}{ps:>14}{rsv:>14}{ratio(pv, rv):>12}")

    pm, rm = py.get("multifile", {}), rs.get("multifile", {})
    if pm:
        print(f"\n  Multi-file codebase ({pm.get('files')} files)")
        print(f"    {'metric':<24}{'python':>14}{'rust':>14}{'speedup':>12}")
        print(
            f"    {'open + index':<24}{pm['open_ms']:>11,.0f} ms{rm.get('open_ms', 0):>11,.0f} ms"
            f"{ratio(pm['open_ms'], rm.get('open_ms')):>12}"
        )
        for feat in pm.get("feature_totals_ms", {}):
            pv = pm["feature_totals_ms"][feat]
            rv = rm.get("feature_totals_ms", {}).get(feat)
            ps = f"{pv:,.0f} ms"
            rsv = f"{rv:,.0f} ms" if isinstance(rv, (int, float)) else "—"
            print(f"    {feat + ' (all)':<24}{ps:>14}{rsv:>14}{ratio(pv, rv):>12}")
        for feat in pm.get("pos_totals_ms", {}):
            pv = pm["pos_totals_ms"][feat]
            rv = rm.get("pos_totals_ms", {}).get(feat)
            ps = f"{pv:,.0f} ms"
            rsv = f"{rv:,.0f} ms" if isinstance(rv, (int, float)) else "—"
            print(f"    {feat + ' (sample)':<24}{ps:>14}{rsv:>14}{ratio(pv, rv):>12}")


if __name__ == "__main__":
    raise SystemExit(main())
