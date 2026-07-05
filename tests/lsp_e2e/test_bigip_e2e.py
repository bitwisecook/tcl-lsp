# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""F5 BIG-IP ``*.conf`` handling, end-to-end against the packaged server.

Two 1.11.0 fixes are pinned here against the real server, keyed on the
canonical BIG-IP basename (``bigip.conf``, ``bigip_base.conf``, …):

  #534 — the document outline must never emit an empty symbol ``name`` (VS Code
         rejects the *entire* outline when any name is falsy);
  #571 — the general Tcl analyser must never run on BIG-IP config text, so its
         encrypted-string markers (``$M$…$``) are not mis-read as Tcl variable
         references (W210) and no general Tcl diagnostics are published.

These run on the dedicated ``lsp_server_bigip`` fixture: opening a BIG-IP conf
switches the whole server's dialect, so it must not share the plain-Tcl server.
"""

from __future__ import annotations

from ._lsp_helpers import flatten_symbols


def _codes(diags) -> set[str]:
    return {str(d.get("code")) for d in (diags or [])}


# A representative BIG-IP config: nameless global singletons (which previously
# produced empty outline names) plus a pool, plus an embedded iRule whose body
# holds an encrypted-string marker the Tcl analyser would mis-read as ``$M``.
_BIGIP_CONF = """\
auth password-policy {
    lockout-duration 10
}
net self-allow {
    defaults {
        tcp:443
    }
}
sys diags ihealth {
    user admin
}
ltm pool /Common/p1 {
    members {
        1.2.3.4:80 { }
    }
}
ltm rule /Common/r {
    when HTTP_REQUEST {
        log local0. "$M$mn$9uYx+bTjLD8YSGZA="
    }
}
"""


class TestBigipDocumentOutline:
    """Issue #534 — every outline symbol carries a non-empty name."""

    def test_outline_symbols_all_have_non_empty_names(self, lsp_server_bigip, bigip_uri_factory):
        uri = bigip_uri_factory()
        lsp_server_bigip.open_document(uri, _BIGIP_CONF)
        # The BIG-IP path doesn't emit the Tcl ``workspace_state.update`` marker,
        # so wait on the published (BIG-IP) diagnostics instead of open_ready.
        lsp_server_bigip.await_diagnostics(uri, version=1)
        syms = lsp_server_bigip.document_symbols(uri) or []
        names = [s.get("name") for s in flatten_symbols(syms)]
        assert names, "expected a non-empty BIG-IP outline"
        assert all(names), f"empty symbol name(s) in outline: {names!r}"

    def test_nameless_singleton_falls_back_to_kind_label(self, lsp_server_bigip, bigip_uri_factory):
        uri = bigip_uri_factory()
        lsp_server_bigip.open_document(uri, _BIGIP_CONF)
        lsp_server_bigip.await_diagnostics(uri, version=1)
        names = {s.get("name") for s in flatten_symbols(lsp_server_bigip.document_symbols(uri))}
        # The nameless ``auth password-policy`` singleton surfaces by its module
        # / kind labels rather than an empty string.
        assert "auth" in names, names


class TestBigipDiagnosticSuppression:
    """Issue #571 — no general Tcl diagnostics on BIG-IP config text."""

    def test_encrypted_marker_does_not_raise_tcl_diagnostics(
        self, lsp_server_bigip, bigip_uri_factory
    ):
        uri = bigip_uri_factory()
        lsp_server_bigip.open_document(uri, _BIGIP_CONF)
        diags = lsp_server_bigip.await_diagnostics(uri, version=1)
        codes = _codes(diags)
        # ``$M$…$`` would be a W210 (read-before-set) if analysed as Tcl; the
        # whole W/E/S general-Tcl family must be absent on a BIG-IP conf.
        assert "W210" not in codes, codes
        assert not any(c.startswith(("W", "E", "S")) for c in codes), (
            f"general Tcl diagnostics leaked on BIG-IP conf: {codes}"
        )

    def test_bare_set_arity_not_flagged_on_bigip_conf(self, lsp_server_bigip, bigip_uri_factory):
        # A line that, as Tcl, is a bare ``set`` (E002 arity) — but here it is
        # BIG-IP config text, so the Tcl arity checker must not fire.
        uri = bigip_uri_factory()
        lsp_server_bigip.open_document(uri, "ltm pool /Common/p { }\nset\n")
        diags = lsp_server_bigip.await_diagnostics(uri, version=1)
        assert "E002" not in _codes(diags), _codes(diags)
