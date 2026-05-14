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
            "sys_httpd",
            module="sys",
            object_types=("httpd",),
        ),
        header_types=(("sys", "httpd"),),
        properties=(
            BigipPropertySpec(
                name="allow",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="hostname", value_type="unknown", in_sections=("allow",)),
            BigipPropertySpec(name="auth-name", value_type="string"),
            BigipPropertySpec(
                name="auth-pam-dashboard-timeout",
                value_type="enum",
                enum_values=("off", "on"),
            ),
            BigipPropertySpec(name="auth-pam-idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="auth-pam-validate-ip",
                value_type="enum",
                enum_values=("off", "on"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="fastcgi-timeout", value_type="integer"),
            BigipPropertySpec(
                name="hostname-lookup",
                value_type="enum",
                enum_values=("double", "off", "on"),
            ),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "error", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="redirect-http-to-https",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="request-body-max-timeout", value_type="integer"),
            BigipPropertySpec(name="request-body-min-rate", value_type="integer"),
            BigipPropertySpec(name="request-body-timeout", value_type="integer"),
            BigipPropertySpec(name="request-header-max-timeout", value_type="integer"),
            BigipPropertySpec(name="request-header-min-rate", value_type="integer"),
            BigipPropertySpec(name="request-header-timeout", value_type="integer"),
            BigipPropertySpec(name="ssl-ca-cert-file", value_type="string"),
            BigipPropertySpec(name="ssl-certchainfile", value_type="string"),
            BigipPropertySpec(name="ssl-certfile", value_type="string"),
            BigipPropertySpec(name="ssl-certkeyfile", value_type="string"),
            BigipPropertySpec(name="ssl-ciphersuite", value_type="string"),
            BigipPropertySpec(name="ssl-include", value_type="string"),
            BigipPropertySpec(name="ssl-ocsp-default-responder", value_type="string"),
            BigipPropertySpec(name="ssl-ocsp-enable", value_type="enum", enum_values=("off", "on")),
            BigipPropertySpec(
                name="ssl-ocsp-override-responder",
                value_type="enum",
                enum_values=("off", "on"),
            ),
            BigipPropertySpec(name="ssl-ocsp-responder-timeout", value_type="integer"),
            BigipPropertySpec(name="ssl-ocsp-response-max-age", value_type="integer"),
            BigipPropertySpec(name="ssl-ocsp-response-time-skew", value_type="integer"),
            BigipPropertySpec(name="ssl-port", value_type="integer"),
            BigipPropertySpec(name="ssl-protocol", value_type="string"),
            BigipPropertySpec(
                name="ssl-verify-client",
                value_type="enum",
                enum_values=("no", "optional", "optional-no-ca", "require"),
            ),
            BigipPropertySpec(name="ssl-verify-depth", value_type="integer"),
        ),
    )
