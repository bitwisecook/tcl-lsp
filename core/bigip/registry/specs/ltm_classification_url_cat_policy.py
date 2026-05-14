from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_classification_url_cat_policy",
            module="ltm",
            object_types=("classification url-cat-policy",),
        ),
        header_types=(("ltm", "classification url-cat-policy"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="feed-lists",
                value_type="list",
                references=("security_ip_intelligence_feed_list",),
                list_operators=frozenset(("add", "delete")),
            ),
        ),
    )
