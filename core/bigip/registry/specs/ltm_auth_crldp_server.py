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
            "ltm_auth_crldp_server",
            module="ltm",
            object_types=("auth crldp-server",),
        ),
        header_types=(("ltm", "auth crldp-server"),),
        properties=(
            BigipPropertySpec(name="base-dn", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="host",
                value_type="boolean",
                allow_none=True,
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="reverse-dn", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
