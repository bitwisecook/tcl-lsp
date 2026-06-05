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
            "apm_aaa_saml_idp_connector",
            module="apm",
            object_types=("aaa saml-idp-connector",),
        ),
        header_types=(("apm", "aaa saml-idp-connector"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="artifact-resolution-service-addr",
                value_type="string",
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="artifact-resolution-service-port", value_type="integer"),
            BigipPropertySpec(
                name="artifact-resolution-service-url",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="basic-auth-password", value_type="string", allow_none=True),
            BigipPropertySpec(name="basic-auth-username", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="entity-id", value_type="string"),
            BigipPropertySpec(
                name="identity-location",
                value_type="enum",
                enum_values=("attribute", "subject"),
            ),
            BigipPropertySpec(
                name="identity-location-attribute",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="idp-certificate", value_type="string", allow_none=True),
            BigipPropertySpec(name="import-metadata", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="metadata-cert", value_type="string", allow_none=True),
            BigipPropertySpec(name="name-qualifier", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="serverssl-profile-name",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="sign-artifact-resolution-rq",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(name="single-logout-binding", value_type="unknown"),
            BigipPropertySpec(
                name="single-logout-response-uri",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="single-logout-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="sso-binding",
                value_type="enum",
                enum_values=("http-post", "http-redirect"),
                default="http-post",
            ),
            BigipPropertySpec(name="sso-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="want-authn-request-signed",
                value_type="enum",
                required=True,
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="want-detached-signature",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
        ),
    )
