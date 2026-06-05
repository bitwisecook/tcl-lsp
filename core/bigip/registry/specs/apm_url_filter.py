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
            "apm_url_filter",
            module="apm",
            object_types=("url-filter",),
        ),
        header_types=(("apm", "url-filter"),),
        properties=(
            BigipPropertySpec(
                name="allowed-categories",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="blocked-categories",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
        ),
    )
