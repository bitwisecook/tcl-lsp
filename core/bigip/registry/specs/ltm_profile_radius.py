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
            BigipPropertySpec(
                name="clients",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_profile_radius",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="persist-avp", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="pem-protocol-profile-radius", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="subscriber-discovery", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
