"""Parity tripwire: BIG-IP semantic-tokens regex catalogue vs the projection.

The semantic-tokens path in ``lsp/features/_semantic_tokens/_bigip.py``
classifies BIG-IP keywords through ``_BIGIP_KEYWORD_TYPE`` and the
``_BIGIP_TOP_DECL_RE`` keyword set.  Both are hand-maintained and
will drift away from the parser/projection over time — every
release adds new BIG-IP kinds the editor should colour but the
regex catalogue probably won't catch.

This test asserts that the headline kinds (the ones every
production config uses) are covered by the catalogue.  A failure
means a refactor accidentally dropped a keyword; an addition of a
new headline kind to the projection requires also touching the
semantic-tokens catalogue.

The wider catalogue stays regex-driven for now (the deeper
parser-driven semantic-tokens rewrite is a larger follow-up),
but this tripwire keeps the two layers in sync on the cases that
matter most.
"""

from __future__ import annotations


def test_semantic_tokens_keyword_catalogue_covers_headline_projection_kinds():
    """``_BIGIP_KEYWORD_TYPE`` must classify every keyword that names
    a headline-kind objet in the projection.

    "Headline" = the kinds that show up on every BIG-IP config and
    that every editor user expects to see coloured.  The wider
    long-tail of minimal kinds is allowed to stay regex-uncovered
    in v1.
    """
    from lsp.features._semantic_tokens._bigip import (
        _BIGIP_DECL_TYPE,
        _BIGIP_KEYWORD_TYPE,
    )

    expected_keywords = {
        "pool",
        "monitor",
        "profile",
        "vlan",
    }
    missing = expected_keywords - set(_BIGIP_KEYWORD_TYPE)
    assert not missing, (
        f"semantic-tokens keyword catalogue missing headline kinds: {missing}.  "
        "Either add the keyword to ``_BIGIP_KEYWORD_TYPE`` in "
        "``lsp/features/_semantic_tokens/_bigip.py`` or fix the projection's "
        "TMSH spelling to match."
    )

    expected_decls = {"pool", "monitor", "profile", "vlan"}
    missing_decls = expected_decls - set(_BIGIP_DECL_TYPE)
    assert not missing_decls, (
        f"semantic-tokens declaration catalogue missing headline kinds: {missing_decls}."
    )
