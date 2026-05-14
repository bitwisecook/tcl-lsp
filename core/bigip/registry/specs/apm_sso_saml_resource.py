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
            "apm_sso_saml_resource",
            module="apm",
            object_types=("sso saml-resource",),
        ),
        header_types=(("apm", "sso saml-resource"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="publish-on-webtop",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="sso-config-saml", value_type="string", allow_none=True),
        ),
    )
