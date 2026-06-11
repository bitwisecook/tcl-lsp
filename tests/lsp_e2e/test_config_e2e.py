"""Effective-config + per-feature toggles, end-to-end against the packaged server.

The single biggest hole in the suite was that *no* e2e test drove the
``workspace/configuration`` path: ``getEffectiveConfig`` was only checked for
``"dialect" in data``, so a server that silently ignores every feature toggle
passed.  The VS Code suite ("Configuration Settings → disabling features.X")
covers this and is what exposed that the native server's ``getEffectiveConfig``
returns only ``{uri, dialect}`` and honours no toggles.

These tests mirror that contract over raw JSON-RPC so it is enforced against
*both* backends:

  * ``getEffectiveConfig`` exposes a resolved ``features`` map (plus dialect,
    line length, optimiser switch);
  * with a provider's feature disabled, that provider returns empty/None;
  * the optimiser master switch round-trips through both ``getEffectiveConfig``
    and observable behaviour;
  * formatting honours the request's indent width;
  * a toggle flipped off and back on leaves no sticky state.

Config is injected through the harness ``config_session`` helper, which updates
the ``tclLsp`` ``workspace/configuration`` reply, notifies
``didChangeConfiguration``, settles on ``getEffectiveConfig`` (no sleeps), and
restores the prior config on exit so the shared server is never left mutated.
"""

from __future__ import annotations

import pytest

# Feature toggles whose handler must return empty/None when disabled.  Keyed by
# the camelCase ``tclLsp.features.*`` name (the key ``getEffectiveConfig``
# reports and ``apply_configuration`` injects).
_TOGGLEABLE_FEATURES = [
    "hover",
    "completion",
    "documentSymbols",
    "definition",
    "references",
    "signatureHelp",
    "folding",
    "selectionRange",
    "documentLinks",
]


def _is_empty(result) -> bool:
    """LSP "no result": ``None``, an empty list, or an empty completion list."""
    if result is None:
        return True
    if isinstance(result, list):
        return len(result) == 0
    if isinstance(result, dict):
        # CompletionList / SelectionRange-style payloads.
        if "items" in result:
            return len(result["items"]) == 0
        return len(result) == 0
    return False


