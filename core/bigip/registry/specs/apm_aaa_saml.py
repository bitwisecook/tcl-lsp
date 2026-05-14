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
            "apm_aaa_saml",
            module="apm",
            object_types=("aaa saml",),
        ),
        header_types=(("apm", "aaa saml"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="assertion-consumer-binding",
                value_type="enum",
                enum_values=("http-artifact", "http-post"),
            ),
            BigipPropertySpec(
                name="attribute-consuming-services",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="attribute-consuming-service-index",
                value_type="integer",
                in_sections=("attribute-consuming-services",),
            ),
            BigipPropertySpec(name="auth-context-class-list", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="auth-context-comparison-method",
                value_type="enum",
                enum_values=("better", "exact", "maximum", "minimum"),
            ),
            BigipPropertySpec(name="auth-context-methods", value_type="list"),
            BigipPropertySpec(
                name="default-attribute-consuming-service",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="entity-id", value_type="string"),
            BigipPropertySpec(
                name="export-metadata",
                value_type="enum",
                enum_values=("no-signing", "with-signing"),
            ),
            BigipPropertySpec(name="force-authn", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(
                name="idp-connectors",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="idp-matching-source",
                value_type="string",
                in_sections=("idp-connectors",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="idp-matching-value",
                value_type="string",
                in_sections=("idp-connectors",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="is-authn-request-signed",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="metadata-cert", value_type="string", allow_none=True),
            BigipPropertySpec(name="metadata-file", value_type="string", allow_none=True),
            BigipPropertySpec(name="metadata-signkey", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="name-id-policy-allow-create",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="name-id-policy-format", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="name-id-policy-sp-name-qualifier",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="provider-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="relay-state", value_type="string", allow_none=True),
            BigipPropertySpec(name="sp-certificate", value_type="string", allow_none=True),
            BigipPropertySpec(name="sp-host", value_type="string", allow_none=True),
            BigipPropertySpec(name="sp-scheme", value_type="enum", enum_values=("http", "https")),
            BigipPropertySpec(name="sp-signkey", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="want-assertion-encrypted",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="want-assertion-signed",
                value_type="enum",
                enum_values=("false", "true"),
            ),
        ),
    )
