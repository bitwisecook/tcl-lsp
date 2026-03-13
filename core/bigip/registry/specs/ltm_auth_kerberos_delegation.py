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
            "ltm_auth_kerberos_delegation",
            module="ltm",
            object_types=("auth kerberos-delegation",),
        ),
        header_types=(("ltm", "auth kerberos-delegation"),),
        properties=(
            BigipPropertySpec(name="client-principal", value_type="string"),
            BigipPropertySpec(
                name="debug-logging", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="protocol-transition", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="server-principal", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
