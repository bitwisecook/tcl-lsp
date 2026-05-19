"""Tests for :mod:`dialects.f5.bigip.tmsh_parse` — tmsh→SCF normalisation."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from dialects.f5.bigip.parser import parse_bigip_conf
from dialects.f5.bigip.tmsh_parse import looks_like_tmsh_output, normalise_tmsh_to_scf


def test_looks_like_tmsh_output_true_for_create():
    text = "tmsh create ltm pool /Common/p1 { }\n"
    assert looks_like_tmsh_output(text) is True


def test_looks_like_tmsh_output_true_for_modify():
    text = "tmsh modify ltm virtual /Common/vs { pool /Common/p1 }\n"
    assert looks_like_tmsh_output(text) is True


def test_looks_like_tmsh_output_skips_leading_blanks_and_comments():
    text = "\n# header comment\n\ntmsh create ltm node /Common/n1 { address 10.0.0.1 }\n"
    assert looks_like_tmsh_output(text) is True


def test_looks_like_tmsh_output_false_for_plain_scf():
    text = "ltm pool /Common/p1 { }\n"
    assert looks_like_tmsh_output(text) is False


def test_normalise_strips_create_prefix():
    text = "tmsh create ltm pool /Common/p1 { load-balancing-mode round-robin }\n"
    assert (
        normalise_tmsh_to_scf(text) == "ltm pool /Common/p1 { load-balancing-mode round-robin }\n"
    )


def test_normalise_strips_modify_prefix():
    text = "tmsh modify ltm virtual /Common/vs_app { pool /Common/p2 }\n"
    assert normalise_tmsh_to_scf(text) == "ltm virtual /Common/vs_app { pool /Common/p2 }\n"


def test_normalise_leaves_embedded_tmsh_prefix_alone():
    text = textwrap.dedent(
        """\
        tmsh create ltm rule /Common/r1 {
            when HTTP_REQUEST {
                log local0. "tmsh create ltm node /Common/fake"
            }
        }
        """
    )
    out = normalise_tmsh_to_scf(text)
    # Top-level prefix stripped …
    assert out.startswith("ltm rule /Common/r1 {")
    # … but the string literal inside the iRule body survives.
    assert 'log local0. "tmsh create ltm node /Common/fake"' in out


def test_normalise_handles_multiline_stanzas():
    text = textwrap.dedent(
        """\
        tmsh create ltm pool /Common/p1 {
            members {
                /Common/n1:80 { address 10.0.0.1 }
            }
            load-balancing-mode round-robin
        }
        tmsh modify ltm virtual /Common/vs_app {
            destination /Common/10.0.0.5:80
            pool /Common/p1
        }
        """
    )
    cfg = parse_bigip_conf(normalise_tmsh_to_scf(text))
    assert "/Common/p1" in cfg.pools
    assert "/Common/vs_app" in cfg.virtual_servers
    assert cfg.virtual_servers["/Common/vs_app"].pool == "/Common/p1"


def test_normalise_preserves_pure_scf_unchanged():
    scf = "ltm pool /Common/p1 { load-balancing-mode round-robin }\n"
    assert normalise_tmsh_to_scf(scf) == scf


def test_normalise_ignores_braces_inside_comments_when_tracking_depth():
    # Regression: a `# }` (or `# {`) inside an iRule body must not affect
    # the brace-depth tracker.  Otherwise a later top-level `tmsh create`
    # line slips through unstripped and parse_bigip_conf misclassifies
    # the stanza as a generic object.
    text = textwrap.dedent(
        """\
        tmsh create ltm rule /Common/r1 {
            when HTTP_REQUEST {
                # closing brace coming: }
                # and an open one too: {
                log local0. "hi"
            }
        }
        tmsh create ltm pool /Common/p1 { load-balancing-mode round-robin }
        """
    )
    cfg = parse_bigip_conf(normalise_tmsh_to_scf(text))
    assert "/Common/r1" in cfg.rules
    assert "/Common/p1" in cfg.pools


def test_normalise_handles_braces_inside_quoted_strings():
    # Quoted-string contents are already ignored by the depth tracker;
    # this guards against a regression of that behaviour.
    text = textwrap.dedent(
        """\
        tmsh create ltm rule /Common/r2 {
            when HTTP_REQUEST {
                log local0. "stray brace: }{"
            }
        }
        tmsh create ltm pool /Common/p2 { }
        """
    )
    cfg = parse_bigip_conf(normalise_tmsh_to_scf(text))
    assert "/Common/r2" in cfg.rules
    assert "/Common/p2" in cfg.pools
