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
            "wam_object_type",
            module="wam",
            object_types=("object-type",),
        ),
        header_types=(("wam", "object-type"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="code", value_type="unknown"),
            BigipPropertySpec(
                name="compression",
                value_type="enum",
                enum_values=("disabled", "policy-controlled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="extensions",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="mime-types",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="symmetric-compression",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
