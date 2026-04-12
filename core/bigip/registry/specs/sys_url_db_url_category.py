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
            "sys_url_db_url_category",
            module="sys",
            object_types=("url-db url-category",),
        ),
        header_types=(("sys", "url-db url-category"),),
        properties=(
            BigipPropertySpec(name="display-name", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="initial-disposition", value_type="integer"),
            BigipPropertySpec(name="is-security-category", value_type="string"),
            BigipPropertySpec(name="parent-cat-number", value_type="integer"),
            BigipPropertySpec(name="severity-level", value_type="integer"),
            BigipPropertySpec(
                name="urls",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
        ),
    )
