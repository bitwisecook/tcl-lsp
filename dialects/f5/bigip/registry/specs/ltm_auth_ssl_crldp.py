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
            "ltm_auth_ssl_crldp",
            module="ltm",
            object_types=("auth ssl-crldp",),
        ),
        header_types=(("ltm", "auth ssl-crldp"),),
        properties=(
            BigipPropertySpec(
                name="cache-timeout",
                value_type="integer",
                default="86400 (24 hours)",
            ),
            BigipPropertySpec(name="connection-timeout", value_type="integer", default="15"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="servers",
                value_type="enum",
                required=True,
                allow_none=True,
                enum_values=("default", "none"),
                default="none",
            ),
            BigipPropertySpec(
                name="update-interval",
                value_type="integer",
                default="0 (zero), which indicates an internal default value is active",
            ),
            BigipPropertySpec(
                name="use-issuer",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
