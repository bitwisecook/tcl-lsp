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
                default="All",
                block=(
                    BigipPropertySpec(
                        name="hostname",
                        value_type="unknown",
                        in_sections=("allow",),
                        repeated=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="hostname",
                value_type="unknown",
                in_sections=("allow",),
                repeated=True,
            ),
            BigipPropertySpec(name="auth-name", value_type="string", default="BIG-IP"),
            BigipPropertySpec(
                name="auth-pam-dashboard-timeout",
                value_type="enum",
                enum_values=("off", "on"),
                default="off",
            ),
            BigipPropertySpec(
                name="auth-pam-idle-timeout",
                value_type="integer",
                default="1200 seconds",
            ),
            BigipPropertySpec(
                name="auth-pam-validate-ip",
                value_type="enum",
                enum_values=("off", "on"),
                default="on",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="fastcgi-timeout", value_type="integer"),
            BigipPropertySpec(
                name="hostname-lookup",
                value_type="enum",
                enum_values=("double", "off", "on"),
                default="off",
            ),
            BigipPropertySpec(name="include", value_type="string", default="none"),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "error", "info", "notice", "warn"),
                default="warn",
            ),
            BigipPropertySpec(
                name="redirect-http-to-https",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="request-body-max-timeout", value_type="integer", default="0"),
            BigipPropertySpec(name="request-body-min-rate", value_type="integer", default="500"),
            BigipPropertySpec(name="request-body-timeout", value_type="integer", default="60"),
            BigipPropertySpec(
                name="request-header-max-timeout",
                value_type="integer",
                default="40",
            ),
            BigipPropertySpec(name="request-header-min-rate", value_type="integer", default="500"),
            BigipPropertySpec(name="request-header-timeout", value_type="integer", default="20"),
            BigipPropertySpec(name="ssl-ca-cert-file", value_type="string", default="none"),
            BigipPropertySpec(name="ssl-certchainfile", value_type="string", default="none"),
            BigipPropertySpec(
                name="ssl-certfile",
                value_type="string",
                default="/etc/httpd/conf/ssl",
            ),
            BigipPropertySpec(
                name="ssl-certkeyfile",
                value_type="string",
                default="/etc/httpd/conf/ssl",
            ),
            BigipPropertySpec(
                name="ssl-ciphersuite",
                value_type="string",
                default='"ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-SHA:ECDHE-RSA-AES256-SHA:ECDHE-RSA-AES128-SHA256:ECDHE-RSA-AES256-SHA384:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES128-SHA:ECDHE-ECDSA-AES256-SHA:ECDHE-ECDSA-AES128-SHA256:ECDHE-ECDSA-AES256-SHA384:AES128-GCM-SHA256:AES256-GCM-SHA384:AES128-SHA:AES256-SHA:AES128-SHA256:AES256-SHA256"',
            ),
            BigipPropertySpec(name="ssl-include", value_type="string", default="none"),
            BigipPropertySpec(
                name="ssl-ocsp-default-responder",
                value_type="string",
                default="http://localhost",
            ),
            BigipPropertySpec(
                name="ssl-ocsp-enable",
                value_type="enum",
                enum_values=("off", "on"),
                default="off",
            ),
            BigipPropertySpec(
                name="ssl-ocsp-override-responder",
                value_type="enum",
                enum_values=("off", "on"),
                default="off",
            ),
            BigipPropertySpec(
                name="ssl-ocsp-responder-timeout",
                value_type="integer",
                default="300 seconds",
            ),
            BigipPropertySpec(name="ssl-ocsp-response-max-age", value_type="integer"),
            BigipPropertySpec(
                name="ssl-ocsp-response-time-skew",
                value_type="integer",
                default="300 seconds",
            ),
            BigipPropertySpec(name="ssl-port", value_type="integer", default="443"),
            BigipPropertySpec(
                name="ssl-protocol",
                value_type="string",
                default="all -SSLv2 -SSLv3 -TLSv1",
            ),
            BigipPropertySpec(
                name="ssl-verify-client",
                value_type="enum",
                enum_values=("no", "optional", "optional-no-ca", "require"),
                default="no",
            ),
            BigipPropertySpec(name="ssl-verify-depth", value_type="integer", default="10"),
        ),
    )
