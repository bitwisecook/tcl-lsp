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
            BigipPropertySpec(name="cache-timeout", value_type="integer"),
            BigipPropertySpec(name="connection-timeout", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="servers", value_type="enum", allow_none=True, enum_values=("default", "none")
            ),
            BigipPropertySpec(name="update-interval", value_type="integer"),
            BigipPropertySpec(
                name="use-issuer", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
