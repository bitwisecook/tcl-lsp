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
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="hostname",
                value_type="list",
                in_sections=("allow",),
                repeated=True,
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(name="auth-name", value_type="string"),
            BigipPropertySpec(
                name="auth-pam-dashboard-timeout", value_type="enum", enum_values=("off", "on")
            ),
            BigipPropertySpec(name="auth-pam-idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="auth-pam-validate-ip", value_type="enum", enum_values=("off", "on")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="fastcgi-timeout", value_type="integer"),
            BigipPropertySpec(name="hostname-lookup", value_type="string"),
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
            BigipPropertySpec(name="request-header-max-timeout", value_type="integer"),
            BigipPropertySpec(name="request-header-min-rate", value_type="integer"),
            BigipPropertySpec(name="request-header-timeout", value_type="integer"),
            BigipPropertySpec(name="request-body-max-timeout", value_type="integer"),
            BigipPropertySpec(name="request-body-min-rate", value_type="integer"),
            BigipPropertySpec(name="request-body-timeout", value_type="integer"),
            BigipPropertySpec(name="ssl-ca-cert-file", value_type="string"),
            BigipPropertySpec(name="ssl-certchainfile", value_type="string"),
            BigipPropertySpec(name="ssl-certfile", value_type="string"),
            BigipPropertySpec(name="ssl-certkeyfile", value_type="string"),
            BigipPropertySpec(name="ssl-ciphersuite", value_type="string"),
            BigipPropertySpec(name="ssl-include", value_type="string"),
            BigipPropertySpec(name="ssl-protocol", value_type="string"),
            BigipPropertySpec(name="ssl-port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="ssl-verify-client",
                value_type="enum",
                enum_values=("no", "require", "optional", "optional-no-ca"),
            ),
            BigipPropertySpec(name="ssl-verify-depth", value_type="integer"),
            BigipPropertySpec(name="ssl-ocsp-enable", value_type="enum", enum_values=("on", "off")),
            BigipPropertySpec(name="ssl-ocsp-default-responder", value_type="string"),
            BigipPropertySpec(
                name="ssl-ocsp-override-responder", value_type="enum", enum_values=("on", "off")
            ),
            BigipPropertySpec(name="ssl-ocsp-responder-timeout", value_type="integer"),
            BigipPropertySpec(name="ssl-ocsp-response-max-age", value_type="integer"),
            BigipPropertySpec(name="ssl-ocsp-response-time-skew", value_type="integer"),
        ),
    )
