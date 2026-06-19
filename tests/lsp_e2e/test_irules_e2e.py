"""F5 iRules dialect features, end-to-end against a dedicated server.

These run against ``lsp_server_irules`` rather than the shared ``lsp_server``:
opening an iRules document auto-switches the server's process-global command
pack into the ``f5-irules`` dialect, so dialect-sensitive cases are isolated
on their own server to keep the main Tcl server uncontaminated.

Ported from the iRules cases in ``tests/test_hover.py``.
"""

from __future__ import annotations

from ._lsp_helpers import (
    completion_items,
    completion_labels,
    decode_semantic_tokens,
    hover_text,
)


def _hover(lsp_server, uri, line, char):
    return hover_text(lsp_server.hover(uri, line, char))


def _open(lsp_server_irules, uri_factory, source):
    uri = uri_factory("irule")
    lsp_server_irules.open_ready(uri, source, language_id="tcl-irule")
    return uri


def _labels(lsp_server_irules, uri, line, char):
    return completion_labels(lsp_server_irules.completion(uri, line, char))


class TestIrulesHover:
    def test_irules_subcommand_hover(self, lsp_server_irules, uri_factory):
        uri = uri_factory("irule")
        lsp_server_irules.open_ready(uri, "HTTP::header insert X-Test 1\n", language_id="tcl-irule")
        text = _hover(lsp_server_irules, uri, 0, 15)
        assert "insert" in text.lower()
        assert "header" in text.lower()

    def test_curated_irules_hover_does_not_mark_refinement_status(
        self, lsp_server_irules, uri_factory
    ):
        uri = uri_factory("irule")
        lsp_server_irules.open_ready(
            uri, 'when HTTP_REQUEST { log local0. "ok" }\n', language_id="tcl-irule"
        )
        assert "note:" not in _hover(lsp_server_irules, uri, 0, 2).lower()

    def test_namespace_only_irules_hover_shows_profile_requirement(
        self, lsp_server_irules, uri_factory
    ):
        uri = uri_factory("irule")
        lsp_server_irules.open_ready(uri, 'ACCESS::log 1 "trace"\n', language_id="tcl-irule")
        text = _hover(lsp_server_irules, uri, 0, 5)
        assert "Requires" in text
        assert "ACCESS" in text


