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
            "ltm_profile_connector",
            module="ltm",
            object_types=("profile connector",),
        ),
        header_types=(("ltm", "profile connector"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="connect-on-data",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="connection-timeout", value_type="integer"),
            BigipPropertySpec(name="entry-virtual-server", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="service-down-action",
                value_type="enum",
                enum_values=("drop", "ignore", "reset"),
            ),
        ),
    )
