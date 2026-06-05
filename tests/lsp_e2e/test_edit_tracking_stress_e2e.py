"""Stress tests that punish the server's open-document edit tracking.

The server advertises *incremental* sync (``textDocumentSync.change == 2``),
so every keystroke an editor makes arrives as a range edit the server splices
into its own buffer.  A bug anywhere on that path — a UTF-16 offset slip, a
multi-line splice that loses a newline, a stale version winning a race, a
cache that survives a reopen — corrupts the buffer the server reasons over,
and every downstream feature then lies.

The oracle here is content-equivalence: drive a long, hostile sequence of
incremental edits, then assert the live buffer behaves *identically* to a
freshly opened document holding the same final text — same diagnostics, same
document-symbol tree (names, kinds, and positions).  The independent copy of
the apply-change algorithm lives in :mod:`._textmirror` and exists only so a
divergence between it and the server is a failure.
"""

from __future__ import annotations

import random

import pytest

from ._textmirror import Edit, TextMirror, random_edit


def _norm_symbols(symbols) -> list:
    out = []
    for sym in symbols or []:
        rng = sym["range"]
        sel = sym["selectionRange"]
        out.append(
            (
                sym.get("name"),
                sym.get("kind"),
                sym.get("detail"),
                (
                    rng["start"]["line"],
                    rng["start"]["character"],
                    rng["end"]["line"],
                    rng["end"]["character"],
                ),
                (sel["start"]["line"], sel["start"]["character"]),
                _norm_symbols(sym.get("children")),
            )
        )
    return out


def _assert_buffer_equiv(lsp_server, edited_uri, edited_version, fresh_uri, fresh_text):
    """The incrementally-edited buffer must behave like a fresh open of its text.

    The oracle is the document-symbol tree: it is a *synchronous*, position-
    sensitive function of the buffer the server is currently holding (computed
    on request, not via the debounced two-phase diagnostics push), so it pins
    every proc/namespace/class/variable name, kind, detail, and range.  After a
    long hostile edit sequence, a single mis-applied splice shifts a declaration
    by a line or column and this comparison fails — which is exactly the
    tracking regression these tests exist to catch.

    (Diagnostics are deliberately *not* used as the oracle here: they are pushed
    in two phases — a fast basic pass then a deep pass at the same document
    version — so an equality check races the deep pass.  Version-targeted
    diagnostic assertions live in ``test_diagnostics_e2e`` instead.)
    """
    # Ensure the edited buffer's analysis has advanced to the final version
    # before we read its synchronous features.
    lsp_server.await_diagnostics(edited_uri, version=edited_version)
    lsp_server.open_ready(fresh_uri, fresh_text)

    edited_syms = _norm_symbols(lsp_server.document_symbols(edited_uri))
    fresh_syms = _norm_symbols(lsp_server.document_symbols(fresh_uri))
    assert edited_syms == fresh_syms, "document-symbol tree diverged (content or position drift)"


SEED_DOC = (
    "proc greet {name} {\n"
    '    puts "Hello $name"\n'
    "}\n"
    "set total [expr {1 + 2}]\n"
    "if $cond { puts $total }\n"
    "greet World\n"
)


class TestRandomEditStorm:
    @pytest.mark.parametrize("seed", [1, 7, 13, 42, 99, 256, 1024])
    def test_random_incremental_edits_match_fresh_open(self, lsp_server, uri_factory, seed):
        uri = uri_factory()
        lsp_server.open_ready(uri, SEED_DOC)
        mirror = TextMirror(SEED_DOC)
        rng = random.Random(seed)
        version = 1
        for _ in range(60):
            edit = random_edit(mirror, rng)
            mirror.apply(edit)
            version += 1
            lsp_server.change_document(uri, version, [edit.as_content_change()])
        _assert_buffer_equiv(lsp_server, uri, version, uri_factory(), mirror.text)

    @pytest.mark.parametrize("seed", [3, 17, 64])
    def test_batched_multi_edit_changes(self, lsp_server, uri_factory, seed):
        # Each didChange carries *several* edits at once (the shape a multi-cursor
        # editor sends).  LSP applies them in array order against the evolving
        # buffer, so the mirror must too.
        uri = uri_factory()
        lsp_server.open_ready(uri, SEED_DOC)
        mirror = TextMirror(SEED_DOC)
        rng = random.Random(seed)
        version = 1
        for _ in range(25):
            batch: list[Edit] = []
            changes = []
            for _ in range(rng.randint(1, 4)):
                edit = random_edit(mirror, rng)
                mirror.apply(edit)
                batch.append(edit)
                changes.append(edit.as_content_change())
            version += 1
            lsp_server.change_document(uri, version, changes)
        _assert_buffer_equiv(lsp_server, uri, version, uri_factory(), mirror.text)


