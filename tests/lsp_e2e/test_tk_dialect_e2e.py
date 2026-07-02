"""Tk command availability, end-to-end over LSP.

Tk widget/window commands (`button`, `pack`, `wm`, the `ttk::` forms, …) are
gated three ways, verified here through completion:

1. **Loaded** — only offered once the file makes Tk available, either a
   `# tcl-dialect: tk` (`wish`) document or a `package require Tk`.  A plain
   `.tcl` script must not be offered Tk commands.
2. **Dialect** — never offered in the restricted embedded dialects (F5 iRules /
   iApps), where Tk does not exist, even if the source declares
   `package require Tk`.

Oracle: Tk ships as a separate package loaded via `package require Tk`; its
commands do not exist in a bare `tclsh` nor in the F5 rule engines.
"""

from __future__ import annotations

import pytest

from .harness import server_kind

# The Tk *loaded* gating (only offer Tk commands once `package require Tk` ran)
# is a native-server feature; the Python reference server does not implement it.
# The dialect-gating half (Tk absent from iRules / iApps) is additionally pinned
# backend-neutrally by the registry-contract baseline + `_registry_data.tcl`
# codegen tests, so this observable-over-LSP suite targets the Rust server.
pytestmark = pytest.mark.skipif(
    server_kind() != "rust", reason="Tk load-gating is a native-server feature"
)


def _labels(items):
    if isinstance(items, dict):
        items = items.get("items", [])
    return {i.get("label") for i in (items or [])}


def _complete(lsp_server, uri, src, line, char):
    lsp_server.open_ready(uri, src)
    return _labels(lsp_server.completion(uri, line, char))


class TestTkLoadedGating:
    def test_button_absent_in_plain_tcl(self, lsp_server, uri_factory):
        # No `package require Tk` — Tk commands must not surface.
        labels = _complete(lsp_server, uri_factory(), "# tcl-dialect: tcl8.6\nbutt\n", 1, 4)
        assert "button" not in labels

    def test_button_offered_after_package_require_tk(self, lsp_server, uri_factory):
        labels = _complete(
            lsp_server,
            uri_factory(),
            "# tcl-dialect: tcl8.6\npackage require Tk\nbutt\n",
            2,
            4,
        )
        assert "button" in labels

    # The `wish`/`tk`-dialect path (a `language_id: tk` document is implicitly
    # Tk-loaded) is not reachable via a `# tcl-dialect:` directive — `tk` is
    # deliberately not a selectable dialect profile — so it is covered by the
    # `tk_commands_offered_in_wish_tk_dialect` unit test in
    # `rust/tcl-lsp-core/src/completion.rs`.


class TestTkDialectGating:
    def test_button_never_offered_in_irules(self, lsp_server, uri_factory):
        # Even a stray `package require Tk` cannot make Tk valid in iRules.
        labels = _complete(
            lsp_server,
            uri_factory(),
            "# tcl-dialect: f5-irules\npackage require Tk\nbutt\n",
            2,
            4,
        )
        assert "button" not in labels

    def test_ttk_widget_absent_in_iapps(self, lsp_server, uri_factory):
        labels = _complete(
            lsp_server,
            uri_factory(),
            "# tcl-dialect: f5-iapps\npackage require Tk\nttk::butt\n",
            2,
            9,
        )
        assert "ttk::button" not in labels
