#!/usr/bin/env python3
"""Generate the Rust iRules event-description table from the Python registry.

Ports `compiler/registry/namespace_data.py`'s ``EVENT_DESCRIPTIONS`` to
``rust/tcl-registry/src/event_descriptions.rs`` (preserving Python dict
insertion order), which drives the ``description:`` line of
``f5 irule event-info``. Re-run whenever the Python prose moves.

Run from the repo root with the project venv active:

    python scripts/registry-audit/gen_event_descriptions.py
"""

from __future__ import annotations

from pathlib import Path


def rust_str(s: str) -> str:
    return (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )


def main() -> int:
    import tooling.f5.main  # noqa: F401  (loads the registry)
    from compiler.registry.namespace_data import EVENT_DESCRIPTIONS

    repo = Path(__file__).resolve().parents[2]
    out = repo / "rust" / "tcl-registry" / "src" / "event_descriptions.rs"

    lines = [
        "//! Per-event description prose for iRules events.",
        "//!",
        "//! Generated from the Python registry's `EVENT_DESCRIPTIONS`",
        "//! (`compiler/registry/namespace_data.py`) by",
        "//! `scripts/registry-audit/gen_event_descriptions.py`. Drives the",
        "//! `description:` line of `f5 irule event-info`.",
        "",
        "/// `(event_name, description)` for every iRules event that carries",
        "/// descriptive prose, in Python dict (insertion) order.",
        "pub(crate) const EVENT_DESCRIPTIONS: &[(&str, &str)] = &[",
    ]
    for name, desc in EVENT_DESCRIPTIONS.items():
        lines.append(f'    ("{rust_str(name)}", "{rust_str(desc)}"),')
    lines.append("];")
    lines.append("")
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {len(EVENT_DESCRIPTIONS)} descriptions to {out.relative_to(repo)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
