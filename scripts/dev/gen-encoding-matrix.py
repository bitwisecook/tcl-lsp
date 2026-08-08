#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Regenerate the encoding-hazard probe matrix (issue #1326).

Nineteen cases, each wrapping a byte-identical iRule body so results are
directly comparable, plus an ASCII control.  The cases cover four families:

* **raw byte-level malformation** — truncated multi-byte sequences, overlong
  encodings, CESU-8 lone surrogates, out-of-range lead bytes, stray
  continuation bytes;
* **encoding confusion** — UTF-16 LE/BE with and without a BOM, UTF-32LE,
  latin-1 high bytes, and a UTF-8 file with embedded NULs;
* **semantically hostile but structurally valid UTF-8** — bidi
  overrides/embeddings/isolates (Trojan Source), zero-width joiners,
  homoglyphs;
* **non-findings, kept as regression guards** — a leading UTF-8 BOM and Tcl
  ``\\u`` escapes, both of which are *correct* to leave alone (see the issue's
  "Method" section).

The files are written as **raw bytes** — several of them are deliberately not
valid UTF-8 and cannot be produced by a text-mode writer.  They are committed
to the repository so the behaviour stays testable without regenerating.

Usage::

    python3 scripts/dev/gen-encoding-matrix.py [OUTDIR]

``OUTDIR`` defaults to ``rust/tcl-lsp-core/tests/fixtures/encoding``.
"""

from __future__ import annotations

import pathlib
import sys

# The shared body every case wraps, so a firing count is comparable across
# cases.  Kept deliberately boring: no diagnostics of its own.
BODY_ASCII = b'when HTTP_REQUEST {\n    if { [HTTP::uri] eq "/a" } {\n        log local0. "hit"\n    }\n}\n'


def _with(marker: bytes) -> bytes:
    """The shared body with `marker` spliced into the logged string."""
    return BODY_ASCII.replace(b'"hit"', b'"hi' + marker + b't"')


CASES: dict[str, bytes] = {
    # -- control -------------------------------------------------------
    "control_ascii.tcl": BODY_ASCII,
    # -- ill-formed UTF-8 ----------------------------------------------
    # A 3-byte lead byte with only one continuation byte present.
    "bad_truncated_3byte.tcl": _with(b"\xe2\x82"),
    # A 2-byte lead byte at end of line with no continuation byte at all.
    "bad_truncated_2byte.tcl": _with(b"\xc3"),
    # Overlong encoding of U+002F SOLIDUS — the classic path-traversal
    # smuggling shape.
    "bad_overlong.tcl": _with(b"\xc0\xaf"),
    # CESU-8 / WTF-8 lone high surrogate U+D800.
    "bad_lone_surrogate.tcl": _with(b"\xed\xa0\x80"),
    # F5..FF are not valid UTF-8 lead bytes at all (post-RFC 3629).
    "bad_out_of_range_lead.tcl": _with(b"\xf5\x80\x80\x80"),
    # A continuation byte with no lead byte in front of it.
    "bad_stray_continuation.tcl": _with(b"\x80"),
    # Latin-1 text mislabelled as UTF-8 ("café" in ISO-8859-1).
    "bad_latin1.tcl": _with(b"\xe9"),
    # -- encoding confusion --------------------------------------------
    "enc_utf16le_bom.tcl": "﻿".encode("utf-16-le") + BODY_ASCII.decode().encode("utf-16-le"),
    "enc_utf16be_bom.tcl": "﻿".encode("utf-16-be") + BODY_ASCII.decode().encode("utf-16-be"),
    "enc_utf16le_nobom.tcl": BODY_ASCII.decode().encode("utf-16-le"),
    "enc_utf32le_bom.tcl": "﻿".encode("utf-32-le") + BODY_ASCII.decode().encode("utf-32-le"),
    # Valid UTF-8 that merely *contains* a couple of NULs — the false-positive
    # guard for the UTF-16 heuristic.  Must NOT be called mis-encoded.
    "enc_utf8_with_nuls.tcl": _with(b"\x00\x00"),
    # -- hostile but well-formed UTF-8 ---------------------------------
    # U+202E RIGHT-TO-LEFT OVERRIDE + U+202C POP DIRECTIONAL FORMATTING.
    "bidi_override.tcl": _with("‮drowssap‬".encode()),
    # U+2066 LEFT-TO-RIGHT ISOLATE + U+2069 POP DIRECTIONAL ISOLATE.
    "bidi_isolate.tcl": _with("⁦x⁩".encode()),
    # U+202A/U+202B embedding pair.
    "bidi_embedding.tcl": _with("‪x‫".encode()),
    # Ordinary right-to-left *content* with no override anywhere — the
    # false-positive guard.  Arabic prose is not a Trojan Source attack.
    "rtl_content_clean.tcl": _with("مرحبا".encode()),
    # -- documented non-findings (regression guards) -------------------
    # A leading UTF-8 BOM is ordinary data (tcl-lexer `LeadingBom`).
    "nonfinding_utf8_bom.tcl": b"\xef\xbb\xbf" + BODY_ASCII,
    # A `\uD800` Tcl escape — the *source file* is pure ASCII.
    "nonfinding_u_escape.tcl": _with(b"\\uD800"),
}


def main() -> int:
    out = pathlib.Path(
        sys.argv[1] if len(sys.argv) > 1 else "rust/tcl-lsp-core/tests/fixtures/encoding"
    )
    out.mkdir(parents=True, exist_ok=True)
    for name, data in sorted(CASES.items()):
        (out / name).write_bytes(data)
        print(f"{name:32s} {len(data):5d} bytes")
    print(f"\n{len(CASES)} cases written to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