class TestEffectiveConfigShape:
    def test_reports_resolved_feature_map(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\n")
        cfg = lsp_server.effective_config(uri)
        assert isinstance(cfg, dict)
        # The resolved feature map must be present and cover the toggles the
        # editor namespace advertises — a server returning only {uri, dialect}
        # fails here.
        features = cfg.get("features")
        assert isinstance(features, dict), cfg
        missing = [k for k in _TOGGLEABLE_FEATURES if k not in features]
        assert not missing, f"effective config missing feature keys {missing}: {features}"
        assert all(isinstance(v, bool) for v in features.values()), features

    def test_reports_dialect_and_scalars(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\n")
        cfg = lsp_server.effective_config(uri)
        assert isinstance(cfg.get("dialect"), str) and cfg["dialect"], cfg
        assert isinstance(cfg.get("line_length"), int), cfg
        assert isinstance(cfg.get("optimiser_enabled"), bool), cfg


# --------------------------------------------------------------------------- #
# Per-provider disable contract — one case per toggleable feature.
# --------------------------------------------------------------------------- #


def _probe(lsp_server, uri: str, feature: str):
    """Open a document that yields a non-empty result for *feature* and query it.

    Returns ``(result, query)`` where ``query`` re-runs the same request, so a
    test can compare the enabled baseline against the disabled re-query without
    reopening the document (handlers read the feature toggle at request time).
    """
    if feature == "hover":
        lsp_server.open_ready(uri, "puts hello\n")
        return lambda: lsp_server.hover(uri, 0, 2)
    if feature == "completion":
        lsp_server.open_ready(uri, "pu")
        return lambda: lsp_server.completion(uri, 0, 2)
    if feature == "documentSymbols":
        lsp_server.open_ready(uri, "proc greet {} { return }\n")
        return lambda: lsp_server.document_symbols(uri)
    if feature == "definition":
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        return lambda: lsp_server.definition(uri, 1, 2)
    if feature == "references":
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        return lambda: lsp_server.references(uri, 0, 6)
    if feature == "signatureHelp":
        lsp_server.open_ready(uri, "proc greet {name greeting} { return }\ngreet World\n")
        return lambda: lsp_server.signature_help(uri, 1, 12)
    if feature == "folding":
        lsp_server.open_ready(uri, "proc greet {} {\n    set x 1\n    return $x\n}\n")
        return lambda: lsp_server.folding_range(uri)
    if feature == "selectionRange":
        lsp_server.open_ready(uri, "proc greet {} {\n    set x 1\n    return $x\n}\n")
        return lambda: lsp_server.selection_range(uri, [(2, 11)])
    if feature == "documentLinks":
        lsp_server.open_ready(uri, "source other.tcl\nputs done\n")
        return lambda: lsp_server.document_links(uri)
    raise AssertionError(f"no probe for feature {feature!r}")


@pytest.mark.parametrize("feature", _TOGGLEABLE_FEATURES)
def test_disabling_feature_suppresses_its_provider(feature, lsp_server, uri_factory):
    uri = uri_factory()
    query = _probe(lsp_server, uri, feature)

    # Baseline: the provider produces a non-empty result with defaults on, so
    # the disabled assertion below is meaningful (not vacuously empty).
    baseline = query()
    assert not _is_empty(baseline), f"{feature}: expected a non-empty baseline, got {baseline!r}"

    # Disable just this feature; settle on the effective-config reflecting it.
    with lsp_server.config_session(
        {"features": {feature: False}},
        settle=lambda c: c.get("features", {}).get(feature) is False,
        settle_uri=uri,
    ):
        disabled = query()
        assert _is_empty(disabled), (
            f"{feature}: provider must return empty/None when disabled, got {disabled!r}"
        )

    # config_session restored the prior config — the provider works again.
    assert not _is_empty(query()), f"{feature}: provider did not recover after re-enable"


# --------------------------------------------------------------------------- #
# Optimiser master switch — round-trips through getEffectiveConfig + behaviour.
# --------------------------------------------------------------------------- #


class TestOptimiserToggle:
    def test_optimiser_disable_round_trips(self, lsp_server, uri_factory):
        uri = uri_factory()
        # ``llength`` of a literal list folds to a constant — an O1xx optimiser
        # offer when the optimiser is on.
        lsp_server.open_ready(uri, "puts [llength [list a b c]]\n")

        on = lsp_server.effective_config(uri)
        assert on.get("optimiser_enabled") is True, on
        offers_on = lsp_server.execute_command("tcl-lsp.optimiseDocument", [uri, "full"])
        assert offers_on and "puts 3" in offers_on.get("source", ""), offers_on

        with lsp_server.config_session(
            {"optimiser": {"enabled": False}},
            settle=lambda c: c.get("optimiser_enabled") is False,
            settle_uri=uri,
        ):
            off = lsp_server.effective_config(uri)
            assert off.get("optimiser_enabled") is False, off

        # Restored.
        assert lsp_server.effective_config(uri).get("optimiser_enabled") is True


# --------------------------------------------------------------------------- #
# Formatting honours the request's indent width.
# --------------------------------------------------------------------------- #


class TestFormattingIndentRoundTrip:
    """The formatter must honour the LSP request's ``tabSize``.

    ``getEffectiveConfig`` does not surface formatter settings, and an explicit
    ``FormattingOptions.tabSize`` from the client overrides the server's
    ``formatting.indentSize`` config by LSP contract — so the observable
    round-trip is request-options → output indent width.
    """

    SRC = "proc f {} {\nputs hi\n}\n"

    @staticmethod
    def _body_indent(formatted: str) -> str:
        for line in formatted.splitlines():
            if line.lstrip().startswith("puts"):
                return line[: len(line) - len(line.lstrip())]
        raise AssertionError(f"no `puts` body line in {formatted!r}")

    def test_two_space_indent(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, self.SRC)
        edits = lsp_server.formatting(uri, tab_size=2, insert_spaces=True) or []
        assert edits, "expected formatting edits"
        assert self._body_indent(edits[0]["newText"]) == "  "

    def test_four_space_indent(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, self.SRC)
        edits = lsp_server.formatting(uri, tab_size=4, insert_spaces=True) or []
        assert edits, "expected formatting edits"
        assert self._body_indent(edits[0]["newText"]) == "    "


# --------------------------------------------------------------------------- #
# No sticky state across a disable/enable cycle on the shared server.
# --------------------------------------------------------------------------- #


class TestToggleNoStickyState:
    def test_repeated_cycles_keep_provider_working(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")

        for _ in range(2):
            assert lsp_server.hover(uri, 0, 2) is not None
            with lsp_server.config_session(
                {"features": {"hover": False}},
                settle=lambda c: c.get("features", {}).get("hover") is False,
                settle_uri=uri,
            ):
                assert _is_empty(lsp_server.hover(uri, 0, 2))
            # Restored each iteration; never stuck off.
            assert lsp_server.hover(uri, 0, 2) is not None
