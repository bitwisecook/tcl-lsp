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
            "net_fdb_vlan",
            module="net",
            object_types=("fdb vlan",),
        ),
        header_types=(("net", "fdb vlan"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="trunk", value_type="string"),
            BigipPropertySpec(name="interface", value_type="string"),
            BigipPropertySpec(name="records", value_type="boolean", allow_none=True),
        ),
    )
