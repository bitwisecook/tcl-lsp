"""BIG-IP config files resolve to the ``f5-bigip`` dialect consistently.

These files are key-value config stanzas, not Tcl source, even though they
embed ``ltm rule`` stanzas (iRules) and BIG-IP encrypted-string markers
(``$M$…$``).  The general Tcl analyser must never run on their top-level
text, so the two dialect resolvers — ``infer_document_dialect`` (sets the
``DocumentState.dialect_hint``) and ``resolve_dialect_for_uri`` (drives the
per-request ``dialect_scope``) — must both classify them as ``f5-bigip``.

The subtle case guarded here: a real ``bigip.conf`` contains ``ltm rule``
stanzas, which the conf-wrapped-iRules heuristic in
``detect_dialect_from_source`` would otherwise report as ``f5-irules``.  The
canonical BIG-IP basename must win over that autodetection so the analysis
skip is not silently defeated.
"""

from __future__ import annotations

import pytest

from compiler.registry.dialect import detect_dialect_from_source
from dialects.f5.bigip.rule_extract import is_conf_wrapped_irules
from server.state import resolve_dialect_for_uri
from server.workspace.document_state import infer_document_dialect
from server.workspace.scanner import _BIGIP_CONF_NAMES, is_bigip_conf_name

# A representative bigip.conf fragment: a pool, plus an embedded iRule whose
# body contains an encrypted-string marker that the Tcl analyser would
# mis-read as a variable reference (W210).
_BIGIP_CONF_WITH_RULE = """ltm pool /Common/p { members { 1.2.3.4:80 { } } }
ltm rule /Common/r {
  when HTTP_REQUEST {
    log local0. "$M$mn$9uYx+bTjLD8YSGZA="
  }
}
"""


@pytest.mark.parametrize("name", sorted(_BIGIP_CONF_NAMES))
def test_is_bigip_conf_name_matches_canonical_basenames(name):
    assert is_bigip_conf_name(f"file:///config/{name}") is True
    # Case-insensitive and path-stripped.
    assert is_bigip_conf_name(f"file:///CONFIG/{name.upper()}") is True
    assert is_bigip_conf_name(name) is True


def test_is_bigip_conf_name_rejects_other_files():
    assert is_bigip_conf_name("file:///x/foo.tcl") is False
    assert is_bigip_conf_name("file:///x/my_bigip.conf") is False
    assert is_bigip_conf_name("file:///x/bigip.conf.bak") is False


def test_embedded_rule_would_autodetect_as_irules():
    # Precondition for the regression below: the bare source heuristic
    # classifies the fragment as f5-irules.
    assert is_conf_wrapped_irules(_BIGIP_CONF_WITH_RULE) is True
    assert detect_dialect_from_source(_BIGIP_CONF_WITH_RULE) == "f5-irules"


def test_both_resolvers_classify_bigip_conf_with_embedded_rule_as_bigip():
    uri = "file:///config/bigip.conf"
    assert infer_document_dialect(uri, _BIGIP_CONF_WITH_RULE) == "f5-bigip"
    dialect, _extras = resolve_dialect_for_uri(uri, _BIGIP_CONF_WITH_RULE)
    assert dialect == "f5-bigip"


def test_resolvers_agree_for_empty_bigip_conf():
    uri = "file:///config/bigip_base.conf"
    assert infer_document_dialect(uri, "") == "f5-bigip"
    assert resolve_dialect_for_uri(uri, "")[0] == "f5-bigip"


def test_explicit_directive_overrides_bigip_basename():
    # The user's `# tcl-dialect:` directive wins over the basename heuristic,
    # consistently across both resolvers.
    src = "# tcl-dialect: tcl8.6\n" + _BIGIP_CONF_WITH_RULE
    uri = "file:///config/bigip.conf"
    assert infer_document_dialect(uri, src) == "tcl8.6"
    assert resolve_dialect_for_uri(uri, src)[0] == "tcl8.6"


def test_normal_tcl_file_is_unaffected():
    # A non-bigip basename containing the same `$M$…` text is still treated
    # as ordinary Tcl (no forced f5-bigip), so normal analysis applies.
    uri = "file:///x/foo.tcl"
    assert infer_document_dialect(uri, 'set x "$M$abc="') is None
    assert resolve_dialect_for_uri(uri, "set x 1")[0] in (None, "tcl8.6")
