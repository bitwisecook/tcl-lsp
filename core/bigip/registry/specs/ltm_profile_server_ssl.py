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
            "ltm_profile_server_ssl",
            module="ltm",
            object_types=("profile server-ssl",),
        ),
        header_types=(("ltm", "profile server-ssl"),),
        properties=(
            BigipPropertySpec(name="alert-timeout", value_type="integer"),
            BigipPropertySpec(
                name="allow-expired-crl", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="authenticate", value_type="enum", enum_values=("always", "once")
            ),
            BigipPropertySpec(name="authenticate-depth", value_type="integer"),
            BigipPropertySpec(name="authenticate-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="bypass-on-client-cert-fail",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bypass-on-handshake-alert",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="c3d-ca-cert", value_type="string"),
            BigipPropertySpec(name="c3d-ca-key", value_type="string"),
            BigipPropertySpec(name="c3d-ca-passphrase", value_type="string"),
            BigipPropertySpec(
                name="c3d-cert-extension-custom-oids", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="c3d-cert-extension-includes", value_type="string"),
            BigipPropertySpec(
                name="none", value_type="string", in_sections=("c3d-cert-extension-includes",)
            ),
            BigipPropertySpec(
                name="key-usage", value_type="string", in_sections=("c3d-cert-extension-includes",)
            ),
            BigipPropertySpec(name="c3d-cert-lifespan", value_type="integer"),
            BigipPropertySpec(name="ca-file", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cache-size", value_type="integer"),
            BigipPropertySpec(name="cache-timeout", value_type="integer"),
            BigipPropertySpec(name="cert", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="chain", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cipher-group", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ciphers", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="crl", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="crl-file", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_server_ssl",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="expire-cert-response-control",
                value_type="enum",
                enum_values=("drop", "ignore", "mask"),
            ),
            BigipPropertySpec(name="handshake-timeout", value_type="integer"),
            BigipPropertySpec(name="key", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-ssl-c3d-events", value_type="boolean"),
            BigipPropertySpec(name="log-ssl-client-authentication-events", value_type="boolean"),
            BigipPropertySpec(name="log-ssl-forward-proxy-events", value_type="boolean"),
            BigipPropertySpec(name="log-ssl-handshake-events", value_type="boolean"),
            BigipPropertySpec(name="max-active-handshakes", value_type="integer"),
            BigipPropertySpec(
                name="mod-ssl-methods", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="ocsp", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="options", value_type="string"),
            BigipPropertySpec(name="none", value_type="string", in_sections=("options",)),
            BigipPropertySpec(name="no-ssl", value_type="boolean", in_sections=("options",)),
            BigipPropertySpec(name="single-dh-use", value_type="string", in_sections=("options",)),
            BigipPropertySpec(name="passphrase", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="peer-cert-mode", value_type="enum", enum_values=("ignore", "require")
            ),
            BigipPropertySpec(
                name="proxy-ssl", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="proxy-ssl-passthrough", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="renegotiate-period", value_type="integer"),
            BigipPropertySpec(name="renegotiate-size", value_type="integer"),
            BigipPropertySpec(
                name="renegotiation", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="retain-certificate", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(
                name="revoked-cert-status-response-control",
                value_type="enum",
                enum_values=("drop", "ignore", "mask"),
            ),
            BigipPropertySpec(
                name="secure-renegotiation",
                value_type="enum",
                enum_values=("request", "require", "require-strict"),
            ),
            BigipPropertySpec(name="server-name", value_type="string"),
            BigipPropertySpec(
                name="session-mirroring", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="session-ticket", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="generic-alert", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="sni-default", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="sni-require", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(
                name="ssl-c3d", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ssl-forward-proxy", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ssl-forward-proxy-bypass",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="ssl-forward-proxy-verified-handshake",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="ssl-sign-hash",
                value_type="enum",
                enum_values=("any", "sha1", "sha256", "sha384"),
            ),
            BigipPropertySpec(
                name="strict-resume", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="unclean-shutdown", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="data-0rtt", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="unknown-cert-status-response-control",
                value_type="enum",
                enum_values=("ignore", "drop", "mask"),
            ),
            BigipPropertySpec(
                name="untrusted-cert-response-control",
                value_type="enum",
                enum_values=("drop", "ignore", "mask"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
