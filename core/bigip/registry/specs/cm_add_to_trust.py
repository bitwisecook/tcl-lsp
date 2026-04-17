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
            "cm_add_to_trust",
            module="cm",
            object_types=("add-to-trust",),
        ),
        header_types=(("cm", "add-to-trust"),),
        properties=(
            BigipPropertySpec(name="device", value_type="string"),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(name="device-name", value_type="string"),
            BigipPropertySpec(name="password", value_type="string"),
            BigipPropertySpec(name="username", value_type="reference", references=("auth_user",)),
        ),
    )