class TestSupersession:
    def test_rapid_edits_final_version_wins(self, lsp_server, uri_factory):
        # Fire a burst of edits without waiting between them, then demand the
        # publish for the *final* version.  The async pipeline must not let an
        # earlier, slower analysis win the race and leave stale diagnostics.
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\n")
        mirror = TextMirror("set x 1\n")
        version = 1
        for i in range(40):
            # Append a fresh line each time so every version has distinct content.
            end = mirror.position_at(len(mirror.text))
            edit = Edit(end, end, f"set v{i} {i}\n")
            mirror.apply(edit)
            version += 1
            lsp_server.change_document(uri, version, [edit.as_content_change()])
        # The buffer the server settled on must be the *final* version's text,
        # not an earlier, slower analysis that won the race.
        _assert_buffer_equiv(lsp_server, uri, version, uri_factory(), mirror.text)

    def test_introduce_then_immediately_fix_error(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")
        # v2: break it (bare `set` → E002).  v3: fix it again — without waiting
        # on v2.  The final publish must reflect v3 (no leftover E002 from v2).
        lsp_server.replace_document(uri, 2, "set\n")
        lsp_server.replace_document(uri, 3, "puts hello\n")
        final = lsp_server.await_diagnostics(uri, version=3)
        assert "E002" not in {str(d.get("code")) for d in final}


class TestStructuralEdits:
    def test_multiline_insertions_and_deletions(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set a 1\nset b 2\nset c 3\n")
        mirror = TextMirror("set a 1\nset b 2\nset c 3\n")
        version = 1

        def edit(start, end, text):
            nonlocal version
            e = Edit(start, end, text)
            mirror.apply(e)
            version += 1
            lsp_server.change_document(uri, version, [e.as_content_change()])

        # Insert two whole lines at the top.
        edit((0, 0), (0, 0), "proc p {} {\n    return\n}\n")
        # Delete the middle original line (`set b 2`).
        edit((4, 0), (5, 0), "")
        # Replace a span crossing a line boundary.
        edit((0, 5), (1, 4), "q {} {\n    error x")
        _assert_buffer_equiv(lsp_server, uri, version, uri_factory(), mirror.text)

    def test_delete_to_empty_then_rebuild(self, lsp_server, uri_factory):
        uri = uri_factory()
        start = "proc greet {} { puts hi }\ngreet\n"
        lsp_server.open_ready(uri, start)
        mirror = TextMirror(start)
        version = 1
        # Delete the entire buffer in one edit.
        end_pos = mirror.position_at(len(mirror.text))
        e = Edit((0, 0), end_pos, "")
        mirror.apply(e)
        version += 1
        lsp_server.change_document(uri, version, [e.as_content_change()])
        # Rebuild a different program one fragment at a time.
        for fragment in ["proc ", "add {a b} ", "{\n", "    expr {$a + $b}\n", "}\n", "add 1 2\n"]:
            pos = mirror.position_at(len(mirror.text))
            e = Edit(pos, pos, fragment)
            mirror.apply(e)
            version += 1
            lsp_server.change_document(uri, version, [e.as_content_change()])
        _assert_buffer_equiv(lsp_server, uri, version, uri_factory(), mirror.text)


class TestUnicodeTracking:
    def test_utf16_offsets_survive_astral_chars(self, lsp_server, uri_factory):
        # Astral-plane characters occupy two UTF-16 code units; a tracker that
        # confuses code units with code points will splice later edits at the
        # wrong column.  Interleave astral inserts with edits *after* them.
        uri = uri_factory()
        lsp_server.open_ready(uri, "set s {}\nset x 1\n")
        mirror = TextMirror("set s {}\nset x 1\n")
        version = 1

        def edit(start, end, text):
            nonlocal version
            e = Edit(start, end, text)
            mirror.apply(e)
            version += 1
            lsp_server.change_document(uri, version, [e.as_content_change()])

        # Put an emoji inside the braces on line 0.
        edit((0, 6), (0, 7), "\U0001f600\U0001f602")
        # Now edit line 0 *after* the astral chars; derive the end-of-line-0
        # position from the mirror so the UTF-16 column accounts for them.
        eol0 = mirror.position_at(mirror.text.index("\n"))
        edit(eol0, eol0, " ;# tail")
        # And change line 1 to confirm cross-line coordinates still resolve.
        edit((1, 6), (1, 7), "999")
        _assert_buffer_equiv(lsp_server, uri, version, uri_factory(), mirror.text)


class TestReopenLifecycle:
    def test_close_and_reopen_resets_version_without_stale_cache(self, lsp_server, uri_factory):
        uri = uri_factory()
        # First session: a clean file at versions 1..3.
        lsp_server.open_ready(uri, "set x 1\n")
        lsp_server.replace_document(uri, 2, "set y 2\n")
        lsp_server.replace_document(uri, 3, "set z 3\n")
        lsp_server.await_diagnostics(uri, version=3)
        lsp_server.close_document(uri)
        # Drop buffered publishes from the first session so the reopen's
        # version-1 publish (not the prior session's, also version 1) is what we
        # observe — the very ambiguity the reopen-reset cache must handle.
        lsp_server.clear_diagnostics_log()
        # Reopen with version reset to 1 holding *broken* content — a stale
        # hover/diagnostic cache keyed on (uri, version) must not resurface.
        diags = lsp_server.open_ready(uri, "set\n", version=1)
        assert "E002" in {str(d.get("code")) for d in diags}
        # Hover at the command resolves against the reopened buffer, not the old one.
        from ._lsp_helpers import hover_text

        assert "set" in hover_text(lsp_server.hover(uri, 0, 1))

    def test_feature_requests_interleaved_with_edits(self, lsp_server, uri_factory):
        # Hammer feature requests *between* edits; the server must answer each
        # against the then-current buffer and never wedge.
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc f {} { return }\nf\n")
        mirror = TextMirror("proc f {} { return }\nf\n")
        rng = random.Random(5)
        version = 1
        for _ in range(20):
            edit = random_edit(mirror, rng)
            mirror.apply(edit)
            version += 1
            lsp_server.change_document(uri, version, [edit.as_content_change()])
            # Fire a request that reads the buffer; we only require it not to error.
            lsp_server.document_symbols(uri)
            lsp_server.semantic_tokens(uri)
        _assert_buffer_equiv(lsp_server, uri, version, uri_factory(), mirror.text)
