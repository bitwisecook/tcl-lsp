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
            "apm_sso_saml",
            module="apm",
            object_types=("sso saml",),
        ),
        header_types=(("apm", "sso saml"),),
        properties=(
            BigipPropertySpec(name="apm-log-config", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="artifact-resolution-service-name",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="assertion-validity", value_type="integer"),
            BigipPropertySpec(
                name="attributes",
                value_type="list",
                allow_none=True,
                usage_flags=frozenset(("deprecated", "optional")),
            ),
            BigipPropertySpec(
                name="auth-context-method",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="encrypt",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="encrypt-subject",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="encryption-type",
                value_type="enum",
                enum_values=("aes128", "aes192", "aes256"),
            ),
            BigipPropertySpec(
                name="encryption-type-subject",
                value_type="enum",
                enum_values=("aes128", "aes192", "aes256"),
                default="aes128",
            ),
            BigipPropertySpec(name="entity-id", value_type="string", required=True),
            BigipPropertySpec(
                name="export-metadata",
                value_type="enum",
                enum_values=("no-signing", "with-signing"),
            ),
            BigipPropertySpec(
                name="idp-certificate",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="idp-certificate-session-var",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="idp-host",
                value_type="enum",
                required=True,
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="idp-scheme",
                value_type="enum",
                enum_values=("http", "https"),
                default="https",
            ),
            BigipPropertySpec(
                name="idp-signkey",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="idp-signkey-session-var",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="key-transport-algorithm",
                value_type="enum",
                enum_values=("rsa-oaep", "rsa-v1.5"),
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="metadata-cert", value_type="string", allow_none=True),
            BigipPropertySpec(name="metadata-file", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="metadata-signkey",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="multi-values", value_type="list"),
            BigipPropertySpec(name="name-qualifier", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="saml-profiles",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="web-browser-sso",
            ),
            BigipPropertySpec(
                name="sp-connectors",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="subject-type",
                value_type="enum",
                enum_values=(
                    "entity",
                    "kerberos",
                    "persistent",
                    "transient",
                    "unspecified",
                    "x509-subject",
                ),
            ),
            BigipPropertySpec(
                name="subject-value",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
        ),
    )