class TestIrulesCompletion:
    def test_when_event_name_completion(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "when ")
        labels = _labels(lsp_server_irules, uri, 0, 5)
        assert "HTTP_REQUEST" in labels
        assert "CLIENT_ACCEPTED" in labels

    def test_when_priority_and_timing_keywords_after_event(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "priority" in labels
        assert "timing" in labels

    def test_when_priority_and_timing_partial_keyword(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST pr"
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "priority" in labels
        assert "timing" not in labels

    def test_when_timing_value_keywords_after_timing(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST timing "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "enable" in labels
        assert "disable" in labels

    def test_when_timing_values_not_suggested_after_priority(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST priority "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "enable" not in labels
        assert "disable" not in labels

    def test_http_header_subcommand_keywords(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "HTTP::header ")
        labels = _labels(lsp_server_irules, uri, 0, 13)
        assert {"insert", "replace", "value"} <= set(labels)

    def test_http_header_partial_keyword(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "HTTP::header re")
        labels = _labels(lsp_server_irules, uri, 0, 15)
        assert "remove" in labels
        assert "replace" in labels
        assert "insert" not in labels

    def test_http_respond_options_after_status_code(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "HTTP::respond 302 ")
        labels = _labels(lsp_server_irules, uri, 0, 18)
        assert {"content", "noserver", "version"} <= set(labels)

    def test_irules_event_valid_command_ranked_before_invalid(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "when HTTP_REQUEST {\n    \n}\n")
        by = {i["label"]: i for i in completion_items(lsp_server_irules.completion(uri, 1, 4))}
        assert by["HTTP::header"]["sortText"] < by["TCP::collect"]["sortText"]

    def test_when_priority_and_timing_after_priority_value(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST priority 500 "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "priority" in labels
        assert "timing" in labels

    def test_argument_value_has_documentation(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "when ")
        items = completion_items(lsp_server_irules.completion(uri, 0, 5))
        assert items
        assert any(
            i.get("documentation") is not None for i in items if i["label"] == "HTTP_REQUEST"
        )


def _ca_new_texts(actions):
    out = []
    for action in actions or []:
        edit = action.get("edit") or {}
        for edits in (edit.get("changes") or {}).values():
            out.extend(e["newText"] for e in edits)
        for change in edit.get("documentChanges") or []:
            out.extend(e["newText"] for e in change.get("edits") or [])
    return out


def _ca_titles(actions):
    return [a.get("title", "") for a in actions or []]


def _source_actions(actions):
    return [a for a in actions or [] if (a.get("kind") or "") == "source"]


def _diag(code, message, start, end):
    return {
        "range": {
            "start": {"line": start[0], "character": start[1]},
            "end": {"line": end[0], "character": end[1]},
        },
        "code": code,
        "message": message,
        "source": "tcl-lsp",
    }


class TestIrulesCollectCodeActions:
    def test_irule1005_adds_collect_bootstrap_options(self, lsp_server_irules, uri_factory):
        uri = _open(
            lsp_server_irules,
            uri_factory,
            "when CLIENT_DATA {\n    set payload [TCP::payload]\n}\n",
        )
        diag = _diag(
            "IRULE1005",
            "'CLIENT_DATA' will never fire without a TCP::collect or UDP::collect call in another event.",
            (0, 5),
            (0, 16),
        )
        actions = lsp_server_irules.code_actions(uri, (0, 5), (0, 16), diagnostics=[diag])
        snippets = _ca_new_texts(actions)
        assert any("when CLIENT_ACCEPTED" in s and "TCP::collect" in s for s in snippets)
        assert any("when CLIENT_ACCEPTED" in s and "UDP::collect" in s for s in snippets)

    def test_irule1006_prefers_server_ssl_handshake_bootstrap(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "when SERVERSSL_DATA {\n    SSL::payload\n}\n")
        diag = _diag(
            "IRULE1006",
            "'SSL::payload' without a SSL::collect call. The payload buffer will be empty.",
            (1, 4),
            (1, 16),
        )
        actions = lsp_server_irules.code_actions(uri, (1, 4), (1, 16), diagnostics=[diag])
        snippets = _ca_new_texts(actions)
        assert any("when SERVERSSL_HANDSHAKE" in s and "SSL::collect" in s for s in snippets)


class TestIrulesTaintQuickFixes:
    def _fix(self, lsp_server_irules, uri_factory, source, code, message, start, end):
        uri = _open(lsp_server_irules, uri_factory, source)
        diag = _diag(code, message, start, end)
        return lsp_server_irules.code_actions(
            uri, start, end, diagnostics=[diag], only=["quickfix"]
        )

    def test_irule3001_wrap_html_encode(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "HTTP::respond 200 content $raw\n",
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            (0, 0),
            (0, 29),
        )
        fixes = [a for a in actions if "html_encode" in a.get("title", "")]
        assert len(fixes) == 1
        assert any("[html_encode $raw]" in s for s in _ca_new_texts(fixes))

    def test_irule3002_wrap_uri_encode(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "HTTP::header insert X-Fwd $raw\n",
            "IRULE3002",
            "Tainted variable $raw in HTTP header/cookie value (HTTP::header insert); risk of header injection",
            (0, 0),
            (0, 29),
        )
        fixes = [a for a in actions if "URI::encode" in a.get("title", "")]
        assert len(fixes) == 1
        assert any("[URI::encode $raw]" in s for s in _ca_new_texts(fixes))

    def test_t103_wrap_regex_quote(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "regexp $pat $data\n",
            "T103",
            "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
            (0, 0),
            (0, 16),
        )
        fixes = [a for a in actions if "regex::quote" in a.get("title", "")]
        assert len(fixes) == 1
        assert any("[regex::quote $pat]" in s for s in _ca_new_texts(fixes))

    def test_t102_insert_double_dash(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "regexp $pat $data\n",
            "T102",
            "Tainted variable $pat in option position of 'regexp' without '--' terminator; risk of option injection",
            (0, 0),
            (0, 16),
        )
        fixes = [a for a in actions if "--" in a.get("title", "")]
        assert len(fixes) == 1
        assert any("-- " in s for s in _ca_new_texts(fixes))

    def test_braced_variable_wrapped(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "HTTP::respond 200 content ${raw}\n",
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            (0, 0),
            (0, 30),
        )
        fixes = [a for a in actions if "html_encode" in a.get("title", "")]
        assert any("[html_encode ${raw}]" in s for s in _ca_new_texts(fixes))

    def test_no_fix_for_unknown_code(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "puts $raw\n",
            "T101",
            "Tainted variable $raw flows into puts; output may contain injected content",
            (0, 0),
            (0, 8),
        )
        assert not [
            a
            for a in (actions or [])
            if any(
                k in a.get("title", "")
                for k in ("HTML::encode", "URI::encode", "regex::quote", "--")
            )
        ]

    def test_t106_remove_redundant_encode(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "set double [HTML::encode $safe]\n",
            "T106",
            "Variable $safe is already HTML-escaped; passing through HTML::encode double-encodes the value",
            (0, 0),
            (0, 30),
        )
        fixes = [a for a in actions if "Remove redundant" in a.get("title", "")]
        assert len(fixes) == 1
        assert any(s == "$safe" for s in _ca_new_texts(fixes))

    def test_irule3004_no_autofix(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "HTTP::redirect $url\n",
            "IRULE3004",
            "Tainted variable $url in redirect URL (HTTP::redirect); risk of open redirect",
            (0, 0),
            (0, 18),
        )
        assert not [
            a
            for a in (actions or [])
            if any(k in a.get("title", "") for k in ("html_encode", "URI::encode", "regex::quote"))
        ]


class TestIrulesTaintProcInsertion:
    def _fix(self, lsp_server_irules, uri_factory, source, code, message, start, end):
        uri = _open(lsp_server_irules, uri_factory, source)
        diag = _diag(code, message, start, end)
        return lsp_server_irules.code_actions(
            uri, start, end, diagnostics=[diag], only=["quickfix"]
        )

    def test_t103_inserts_regex_quote_proc(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "regexp $pat $data\n",
            "T103",
            "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
            (0, 0),
            (0, 16),
        )
        fixes = [a for a in actions if "regex::quote" in a.get("title", "")]
        snippets = _ca_new_texts(fixes)
        assert any("[regex::quote $pat]" in s for s in snippets)
        assert any("proc regex::quote" in s for s in snippets)
        assert any("regsub" in s for s in snippets)

    def test_irule3001_inserts_html_encode_proc(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "HTTP::respond 200 content $raw\n",
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            (0, 0),
            (0, 29),
        )
        fixes = [a for a in actions if "html_encode" in a.get("title", "")]
        snippets = _ca_new_texts(fixes)
        assert any("[html_encode $raw]" in s for s in snippets)
        assert any("proc html_encode" in s for s in snippets)
        assert any("string map" in s for s in snippets)

    def test_irule3002_no_proc_insert(self, lsp_server_irules, uri_factory):
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            "HTTP::header insert X-Fwd $raw\n",
            "IRULE3002",
            "Tainted variable $raw in HTTP header/cookie value (HTTP::header insert); risk of header injection",
            (0, 0),
            (0, 29),
        )
        fixes = [a for a in actions if "URI::encode" in a.get("title", "")]
        assert not any("proc " in s for s in _ca_new_texts(fixes))


class TestIrulesProfilesHeader:
    def _src(self, lsp_server_irules, uri_factory, source):
        uri = _open(lsp_server_irules, uri_factory, source)
        return _source_actions(lsp_server_irules.code_actions(uri, (0, 0), (0, 0)))

    def test_http_event_generates_http_profile(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules, uri_factory, "when HTTP_REQUEST {\n    HTTP::respond 200\n}\n"
        )
        assert len(sa) == 1
        assert _ca_new_texts(sa)[0] == "# Profiles: HTTP\n"

    def test_existing_matching_directive_no_action(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            "# Profiles: HTTP\nwhen HTTP_REQUEST {\n    HTTP::respond 200\n}\n",
        )
        assert len(sa) == 0

    def test_dns_event_generates_dns_profile(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            "when DNS_REQUEST {\n    set q [DNS::question name]\n}\n",
        )
        assert len(sa) == 1
        assert _ca_new_texts(sa)[0] == "# Profiles: DNS\n"

    def test_multiple_events_combines_profiles(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            'when HTTP_REQUEST {\n    HTTP::uri\n}\nwhen CLIENTSSL_HANDSHAKE {\n    log local0. "done"\n}\n',
        )
        assert len(sa) == 1
        assert _ca_new_texts(sa)[0] == "# Profiles: CLIENTSSL, HTTP\n"

    def test_existing_outdated_directive_offers_update(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            "# Profiles: HTTP\nwhen HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n",
        )
        assert len(sa) == 1
        assert "Update" in sa[0]["title"]
        assert "CLIENTSSL" in _ca_new_texts(sa)[0]

    def test_rule_init_only_no_action(self, lsp_server_irules, uri_factory):
        sa = self._src(lsp_server_irules, uri_factory, "when RULE_INIT {\n    set ::counter 0\n}\n")
        assert len(sa) == 0

    def test_irule1006_bootstrap_action_is_deduplicated(self, lsp_server_irules, uri_factory):
        uri = _open(
            lsp_server_irules,
            uri_factory,
            "when CLIENT_DATA {\n    set a [TCP::payload]\n    set b [TCP::payload]\n}\n",
        )
        first = _diag("IRULE1006", "'TCP::payload' without a TCP::collect call.", (1, 11), (1, 23))
        second = _diag("IRULE1006", "'TCP::payload' without a TCP::collect call.", (2, 11), (2, 23))
        actions = lsp_server_irules.code_actions(uri, (1, 11), (1, 23), diagnostics=[first, second])
        collect = [a for a in (actions or []) if "TCP::collect" in a.get("title", "")]
        assert len(collect) == 1


class TestIrulesTaintProcAlreadyDefined:
    def _fix(self, lsp_server_irules, uri_factory, source, code, message, start, end):
        uri = _open(lsp_server_irules, uri_factory, source)
        diag = _diag(code, message, start, end)
        return lsp_server_irules.code_actions(
            uri, start, end, diagnostics=[diag], only=["quickfix"]
        )

    def test_t103_no_proc_insert_when_already_defined(self, lsp_server_irules, uri_factory):
        source = (
            "proc regex::quote {str} { regsub -all {[][{}()*+?.\\\\^$|]} $str {\\\\&} }\n"
            "regexp $pat $data\n"
        )
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            source,
            "T103",
            "Tainted variable $pat in regexp pattern position (regexp); risk of regex injection",
            (1, 0),
            (1, 16),
        )
        fixes = [a for a in actions if "regex::quote" in a.get("title", "")]
        snippets = _ca_new_texts(fixes)
        assert any("[regex::quote $pat]" in s for s in snippets)
        assert not any("proc regex::quote" in s for s in snippets)

    def test_irule3001_no_proc_insert_when_already_defined(self, lsp_server_irules, uri_factory):
        source = (
            "proc html_encode {str} { string map {& &amp; < &lt;} $str }\n"
            "HTTP::respond 200 content $raw\n"
        )
        actions = self._fix(
            lsp_server_irules,
            uri_factory,
            source,
            "IRULE3001",
            "Tainted variable $raw in HTTP response body (HTTP::respond); risk of XSS",
            (1, 0),
            (1, 29),
        )
        fixes = [a for a in actions if "html_encode" in a.get("title", "")]
        snippets = _ca_new_texts(fixes)
        assert any("[html_encode $raw]" in s for s in snippets)
        assert not any("proc html_encode" in s for s in snippets)


class TestIrulesProfilesHeaderExtended:
    def _src(self, lsp_server_irules, uri_factory, source):
        uri = _open(lsp_server_irules, uri_factory, source)
        return _source_actions(lsp_server_irules.code_actions(uri, (0, 0), (0, 0)))

    def test_http_event_plus_ssl_command(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            "when HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n",
        )
        assert len(sa) == 1
        text = _ca_new_texts(sa)[0]
        assert "CLIENTSSL" in text
        assert "HTTP" in text

    def test_existing_matching_directive_comma_format(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            "# Profiles: CLIENTSSL, HTTP, SERVERSSL\n"
            "when HTTP_REQUEST {\n    SSL::extensions -type renegotiation\n}\n",
        )
        assert len(sa) == 0

    def test_fasthttp_normalised_to_http(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules, uri_factory, "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}\n"
        )
        assert len(sa) == 1
        text = _ca_new_texts(sa)[0]
        assert "FASTHTTP" not in text
        assert "HTTP" in text

    def test_clientssl_event_generates_clientssl_profile(self, lsp_server_irules, uri_factory):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            'when CLIENTSSL_HANDSHAKE {\n    log local0. "TLS done"\n}\n',
        )
        assert len(sa) == 1
        assert _ca_new_texts(sa)[0] == "# Profiles: CLIENTSSL\n"

    def test_clientssl_clienthello_omits_persist_helper_profile(
        self, lsp_server_irules, uri_factory
    ):
        sa = self._src(
            lsp_server_irules,
            uri_factory,
            'when CLIENTSSL_CLIENTHELLO {\n    log local0. "TLS hello"\n}\n',
        )
        assert len(sa) == 1
        text = _ca_new_texts(sa)[0]
        assert text == "# Profiles: CLIENTSSL\n"
        assert "PERSIST" not in text


def _irules_typed(lsp_server_irules, uri):
    legend = lsp_server_irules.initialize_result["capabilities"]["semanticTokensProvider"][
        "legend"
    ]["tokenTypes"]
    tokens = decode_semantic_tokens(lsp_server_irules.semantic_tokens(uri))
    for tok in tokens:
        tok["type"] = legend[tok["type"]]
    return tokens


class TestIrulesSemanticTokens:
    def test_comment_with_namespace_qualifiers_stays_one_comment(
        self, lsp_server_irules, uri_factory
    ):
        source = "# TCP::collect / TCP::payload / TCP::release\n"
        uri = _open(lsp_server_irules, uri_factory, source)
        tokens = _irules_typed(lsp_server_irules, uri)
        assert len(tokens) == 1
        assert tokens[0]["type"] == "comment"
        assert tokens[0]["length"] == len(source.rstrip("\n"))

    def test_comment_header_block_all_comments(self, lsp_server_irules, uri_factory):
        source = (
            "# Flow:\n"
            "#   1. CLIENT_ACCEPTED / SERVER_CONNECTED -> TCP::collect\n"
            "#   2. CLIENT_DATA    / SERVER_DATA      -> TCP::payload ... TCP::release\n"
        )
        uri = _open(lsp_server_irules, uri_factory, source)
        tokens = _irules_typed(lsp_server_irules, uri)
        assert all(t["type"] == "comment" for t in tokens)


def _irules_codes(diags) -> set[str]:
    return {str(d.get("code")) for d in (diags or [])}


class TestIrulesTaintDiagnostics:
    """Taint diagnostics must actually *fire* on the wire (positive + negative).

    ``TestIrulesTaintQuickFixes`` synthesises a diagnostic to test the quick-fix
    geometry; nothing asserted that the server *publishes* a taint diagnostic at
    all.  These pin the taint analysis end-to-end: a taint source flowing into an
    HTTP sink raises ``IRULE3102``, and the same sink fed a constant stays
    silent — the must-fire / must-stay-silent contract for the taint family.
    """

    def test_taint_source_in_http_sink_fires_irule3102(self, lsp_server_irules, uri_factory):
        uri = uri_factory("irule")
        diags = lsp_server_irules.open_ready(
            uri,
            "when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri]\n}\n",
            language_id="tcl-irule",
        )
        assert "IRULE3102" in _irules_codes(diags), _irules_codes(diags)

    def test_constant_in_http_sink_is_silent(self, lsp_server_irules, uri_factory):
        uri = uri_factory("irule")
        diags = lsp_server_irules.open_ready(
            uri,
            'when HTTP_REQUEST {\n    HTTP::respond 200 content "static body"\n}\n',
            language_id="tcl-irule",
        )
        assert "IRULE3102" not in _irules_codes(diags), _irules_codes(diags)


class TestIrulesByteArrayCorruption:
    """S110 byte-array corruption on the wire (F5 KB K22406348).

    Reading a ``*::payload`` byte array, coercing it to a character string,
    and writing it back with ``<proto>::payload replace`` double-encodes every
    byte >= 0x80.  These pin the must-fire / must-stay-silent contract for the
    byte-array round-trip, end-to-end against the packaged server.
    """

    # A deep-tier iRules code that fires for every ``*::payload`` body here
    # (payload accessed without a matching collect).  Its presence proves the
    # deep diagnostics publish has landed, so S110 (also deep) is decidable.
    _DEEP_MARKER = "IRULE1006"

    @classmethod
    def _deep_diags(cls, server, uri_factory, source: str) -> list[dict]:
        """Open *source* and return the final (basic + deep) publish.

        Shimmer (S110) is a *deep* diagnostic: the server publishes the fast
        tier first, then a second publish with the deep tier.  Poll the latest
        publish until the deep marker lands so we never race the fast tier.
        """
        import time as _time

        uri = uri_factory("irule")
        server.open_ready(uri, source, language_id="tcl-irule")
        deadline = _time.monotonic() + 25.0
        diags: list[dict] = []
        while _time.monotonic() < deadline:
            diags = server.await_diagnostics(uri, version=1)
            if any(str(d.get("code")) == cls._DEEP_MARKER for d in diags):
                return diags
            _time.sleep(0.2)
        raise AssertionError(
            f"deep diagnostics ({cls._DEEP_MARKER}) never published; saw {_irules_codes(diags)}"
        )

    def test_payload_string_roundtrip_fires_s110(self, lsp_server_irules, uri_factory):
        diags = self._deep_diags(
            lsp_server_irules,
            uri_factory,
            "when HTTP_REQUEST_DATA {\n"
            "    set original_data [HTTP::payload]\n"
            '    set new_data "$original_data MODIFIED"\n'
            "    HTTP::payload replace 0 100 $new_data\n"
            "}\n",
        )
        s110 = [d for d in diags if str(d.get("code")) == "S110"]
        assert s110, _irules_codes(diags)
        assert s110[0].get("severity") == 2  # DiagnosticSeverity.Warning

    def test_clean_payload_writeback_silent(self, lsp_server_irules, uri_factory):
        diags = self._deep_diags(
            lsp_server_irules,
            uri_factory,
            "when HTTP_REQUEST_DATA {\n"
            "    set original_data [HTTP::payload]\n"
            "    HTTP::payload replace 0 100 $original_data\n"
            "}\n",
        )
        assert "S110" not in _irules_codes(diags), _irules_codes(diags)

    def test_binary_scan_fix_silent(self, lsp_server_irules, uri_factory):
        diags = self._deep_diags(
            lsp_server_irules,
            uri_factory,
            "when HTTP_REQUEST_DATA {\n"
            "    set original_data [HTTP::payload]\n"
            '    set new_data "$original_data MODIFIED"\n'
            "    binary scan $new_data c* -\n"
            "    HTTP::payload replace 0 100 $new_data\n"
            "}\n",
        )
        assert "S110" not in _irules_codes(diags), _irules_codes(diags)


class TestIrulesWhenBodyAnalysed:
    """Dialect-gated ``when`` body recursion (PR #640), iRules side.

    Under ``f5-irules`` ``when`` IS a recognised builtin whose braced argument
    is a handler *script*, so its body is analysed and a read-before-set inside
    it fires W210.  This is the positive counterpart to
    ``test_when_body_not_analysed_under_plain_tcl`` in ``test_diagnostics_e2e``
    — the same source stays opaque (no W210) under plain Tcl.
    """

    def test_when_body_is_analysed_under_irules(self, lsp_server_irules, uri_factory):
        uri = uri_factory("irule")
        diags = lsp_server_irules.open_ready(
            uri,
            "when HTTP_REQUEST {\n    set y $undefvar\n}\n",
            language_id="tcl-irule",
        )
        # The W210 on the never-set ``undefvar`` read proves the braced body was
        # recursed into as a script (under plain Tcl it would be opaque data).
        assert "W210" in _irules_codes(diags), _irules_codes(diags)
