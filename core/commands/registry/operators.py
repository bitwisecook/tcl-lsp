"""Hover metadata for expression operators.

Operators live inside ``[expr]`` / ``if`` / ``while`` expressions and
are recognised by the expression lexer.  Each entry maps the operator
keyword to a :class:`HoverSnippet` used by the hover provider.

Two categories:

* **Tcl word operators** — ``eq``, ``ne``, ``in``, ``ni``, ``lt``,
  ``le``, ``gt``, ``ge`` — available in all dialects (Tcl 8.5+).
* **iRules-specific operators** — ``contains``, ``ends_with``, etc. —
  available only when the dialect is ``f5-irules``.
"""

from __future__ import annotations

from .models import HoverSnippet

_TCL_EXPR_SOURCE = "https://www.tcl-lang.org/man/tcl8.6/TclCmd/expr.html"
_IRULES_OPERATORS_SOURCE = "https://clouddocs.f5.com/api/irules/Operators.html"

# Tcl word operators (available in all dialects)

TCL_OPERATOR_HOVER: dict[str, HoverSnippet] = {
    "eq": HoverSnippet(
        summary="String equality — true when both operands are identical strings.",
        synopsis=("<string1> eq <string2>",),
        source=_TCL_EXPR_SOURCE,
    ),
    "ne": HoverSnippet(
        summary="String inequality — true when operands are different strings.",
        synopsis=("<string1> ne <string2>",),
        source=_TCL_EXPR_SOURCE,
    ),
    "in": HoverSnippet(
        summary="List membership — true when *value* is an element of *list*.",
        synopsis=("<value> in <list>",),
        source=_TCL_EXPR_SOURCE,
    ),
    "ni": HoverSnippet(
        summary="List non-membership — true when *value* is not an element of *list*.",
        synopsis=("<value> ni <list>",),
        source=_TCL_EXPR_SOURCE,
    ),
    "lt": HoverSnippet(
        summary="String less-than comparison.",
        synopsis=("<string1> lt <string2>",),
        source=_TCL_EXPR_SOURCE,
    ),
    "le": HoverSnippet(
        summary="String less-than-or-equal comparison.",
        synopsis=("<string1> le <string2>",),
        source=_TCL_EXPR_SOURCE,
    ),
    "gt": HoverSnippet(
        summary="String greater-than comparison.",
        synopsis=("<string1> gt <string2>",),
        source=_TCL_EXPR_SOURCE,
    ),
    "ge": HoverSnippet(
        summary="String greater-than-or-equal comparison.",
        synopsis=("<string1> ge <string2>",),
        source=_TCL_EXPR_SOURCE,
    ),
}

# iRules-specific word operators

IRULES_OPERATOR_HOVER: dict[str, HoverSnippet] = {
    "contains": HoverSnippet(
        summary=(
            "Tests if *string1* contains *string2* as a substring (case-sensitive, no wildcards)."
        ),
        synopsis=("<string1> contains <string2>",),
        source="https://clouddocs.f5.com/api/irules/contains.html",
    ),
    "ends_with": HoverSnippet(
        summary="Tests if *string1* ends with *string2* as a suffix.",
        synopsis=("<string1> ends_with <string2>",),
        source="https://clouddocs.f5.com/api/irules/ends_with.html",
    ),
    "equals": HoverSnippet(
        summary="Tests if *string1* is identical to *string2* (string comparison).",
        synopsis=("<string1> equals <string2>",),
        source="https://clouddocs.f5.com/api/irules/equals.html",
    ),
    "matches_glob": HoverSnippet(
        summary=(
            "Tests if *string1* matches the glob *pattern* (supports `*`, `?`, `[...]` wildcards)."
        ),
        synopsis=("<string1> matches_glob <pattern>",),
        source="https://clouddocs.f5.com/api/irules/matches_glob.html",
    ),
    "matches_regex": HoverSnippet(
        summary="Tests if *string1* matches the regular expression *regex*.",
        synopsis=("<string1> matches_regex <regex>",),
        source="https://clouddocs.f5.com/api/irules/matches_regex.html",
    ),
    "starts_with": HoverSnippet(
        summary="Tests if *string1* starts with *string2* as a prefix.",
        synopsis=("<string1> starts_with <string2>",),
        source="https://clouddocs.f5.com/api/irules/starts_with.html",
    ),
    "and": HoverSnippet(
        summary="Logical AND — true when both operands are true.",
        synopsis=("<expr1> and <expr2>",),
        source="https://clouddocs.f5.com/api/irules/and.html",
    ),
    "or": HoverSnippet(
        summary="Logical OR — true when either operand is true.",
        synopsis=("<expr1> or <expr2>",),
        source="https://clouddocs.f5.com/api/irules/or.html",
    ),
    "not": HoverSnippet(
        summary="Logical NOT — inverts the boolean value of its operand.",
        synopsis=("not <expr>",),
        source="https://clouddocs.f5.com/api/irules/not.html",
    ),
}


def operator_hover(name: str, dialect: str | None) -> HoverSnippet | None:
    """Return the hover snippet for an expression operator, or *None*.

    Checks iRules operators first (when dialect matches), then falls
    back to standard Tcl word operators.
    """
    if dialect == "f5-irules":
        hit = IRULES_OPERATOR_HOVER.get(name)
        if hit is not None:
            return hit
    return TCL_OPERATOR_HOVER.get(name)
