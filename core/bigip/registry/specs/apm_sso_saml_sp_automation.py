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
            "apm_sso_saml_sp_automation",
            module="apm",
            object_types=("sso saml-sp-automation",),
        ),
        header_types=(("apm", "sso saml-sp-automation"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="dns-resolver-name", value_type="string"),
            BigipPropertySpec(name="frequency", value_type="integer"),
            BigipPropertySpec(
                name="metadata-urls",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="url-value",
                value_type="string",
                in_sections=("metadata-urls",),
            ),
            BigipPropertySpec(name="serverssl-profile-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="sp-obj-name-tag", value_type="string"),
            BigipPropertySpec(name="sso-config-saml", value_type="string"),
        ),
    )
