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
            "apm_policy_agent_aaa_saml",
            module="apm",
            object_types=("policy agent aaa-saml",),
        ),
        header_types=(("apm", "policy agent aaa-saml"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="attr-consuming-service-session-var",
                value_type="unknown",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="attribute-consuming-service",
                value_type="unknown",
                allow_none=True,
            ),
            BigipPropertySpec(name="server", value_type="unknown", allow_none=True),
        ),
    )
