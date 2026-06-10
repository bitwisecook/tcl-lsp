"""Data-driven view models for the Textual explorer.

The TUI renders every analysis view as an interactive tree of
:class:`ViewNode` items built from the *same* serialised result the web GUI
consumes (``tooling.cli.serialise.serialise_result``).  Each node carries a
one-line ``label``, a ``detail`` table revealed when it is highlighted, and
optional ``children`` — so every item in every view expands to its full
information, the same contract the GUI's click/Space expansion offers.

Keeping these builders next to one serialisation (rather than re-deriving from
the IR/CFG objects) means the GUI and TUI show the same fields, and adding a
field is a one-place edit.  ``asm`` / ``wasm`` stay on the text renderer (they
are specialised disassembly, navigated rather than expanded), exactly as the
GUI keeps them out of the generic expander.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field


@dataclass
class ViewNode:
    """One expandable row: a summary ``label``, a ``detail`` table, children."""

    label: str
    detail: list[tuple[str, str]] = field(default_factory=list)
    children: list[ViewNode] = field(default_factory=list)
    style: str | None = None  # optional Rich style for the label


# Small formatting helpers shared by the builders.


def _yn(v: object) -> str:
    return "yes" if v else "no"


def _fmt_bound(v: object, positive: bool) -> str:
    """An interval endpoint: ``None`` is ±infinity."""
    if v is None:
        return "+inf" if positive else "-inf"
    return str(v)


def _rng(r: dict | None) -> str:
    if not r:
        return "?"
    return f"{r['startLine'] + 1}:{r['startCol'] + 1}  ({r['startOffset']}…{r['endOffset']})"


def _flags(p: dict) -> str:
    flags = []
    if p.get("hasBarrier"):
        flags.append("barrier")
    if p.get("hasUnknownCalls"):
        flags.append("unknown_calls")
    if p.get("writesGlobal"):
        flags.append("writes_global")
    return ", ".join(flags) or "—"


def _fmt_usedef(m: dict | None) -> str:
    if not m:
        return "{}"
    parts = []
    for name, info in sorted(m.items()):
        if isinstance(info, dict):
            s = f"{name}#{info.get('version', '?')}"
            if info.get("lattice"):
                s += f"={info['lattice']}"
            if info.get("type"):
                s += f":{info['type']}"
        else:
            s = f"{name}#{info}"
        parts.append(s)
    return "{" + ", ".join(parts) + "}"


# Green tree — every token, opaque {...}/[...] tokens nesting their region.

_GT_OPAQUE = ("STR", "CMD")


def _build_greentree(d: dict) -> list[ViewNode]:
    root = d.get("greentree")
    if not root:
        return [ViewNode("(green tree unavailable)", style="red")]

    def token_node(t: dict) -> ViewNode:
        opaque = t["type"] in _GT_OPAQUE
        one_line = t["text"].replace("\n", "⏎")
        preview = one_line if len(one_line) <= 40 else one_line[:40] + "…"
        label = f"{t['type']} {preview!r} [{t['startOffset']}:{t['endOffset']}]"
        detail = [
            ("token type", t["type"]),
            (
                "byte range",
                f"{t['startOffset']}…{t['endOffset']} "
                f"({max(0, t['endOffset'] - t['startOffset'])} bytes)",
            ),
            (
                "line:col",
                f"{t['startLine'] + 1}:{t['startCol'] + 1} → {t['endLine'] + 1}:{t['endCol'] + 1}",
            ),
        ]
        children: list[ViewNode] = []
        child = t.get("child")
        if child is not None:
            kind = f"{child['kind'].lower()} / {child['mode'].lower()}"
            if child["isError"]:
                kind += " (ERROR — unterminated)"
            detail.append(("region", kind))
            children = [token_node(ct) for ct in child["tokens"]]
        detail.append(("text", t["text"] or "(empty)"))
        return ViewNode(label, detail, children, style="cyan" if opaque else None)

    region = [
        ("region", f"{root['kind'].lower()} / {root['mode'].lower()}"),
        ("width", f"{root['width']} bytes"),
    ]
    label = f"{root['kind'].lower()} [{root['mode'].lower()}] w={root['width']}"
    return [ViewNode(label, region, [token_node(t) for t in root["tokens"]])]


# IR


def _ir_nodes(nodes: list[dict] | None) -> list[ViewNode]:
    out = []
    for n in nodes or []:
        detail = [
            ("kind", n.get("kind", "")),
            ("summary", n["summary"]),
            ("range", _rng(n.get("range"))),
        ]
        children = []
        for c in n.get("children", []) or []:
            children.append(ViewNode(c["label"] + ":", [], _ir_nodes(c["body"]), style="blue"))
        out.append(ViewNode(n["summary"], detail, children))
    return out


def _build_ir(d: dict) -> list[ViewNode]:
    ir = d["ir"]
    nodes = [ViewNode("top-level", [], _ir_nodes(ir["topLevel"]), style="cyan")]
    for name, proc in (ir.get("procedures") or {}).items():
        params = " ".join(proc.get("params", []))
        header = f"{name} {{{params}}}" if params else f"{name} {{}}"
        nodes.append(
            ViewNode(
                header, [("range", _rng(proc.get("range")))], _ir_nodes(proc["body"]), style="cyan"
            )
        )
    return nodes


# CFG (pre-/post-SSA)


def _term_label(t: dict | None) -> str:
    if not t:
        return "term <none>"
    if t["type"] == "goto":
        return f"term goto {t.get('target')}"
    if t["type"] == "branch":
        return (
            f"term branch {t.get('condition', '')} → {t.get('trueTarget')}/{t.get('falseTarget')}"
        )
    if t["type"] == "return":
        return "term return" + (f" {t['value']}" if t.get("value") else "")
    return "term"


def _build_cfg(funcs: list[dict] | None, *, post: bool) -> list[ViewNode]:
    out = []
    for f in funcs or []:
        blocks = []
        for b in f["blocks"]:
            items: list[ViewNode] = []
            if post:
                for p in b.get("phis", []):
                    inc = ", ".join(f"{k}:{v}" for k, v in (p.get("incoming") or {}).items())
                    items.append(
                        ViewNode(
                            f"phi {p['name']}#{p['version']} ← {inc}",
                            [("type", p.get("type") or "?"), ("incoming", inc)],
                            style="magenta",
                        )
                    )
            for s in b["statements"]:
                detail = [("summary", s["summary"]), ("range", _rng(s.get("range")))]
                if post:
                    detail.append(("uses", _fmt_usedef(s.get("uses"))))
                    detail.append(("defs", _fmt_usedef(s.get("defs"))))
                items.append(ViewNode(s["summary"], detail))
            term = b.get("terminator")
            items.append(
                ViewNode(
                    _term_label(term), [("range", _rng((term or {}).get("range")))], style="blue"
                )
            )
            tags = (" [entry]" if b.get("isEntry") else "") + (
                " [unreachable]" if b.get("isUnreachable") else ""
            )
            blocks.append(ViewNode(f"block {b['name']}{tags}", [], items, style="bold"))
        if post and f.get("analysis"):
            a = f["analysis"]
            asub = []
            for br in a.get("constantBranches", []):
                asub.append(
                    ViewNode(
                        f"const branch {br['block']}: always {br['value']}",
                        [
                            ("condition", br.get("condition", "")),
                            ("take", br.get("takenTarget", "")),
                        ],
                        style="blue",
                    )
                )
            for ds in a.get("deadStores", []):
                asub.append(
                    ViewNode(
                        f"dead store {ds['block']} {ds['variable']}#{ds['version']}",
                        [("block", ds["block"]), ("stmt index", ds.get("stmtIndex", "?"))],
                        style="yellow",
                    )
                )
            if a.get("unreachableBlocks"):
                asub.append(
                    ViewNode(
                        "unreachable: " + ", ".join(a["unreachableBlocks"]),
                        [("blocks", ", ".join(a["unreachableBlocks"]))],
                        style="magenta",
                    )
                )
            for name, ty in (a.get("inferredTypes") or {}).items():
                asub.append(
                    ViewNode(f"{name}: {ty}", [("value", name), ("type", ty)], style="green")
                )
            if asub:
                blocks.append(ViewNode("analysis", [], asub, style="bold"))
        out.append(
            ViewNode(
                f"function {f['name']} (entry={f['entry']}, {f['blockCount']} blocks)",
                [],
                blocks,
                style="cyan",
            )
        )
    return out or [ViewNode("(no functions)", style="dim")]


# Loops / types / intervals / bounds / rendered (per-function tables)


def _build_loops(d: dict) -> list[ViewNode]:
    out = []
    for f in d.get("loops", []):
        if not f["loops"]:
            continue
        loops = [
            ViewNode(
                f"header {lp['header']} (depth {lp['depth']}, {lp['blockCount']} blocks)",
                [
                    ("header block", lp["header"]),
                    ("nesting depth", lp["depth"]),
                    ("blocks", ", ".join(lp["blocks"])),
                    ("latches", ", ".join(lp["latches"]) or "—"),
                ],
                style="blue",
            )
            for lp in f["loops"]
        ]
        out.append(ViewNode(f"function {f['name']}", [], loops, style="cyan"))
    return out or [ViewNode("(no natural loops)", style="dim")]


def _build_types(d: dict) -> list[ViewNode]:
    out = []
    for f in d.get("types", []):
        if not f["entries"]:
            continue
        ents = []
        for e in f["entries"]:
            label = (
                e["variable"]
                if e["variable"].startswith("(")
                else f"{e['variable']}#{e['version']}"
            )
            ents.append(
                ViewNode(
                    f"{label}: {e['type']}",
                    [
                        ("variable", e["variable"]),
                        ("version", e["version"]),
                        ("kind", e["kind"]),
                        ("type", e["type"]),
                    ],
                )
            )
        out.append(ViewNode(f"function {f['name']}", [], ents, style="cyan"))
    return out or [ViewNode("(no inferred types)", style="dim")]


def _build_intervals(d: dict) -> list[ViewNode]:
    out = []
    for f in d.get("intervals", []):
        if not f["entries"]:
            continue
        ents = []
        for e in f["entries"]:
            lo, hi = _fmt_bound(e["lo"], False), _fmt_bound(e["hi"], True)
            ents.append(
                ViewNode(
                    f"{e['variable']}#{e['version']}: [{lo}, {hi}]",
                    [
                        ("variable", e["variable"]),
                        ("version", e["version"]),
                        ("lower bound", lo),
                        ("upper bound", hi),
                    ],
                )
            )
        out.append(ViewNode(f"function {f['name']}", [], ents, style="cyan"))
    return out or [ViewNode("(no bounded ranges)", style="dim")]


def _build_bounds(d: dict) -> list[ViewNode]:
    out = []
    for f in d.get("bounds", []):
        if not f["findings"] and not f["divzero"]:
            continue
        items = []
        for b in f["findings"]:
            lo, hi = _fmt_bound(b["lo"], False), _fmt_bound(b["hi"], True)
            items.append(
                ViewNode(
                    f"{b['code']} {b['command']} ${b['indexVar']} in [{lo}, {hi}] vs length {b['length']}",
                    [
                        ("code", b["code"]),
                        ("command", b["command"]),
                        ("index var", "$" + b["indexVar"]),
                        ("index range", f"[{lo}, {hi}]"),
                        ("container length", b["length"]),
                        ("reason", b["reason"]),
                    ],
                    style="yellow",
                )
            )
        for dz in f["divzero"]:
            items.append(
                ViewNode(
                    f"{dz['code']} '{dz['op']}' divisor provably 0",
                    [
                        ("code", dz["code"]),
                        ("operator", dz["op"]),
                        ("reason", "divisor interval is exactly [0, 0]"),
                    ],
                    style="yellow",
                )
            )
        out.append(ViewNode(f"function {f['name']}", [], items, style="cyan"))
    return out or [ViewNode("(no provable out-of-range / divide-by-zero)", style="dim")]


def _build_rendered(d: dict) -> list[ViewNode]:
    out = []
    for f in d.get("renderedProperties", []) or []:
        if not f["entries"]:
            continue
        ents = [
            ViewNode(
                f"{e['variable']}#{e['version']}",
                [
                    ("variable", e["variable"]),
                    ("version", e["version"]),
                    ("may", ", ".join(e["may"]) or "—"),
                    ("must", ", ".join(e["must"]) or "—"),
                ],
            )
            for e in f["entries"]
        ]
        out.append(ViewNode(f"function {f['name']}", [], ents, style="cyan"))
    return out or [ViewNode("(no rendered-property flags)", style="dim")]


# Data flow / interprocedural


def _build_dataflow(d: dict) -> list[ViewNode]:
    df = d.get("dataflow")
    if not df or not df.get("functions"):
        return [ViewNode("(no data-flow information)", style="dim")]
    out = []
    for f in df["functions"]:
        nodes = []
        for a in f.get("aliases", []):
            nodes.append(
                ViewNode(
                    f"alias {a['localName']} ↔ {a['targetName']}",
                    [
                        ("local", a.get("localName", "")),
                        ("target", a.get("targetName", "")),
                        ("reason", a.get("reason", "")),
                    ],
                    style="orange",
                )
            )
        for n in f["nodes"]:
            dead = " (DEAD)" if n.get("isDead") else ""
            label = f"{n['name']}#{n['version']} [{n['defKind']} in {n['block']}]{dead}"
            detail = [
                ("ssa value", f"{n['name']}#{n['version']}"),
                ("def kind", n["defKind"]),
                ("block", n["block"]),
                ("dead", _yn(n.get("isDead"))),
            ]
            if n.get("useCount") is not None:
                detail.append(("uses", n["useCount"]))
            if n.get("lattice"):
                detail.append(("lattice", n["lattice"]))
            if n.get("typeInfo"):
                detail.append(("type", n["typeInfo"]))
            nodes.append(ViewNode(label, detail, style="yellow" if n.get("isDead") else "green"))
        sm = f.get("summary", {})
        out.append(
            ViewNode(
                f"function {f['name']} ({sm.get('totalDefs', 0)} defs, {sm.get('totalUses', 0)} uses)",
                [],
                nodes,
                style="cyan",
            )
        )
    return out


def _build_interproc(d: dict) -> list[ViewNode]:
    out = []
    for p in d.get("interprocedural", []):
        if p.get("kind") == "method":
            detail = [
                ("method kind", p.get("methodKind", "method")),
                ("pure", _yn(p.get("pure"))),
                ("calls", ", ".join(p.get("calls") or []) or "—"),
            ]
            if p.get("writesInstanceVars"):
                detail.append(("writes instance vars", ", ".join(p["writesInstanceVars"])))
            detail.append(("flags", _flags(p)))
            out.append(ViewNode(f"{p['name']} (method)", detail, style="cyan"))
        else:
            detail = [
                ("arity", p.get("arity")),
                ("pure", _yn(p.get("pure"))),
                ("foldable", _yn(p.get("foldable"))),
                ("return shape", p.get("returnShape", "")),
                ("calls", ", ".join(p.get("calls") or []) or "—"),
                ("flags", _flags(p)),
            ]
            out.append(ViewNode(f"{p['name']} arity={p.get('arity')}", detail, style="cyan"))
    return out or [ViewNode("(no procedures)", style="dim")]


# Optimiser / GVN / shimmer / taint / iRules / event order / callouts


def _build_opt(d: dict) -> list[ViewNode]:
    out = [
        ViewNode(
            f"{o['code']} {o['message']} → {o['replacement']}",
            [
                ("code", o["code"]),
                ("message", o["message"]),
                ("replacement", o["replacement"]),
                ("range", _rng(o.get("range"))),
            ],
            style="green",
        )
        for o in d.get("optimisations", [])
    ]
    return out or [ViewNode("(no optimiser rewrites)", style="dim")]


def _build_gvn(d: dict) -> list[ViewNode]:
    out = [
        ViewNode(
            f"{w['code']} {w.get('expression', '')}",
            [
                ("code", w["code"]),
                ("message", w.get("message", "")),
                ("expression", w.get("expression", "")),
                ("first seen", _rng(w.get("firstRange"))),
                ("range", _rng(w.get("range"))),
            ],
            style="green",
        )
        for w in d.get("gvn", [])
    ]
    return out or [ViewNode("(no redundant computations)", style="dim")]


def _build_shimmer(d: dict) -> list[ViewNode]:
    out = [
        ViewNode(
            f"{w['code']} {w['message']}",
            [
                ("code", w["code"]),
                ("severity", w.get("severity", "warning")),
                ("message", w["message"]),
                ("range", _rng(w.get("range"))),
            ],
            style="yellow",
        )
        for w in d.get("shimmer", [])
    ]
    return out or [ViewNode("(no shimmer warnings)", style="dim")]


def _build_taint(d: dict) -> list[ViewNode]:
    out = []
    for w in d.get("taintWarnings", []):
        detail = [
            ("code", w["code"]),
            ("severity", w.get("severity", "warning")),
            ("message", w["message"]),
        ]
        if w.get("variable"):
            detail.append(("variable", w["variable"]))
        if w.get("sinkCommand"):
            detail.append(("sink command", w["sinkCommand"]))
        detail.append(("range", _rng(w.get("range"))))
        out.append(ViewNode(f"{w['code']} {w['message']}", detail, style="red"))
    for f in d.get("taintTracking", []):
        ents = [
            ViewNode(
                f"{e['variable']}#{e['version']}: {e['taint']}",
                [("variable", e["variable"]), ("version", e["version"]), ("taint", e["taint"])],
            )
            for e in f["entries"]
        ]
        out.append(ViewNode(f"tracking {f['name']}", [], ents, style="cyan"))
    return out or [ViewNode("(no tainted data flows)", style="dim")]


def _build_irules(d: dict) -> list[ViewNode]:
    out = []
    for e in d.get("eventOrder", []):
        eff = e["base_priority"] + e["priority_offset"]
        out.append(
            ViewNode(
                f"{e['event']} eff={eff}",
                [
                    ("event", e["event"]),
                    ("effective priority", eff),
                    ("base", e["base_priority"]),
                    ("offset", e["priority_offset"]),
                    ("multiplicity", e.get("multiplicity", "once")),
                    ("range", _rng(e.get("range"))),
                ],
                style="blue",
            )
        )
    for w in d.get("irulesFlow", []):
        out.append(
            ViewNode(
                f"{w['code']} {w['message']}",
                [("code", w["code"]), ("message", w["message"]), ("range", _rng(w.get("range")))],
                style="yellow",
            )
        )
    return out or [ViewNode("(no iRules events)", style="dim")]


def _build_event_order(d: dict) -> list[ViewNode]:
    events = sorted(
        d.get("eventOrder", []),
        key=lambda e: (-(e["base_priority"] + e["priority_offset"]), e["event"]),
    )
    out = []
    for e in events:
        eff = e["base_priority"] + e["priority_offset"]
        mult = (
            f" [{e['multiplicity']}]"
            if e.get("multiplicity") and e["multiplicity"] != "once"
            else ""
        )
        out.append(
            ViewNode(
                f"{e['event']} eff={eff}{mult}",
                [
                    ("event", e["event"]),
                    ("effective priority", eff),
                    ("base priority", e["base_priority"]),
                    ("priority offset", e["priority_offset"]),
                    ("multiplicity", e.get("multiplicity", "once")),
                    ("range", _rng(e.get("range"))),
                ],
                style="blue",
            )
        )
    return out or [ViewNode("(no event-order data)", style="dim")]


def _build_callouts(d: dict) -> list[ViewNode]:
    out = [
        ViewNode(
            f"{a.get('kind', '')}: {a.get('label', '')}",
            [
                ("kind", a.get("kind", "")),
                ("severity", a.get("severity", "")),
                ("label", a.get("label", "")),
                ("range", _rng(a.get("range"))),
            ],
        )
        for a in d.get("annotations", [])
    ]
    return out or [ViewNode("(no salient annotations)", style="dim")]


# Registry: view id → builder.  ``cfg``/``ssa`` share one builder via a flag.
VIEW_BUILDERS: dict[str, Callable[[dict], list[ViewNode]]] = {
    "greentree": _build_greentree,
    "ir": _build_ir,
    "cfg": lambda d: _build_cfg(d.get("cfgPreSsa"), post=False),
    "ssa": lambda d: _build_cfg(d.get("cfgPostSsa"), post=True),
    "loops": _build_loops,
    "types": _build_types,
    "intervals": _build_intervals,
    "bounds": _build_bounds,
    "dataflow": _build_dataflow,
    "interproc": _build_interproc,
    "rendered": _build_rendered,
    "opt": _build_opt,
    "gvn": _build_gvn,
    "shimmer": _build_shimmer,
    "taint": _build_taint,
    "irules": _build_irules,
    "event-order": _build_event_order,
    "callouts": _build_callouts,
}

# Views that the TUI renders as an interactive tree (everything with a
# builder).  ``asm`` / ``wasm`` deliberately stay on the text renderer.
TREE_VIEWS = frozenset(VIEW_BUILDERS)


def build_view(view: str, data: dict) -> list[ViewNode]:
    """Build the :class:`ViewNode` forest for *view* from serialised *data*."""
    builder = VIEW_BUILDERS.get(view)
    return builder(data) if builder else []
