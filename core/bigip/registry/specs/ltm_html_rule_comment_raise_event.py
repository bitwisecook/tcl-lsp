from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_html_rule_comment_raise_event",
            module="ltm",
            object_types=("html-rule comment-raise-event",),
        ),
        header_types=(("ltm", "html-rule comment-raise-event"),),
        properties=(),
    )
