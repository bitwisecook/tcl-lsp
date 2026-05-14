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
            "apm_ntlm_machine_account",
            module="apm",
            object_types=("ntlm machine-account",),
        ),
        header_types=(("apm", "ntlm machine-account"),),
        properties=(
            BigipPropertySpec(
                name="action",
                value_type="enum",
                enum_values=("change-password", "noop"),
            ),
            BigipPropertySpec(name="administrator-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="administrator-password", value_type="string", allow_none=True),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="domain-controller-fqdn", value_type="unknown"),
            BigipPropertySpec(name="domain-fqdn", value_type="unknown"),
            BigipPropertySpec(name="machine-account-name", value_type="string", allow_none=True),
        ),
    )
