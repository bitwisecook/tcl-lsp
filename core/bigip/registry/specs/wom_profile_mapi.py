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
            "wom_profile_mapi",
            module="wom",
            object_types=("profile mapi",),
        ),
        header_types=(("wom", "profile mapi"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("wom_profile_mapi",),
                default="mapi",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="discover-exchange-servers",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="native-compression",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
