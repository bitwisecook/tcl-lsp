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
            "sys_state_mirroring",
            module="sys",
            object_types=("state-mirroring",),
        ),
        header_types=(("sys", "state-mirroring"),),
        properties=(
            BigipPropertySpec(name="addr", value_type="string"),
            BigipPropertySpec(name="peer-addr", value_type="string"),
            BigipPropertySpec(name="secondary-addr", value_type="string"),
            BigipPropertySpec(name="secondary-peer-addr", value_type="string"),
            BigipPropertySpec(name="state", value_type="enum", enum_values=("enabled", "disabled")),
        ),
    )
