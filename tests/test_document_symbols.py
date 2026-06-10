"""BigIP document-outline internals.

The Tcl and TclOO document-symbol provider behaviour is exercised end-to-end
against the packaged server in ``tests/lsp_e2e/test_document_symbols_e2e.py``.
What remains here is ``get_bigip_document_symbols`` — an internal helper driven
by the BigIP config dialect with no plain ``textDocument/documentSymbol``
surface in the default Tcl path — covering the module/kind grouping and the
non-empty-name invariant the VS Code client requires.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features._bigip_symbols import get_bigip_document_symbols


class TestBigipDocumentSymbols:
    def test_outline_groups_by_module_and_kind(self):
        source = (
            "ltm pool /Common/p1 { description x }\n"
            "ltm pool /Common/p2 { }\n"
            "ltm virtual /Common/vs { destination /Common/1.1.1.1:80 }\n"
            "pem policy /Common/pp { }\n"
            "net vlan /Common/v { tag 100 }\n"
        )
        out = get_bigip_document_symbols(source)
        names = sorted(s.name for s in out)
        assert names == ["ltm", "net", "pem"]
        ltm = next(s for s in out if s.name == "ltm")
        kinds = sorted(c.name for c in (ltm.children or []))
        assert "pool" in kinds and "virtual" in kinds
        pool_kind = next(c for c in (ltm.children or []) if c.name == "pool")
        pool_paths = sorted(o.name for o in (pool_kind.children or []))
        assert pool_paths == ["/Common/p1", "/Common/p2"]

    def test_outline_empty_for_non_bigip_input(self):
        # Garbage input parses to an empty BigipConfig (no outline).
        assert get_bigip_document_symbols("this is not a bigip config\n") == []

    def test_global_singletons_have_non_empty_names(self):
        # Global singletons (``auth password-policy``, ``net self-allow``,
        # ``sys diags ihealth``) carry no path.  An empty ``name`` makes the
        # VS Code client reject the *entire* outline ("name must not be
        # falsy"), so every symbol — at every depth — must be non-empty.
        source = (
            "auth password-policy {\n"
            "    lockout-duration 10\n"
            "}\n"
            "net self-allow {\n"
            "    defaults {\n"
            "        tcp:443\n"
            "    }\n"
            "}\n"
            "sys diags ihealth {\n"
            "    user admin\n"
            "}\n"
            "ltm pool /Common/p1 { }\n"
        )
        out = get_bigip_document_symbols(source)

        def all_names(symbols):
            for sym in symbols:
                yield sym.name
                yield from all_names(sym.children or [])

        names = list(all_names(out))
        assert names, "expected a non-empty outline"
        assert all(names), f"empty symbol name(s) present: {names!r}"

        # The nameless singletons fall back to their kind label.
        auth = next(s for s in out if s.name == "auth")
        pwd = next(c for c in (auth.children or []) if c.name == "password-policy")
        assert [o.name for o in (pwd.children or [])] == ["password-policy"]
