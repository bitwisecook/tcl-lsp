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
            "apm_ntlm_ntlm_auth",
            module="apm",
            object_types=("ntlm ntlm-auth",),
        ),
        header_types=(("apm", "ntlm ntlm-auth"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="dc-fqdn-list",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="machine-account-name", value_type="string", allow_none=True),
        ),
    )
