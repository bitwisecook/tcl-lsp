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
            "net_fdb_tunnel",
            module="net",
            object_types=("fdb tunnel",),
        ),
        header_types=(("net", "fdb tunnel"),),
        properties=(
            BigipPropertySpec(
                name="records",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("records",)),
            BigipPropertySpec(name="endpoint", value_type="string", in_sections=("records",)),
            BigipPropertySpec(
                name="endpoints",
                value_type="enum",
                in_sections=("records",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="replicators",
                value_type="enum",
                in_sections=("records",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
        ),
    )
