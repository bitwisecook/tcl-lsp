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
            "ltm_profile_radius",
            module="ltm",
            object_types=("profile radius",),
        ),
        header_types=(("ltm", "profile radius"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="clients",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_profile_radius",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="pem-protocol-profile-radius",
                value_type="reference",
                allow_none=True,
            ),
            BigipPropertySpec(name="persist-avp", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="subscriber-discovery",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
