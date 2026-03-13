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
            "ltm_classification_urldb_feed_list",
            module="ltm",
            object_types=("classification urldb-feed-list",),
        ),
        header_types=(("ltm", "classification urldb-feed-list"),),
        properties=(
            BigipPropertySpec(name="default-url-category", value_type="string"),
            BigipPropertySpec(name="url", value_type="string"),
            BigipPropertySpec(name="poll-interval", value_type="integer"),
            BigipPropertySpec(name="user", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="load", value_type="string"),
        ),
    )
