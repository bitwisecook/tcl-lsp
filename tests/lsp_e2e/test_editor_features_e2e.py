"""Code lens, document links, formatting, and inlay hints — end-to-end.

More of the cross-implementation conformance surface (raw JSON-RPC against the
packaged server; a spec the Rust/Zed port must meet).  Ported from
``tests/test_code_lens.py``, ``tests/test_document_links.py``,
``tests/test_formatter.py``, and ``tests/test_inlay_hints.py``.
"""

from __future__ import annotations


def _full_text(edits) -> str | None:
    """A whole-document formatter returns a single replace edit; return its text."""
    if not edits:
        return None
    return edits[0]["newText"]


class TestCodeLens:
    def test_proc_gets_reference_count_lens(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return }\ngreet\ngreet\n")
        lenses = lsp_server.code_lens(uri) or []
        # A lens sits on the proc name (line 0) carrying its qualified name.
        on_proc = [lens for lens in lenses if lens["range"]["start"]["line"] == 0]
        assert on_proc
        assert any((lens.get("data") or {}).get("qname") == "::greet" for lens in on_proc)

    def test_no_lens_without_procs(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\nputs $x\n")
        assert (lsp_server.code_lens(uri) or []) == []

    @staticmethod
    def _resolve_for_qname(lsp_server, uri, lenses, qname):
        lens = next(lens for lens in lenses if (lens.get("data") or {}).get("qname") == qname)
        return lsp_server.code_lens_resolve(lens)

    def test_count_matches_reference_list_for_forward_call(self, lsp_server, uri_factory):
        """The resolved lens count equals the reference list — issue #637.

        A call written before the proc's definition resolves to ``None`` at
        analysis time, so the old count path reported "0 references" while
        Find All References listed the call. The resolved title must match.
        """
        # A workspace-unique proc name keeps the workspace-wide count isolated
        # from other documents the shared server session has indexed.
        uri = uri_factory()
        lsp_server.open_ready(uri, "fwd637\nproc fwd637 {} { return }\n")
        lenses = lsp_server.code_lens(uri) or []
        resolved = self._resolve_for_qname(lsp_server, uri, lenses, "::fwd637")
        assert resolved["command"]["title"] == "1 reference"
        # The reference list at the proc name (declaration excluded) agrees.
        refs = lsp_server.references(uri, 1, 5, include_declaration=False) or []
        assert len(refs) == 1
        assert resolved["command"]["title"].startswith(str(len(refs)))

    def test_unresolved_call_scoped_to_namespace(self, lsp_server, uri_factory):
        """An unresolved bare call is credited to the proc Tcl picks — PR #644.

        With ``::a::foo`` and ``::b::foo`` both defined and a forward ``foo``
        call inside namespace ``a``, the call belongs to ``::a::foo`` (1) and
        ``::b::foo`` must stay at 0 rather than showing a phantom reference.
        """
        uri = uri_factory()
        lsp_server.open_ready(
            uri,
            "namespace eval nsa644 {\n    dup644\n    proc dup644 {} {}\n}\n"
            "namespace eval nsb644 {\n    proc dup644 {} {}\n}\n",
        )
        lenses = lsp_server.code_lens(uri) or []
        a = self._resolve_for_qname(lsp_server, uri, lenses, "::nsa644::dup644")
        b = self._resolve_for_qname(lsp_server, uri, lenses, "::nsb644::dup644")
        assert a["command"]["title"] == "1 reference"
        assert b["command"]["title"] == "0 references"


class TestDocumentLinks:
    def test_source_command_is_linked(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "source other.tcl\nputs done\n")
        links = lsp_server.document_links(uri) or []
        assert any("other.tcl" in (link.get("tooltip") or "") for link in links)

    def test_package_require_is_linked(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "package require Tcl\n")
        links = lsp_server.document_links(uri) or []
        assert any(link["range"]["start"]["line"] == 0 for link in links)


class TestFormatting:
    def test_full_document_formatting_normalises_spacing(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc  f {}    {\nputs   hi\n}\n")
        formatted = _full_text(lsp_server.formatting(uri))
        assert formatted is not None
        assert "proc f {} {" in formatted
        assert "    puts hi" in formatted

    def test_already_formatted_is_stable(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc f {} {\n    puts hi\n}\n"
        lsp_server.open_ready(uri, src)
        edits = lsp_server.formatting(uri) or []
        # No change needed → either no edits, or an edit that reproduces the text.
        if edits:
            assert _full_text(edits) == src

    def test_range_formatting_returns_edits(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc  f {}    {\nputs   hi\n}\n")
        edits = lsp_server.range_formatting(uri, (0, 0), (2, 1)) or []
        assert edits
        assert "proc f {} {" in edits[0]["newText"]


class TestInlayHints:
    def test_provider_responds_with_a_list(self, lsp_server, uri_factory):
        # Inlay hints are gated off by default (both inlay toggles); the
        # provider must still answer with a well-formed (here empty) list, never
        # an error.  This pins the default-off contract the Rust port must match.
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\n")
        result = lsp_server.inlay_hints(uri, (0, 0), (1, 0))
        assert isinstance(result, list)

    def test_hint_kinds_are_valid_when_present(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc add {a b} { expr {$a + $b} }\nadd 1 2\n")
        for hint in lsp_server.inlay_hints(uri, (0, 0), (2, 0)) or []:
            assert hint.get("kind") in (1, 2)  # Type, Parameter


def _label_text(hint: dict) -> str:
    """An inlay hint's ``label`` is a string or a list of ``{value}`` parts."""
    label = hint.get("label")
    if isinstance(label, list):
        return "".join(str(part.get("value", "")) for part in label)
    return str(label or "")


def _param_labels_on_line(hints, line: int) -> list[tuple[int, str]]:
    """``(character, label)`` of every parameter-kind hint on ``line``."""
    out: list[tuple[int, str]] = []
    for h in hints or []:
        if h.get("kind") != 2:  # InlayHintKind.Parameter
            continue
        pos = h.get("position") or {}
        char = pos.get("character")
        if pos.get("line") == line and char is not None:
            out.append((char, _label_text(h)))
    return sorted(out)


class TestInlayHintOptionalPositionals:
    """Optional-positional parameter labelling — issue #510, end-to-end.

    The synopsis ``puts ?-nonewline? ?channelId? string`` has an *optional*
    positional (``channelId``) before a *required* trailing one (``string``).
    A one-positional call binds that argument to the required ``string`` slot,
    not the leftmost optional name, and the documentation placeholders
    ``?options?`` / ``?switches?`` are never emitted as parameter labels.

    These run against ``lsp_server_inlay`` (inlay hints on) so the provider
    actually produces hints — the default server keeps them off.
    """

    def test_single_positional_binds_required_string_slot(self, lsp_server_inlay, uri_factory):
        uri = uri_factory()
        lsp_server_inlay.open_ready(uri, "puts hello\n")
        labels = _param_labels_on_line(lsp_server_inlay.inlay_hints(uri, (0, 0), (1, 0)), 0)
        names = [name for _char, name in labels]
        # The single argument is the required ``string`` — never the optional
        # ``channelId`` (the pre-fix regression).
        assert "string:" in names, labels
        assert "channelId:" not in names, labels

    def test_two_positionals_label_channel_then_string(self, lsp_server_inlay, uri_factory):
        uri = uri_factory()
        lsp_server_inlay.open_ready(uri, "puts $chan hello\n")
        labels = _param_labels_on_line(lsp_server_inlay.inlay_hints(uri, (0, 0), (1, 0)), 0)
        names = [name for _char, name in labels]
        # Both slots are filled now, left to right.
        assert names == ["channelId:", "string:"], labels

    def test_leading_flag_does_not_shift_required_slot(self, lsp_server_inlay, uri_factory):
        # ``-nonewline`` is an option flag, not a positional; the one real
        # positional still binds to ``string``.
        uri = uri_factory()
        lsp_server_inlay.open_ready(uri, "puts -nonewline hello\n")
        names = [
            name
            for _char, name in _param_labels_on_line(
                lsp_server_inlay.inlay_hints(uri, (0, 0), (1, 0)), 0
            )
        ]
        assert "string:" in names, names
        assert "channelId:" not in names, names

    def test_no_documentation_placeholder_labels(self, lsp_server_inlay, uri_factory):
        # ``lsearch ?options? list pattern`` — the ``?options?`` placeholder is
        # the flag group, not a positional, so the real positionals label
        # ``list`` / ``pattern`` and no ``options``/``switches`` hint appears.
        uri = uri_factory()
        lsp_server_inlay.open_ready(uri, "lsearch $mylist needle\n")
        names = [
            name
            for _char, name in _param_labels_on_line(
                lsp_server_inlay.inlay_hints(uri, (0, 0), (1, 0)), 0
            )
        ]
        assert "list:" in names, names
        assert "pattern:" in names, names
        assert not any(n in ("options:", "switches:") for n in names), names


class TestInlayToggleIndependenceE2E:
    """The single ``inlayHints`` toggle was split into two independent options,
    both off by default (PR #643): ``inlayTypeHints`` (inferred variable types /
    format specifiers) and ``inlayParameterHints`` (the verbose proc/method
    parameter-name labels).  Enabling one must not turn on the other.

    These drive the *shared* server through ``config_session`` (which settles on
    the resolved config and restores it afterwards), so the default-off contract
    the other suites rely on is left untouched.
    """

    def test_parameter_hints_only_produce_labels_no_type_hints(self, lsp_server, uri_factory):
        uri = uri_factory()
        with lsp_server.config_session(
            {"features": {"inlayTypeHints": False, "inlayParameterHints": True}},
            settle=lambda c: (
                c["features"].get("inlayParameterHints") is True
                and c["features"].get("inlayTypeHints") is False
            ),
            settle_uri=uri,
        ):
            # ``set x 42`` would yield a Type-kind hint (``: int``) if type hints
            # leaked on, so the no-Type assertion below is a real check, not a
            # vacuous one; ``puts hello`` carries the ``string:`` parameter label.
            lsp_server.open_ready(uri, "set x 42\nputs hello\n")
            hints = lsp_server.inlay_hints(uri, (0, 0), (2, 0)) or []
            assert any(h.get("kind") == 2 for h in hints), hints  # Parameter present
            assert all(h.get("kind") != 1 for h in hints), hints  # no Type leaked in

    def test_type_hints_only_emit_no_parameter_labels(self, lsp_server, uri_factory):
        uri = uri_factory()
        with lsp_server.config_session(
            {"features": {"inlayTypeHints": True, "inlayParameterHints": False}},
            settle=lambda c: (
                c["features"].get("inlayTypeHints") is True
                and c["features"].get("inlayParameterHints") is False
            ),
            settle_uri=uri,
        ):
            # ``set x 42`` carries the ``: int`` type hint; ``puts hello`` would
            # carry the ``string:`` parameter label if parameter hints leaked on.
            lsp_server.open_ready(uri, "set x 42\nputs hello\n")
            hints = lsp_server.inlay_hints(uri, (0, 0), (2, 0)) or []
            assert any(h.get("kind") == 1 for h in hints), hints  # Type present
            assert all(h.get("kind") != 2 for h in hints), hints  # no Parameter leaked in


class TestInlayLegacyAliasE2E:
    """The retired ``features.inlayHints`` key is accepted as a backward-
    compatible alias that enables *type* hints only — parameter hints stay off
    (PR #643), so an existing explicit opt-in keeps working after the rename.
    """

    def test_legacy_inlay_hints_alias_enables_type_only(self, lsp_server, uri_factory):
        uri = uri_factory()
        with lsp_server.config_session(
            {"features": {"inlayHints": True}},
            settle=lambda c: c["features"].get("inlayTypeHints") is True,
            settle_uri=uri,
        ):
            eff = lsp_server.effective_config(uri)
            # The alias resolves to type hints; parameter hints remain off.
            assert eff["features"].get("inlayTypeHints") is True, eff["features"]
            assert eff["features"].get("inlayParameterHints") is False, eff["features"]
            # Behaviourally the alias enables type hints (``set x 42`` -> a
            # Type-kind hint) while the verbose parameter labels never appear.
            lsp_server.open_ready(uri, "set x 42\nputs hello\n")
            hints = lsp_server.inlay_hints(uri, (0, 0), (2, 0)) or []
            assert any(h.get("kind") == 1 for h in hints), hints  # Type present
            assert all(h.get("kind") != 2 for h in hints), hints  # no Parameter
