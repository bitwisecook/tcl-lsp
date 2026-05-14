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
            "apm_aaa_saml_idp_automation",
            module="apm",
            object_types=("aaa saml-idp-automation",),
        ),
        header_types=(("apm", "aaa saml-idp-automation"),),
        properties=(
            BigipPropertySpec(name="aaa-saml-server", value_type="string"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="connection-properties",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("connection-properties",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="dns-resolver-name",
                value_type="string",
                in_sections=("connection-properties",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="serverssl-profile-name",
                value_type="string",
                in_sections=("connection-properties",),
                allow_none=True,
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="frequency", value_type="integer"),
            BigipPropertySpec(name="idp-matching-source", value_type="string"),
            BigipPropertySpec(name="idp-obj-name-tag", value_type="string"),
            BigipPropertySpec(name="metadata-matching-tag", value_type="string"),
            BigipPropertySpec(name="metadata-urls", value_type="list"),
        ),
    )
