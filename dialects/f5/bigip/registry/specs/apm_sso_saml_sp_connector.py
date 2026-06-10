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
            "apm_sso_saml_sp_connector",
            module="apm",
            object_types=("sso saml-sp-connector",),
        ),
        header_types=(("apm", "sso saml-sp-connector"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="assertion-consumer-services", value_type="list"),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="encryption-type",
                value_type="enum",
                enum_values=("aes128", "aes192", "aes256"),
                default="aes128",
            ),
            BigipPropertySpec(name="entity-id", value_type="string"),
            BigipPropertySpec(
                name="import-metadata",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="index", value_type="unknown"),
            BigipPropertySpec(
                name="is-authn-request-signed",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="is-default",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="metadata-cert", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="multi-domain-location",
                value_type="string",
                required=True,
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="relay-state", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="signature-type",
                value_type="enum",
                enum_values=("rsa-sha1", "rsa-sha256", "rsa-sha384", "rsa-sha512"),
                default="rsa-sha1",
            ),
            BigipPropertySpec(name="single-logout-binding", value_type="unknown"),
            BigipPropertySpec(name="single-logout-response-uri", value_type="string"),
            BigipPropertySpec(name="single-logout-uri", value_type="string"),
            BigipPropertySpec(name="sp-certificate", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="sp-location",
                value_type="enum",
                enum_values=("external", "internal", "internal-multi-domain"),
            ),
            BigipPropertySpec(
                name="sp-name-qualifier",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="uri", value_type="string"),
            BigipPropertySpec(
                name="want-assertion-encrypted",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="want-assertion-signed",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="want-response-signed",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
        ),
    )
