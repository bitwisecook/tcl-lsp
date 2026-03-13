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
            "ltm_profile_client_ssl",
            module="ltm",
            object_types=("profile client-ssl",),
        ),
        header_types=(("ltm", "profile client-ssl"),),
        properties=(
            BigipPropertySpec(name="alert-timeout", value_type="integer"),
            BigipPropertySpec(
                name="allow-non-ssl", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="allow-dynamic-record-sizing",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="authenticate", value_type="enum", enum_values=("always", "once")
            ),
            BigipPropertySpec(name="authenticate-depth", value_type="integer"),
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
            BigipPropertySpec(
                name="c3d-client-fallback-cert", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="c3d-drop-unknown-ocsp-status",
                value_type="enum",
                enum_values=("drop", "ignore"),
            ),
            BigipPropertySpec(name="c3d-ocsp", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ca-file", value_type="string"),
            BigipPropertySpec(name="cache-size", value_type="integer"),
            BigipPropertySpec(name="cache-timeout", value_type="integer"),
            BigipPropertySpec(name="cert", value_type="string"),
            BigipPropertySpec(name="cert-extension-includes", value_type="string"),
            BigipPropertySpec(
                name="none", value_type="string", in_sections=("cert-extension-includes",)
            ),
            BigipPropertySpec(
                name="key-usage", value_type="string", in_sections=("cert-extension-includes",)
            ),
            BigipPropertySpec(
                name="cert-key-chain",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="cert", value_type="boolean", in_sections=("cert-key-chain",), allow_none=True
            ),
            BigipPropertySpec(
                name="chain", value_type="boolean", in_sections=("cert-key-chain",), allow_none=True
            ),
            BigipPropertySpec(name="key", value_type="string", in_sections=("cert-key-chain",)),
            BigipPropertySpec(
                name="passphrase",
                value_type="boolean",
                in_sections=("cert-key-chain",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="usage",
                value_type="enum",
                in_sections=("cert-key-chain",),
                enum_values=("server", "ca"),
            ),
            BigipPropertySpec(name="cert-lifespan", value_type="integer"),
            BigipPropertySpec(
                name="cert-lookup-by-ipaddr-port",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="chain", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cipher-group", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ciphers", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="client-cert-ca", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="crl-file", value_type="string"),
            BigipPropertySpec(
                name="allow-expired-crl", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_profile_client_ssl",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination-ip-blacklist", value_type="string"),
            BigipPropertySpec(name="destination-ip-whitelist", value_type="string"),
            BigipPropertySpec(
                name="forward-proxy-bypass-default-action",
                value_type="enum",
                enum_values=("intercept", "bypass"),
            ),
            BigipPropertySpec(
                name="generic-alert", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="handshake-timeout", value_type="integer"),
            BigipPropertySpec(name="hostname-blacklist", value_type="string"),
            BigipPropertySpec(name="hostname-whitelist", value_type="string"),
            BigipPropertySpec(name="key", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-ssl-c3d-events", value_type="boolean"),
            BigipPropertySpec(name="log-ssl-client-authentication-events", value_type="boolean"),
            BigipPropertySpec(name="log-ssl-forward-proxy-events", value_type="boolean"),
            BigipPropertySpec(name="log-ssl-handshake-events", value_type="boolean"),
            BigipPropertySpec(name="maximum-record-size", value_type="integer"),
            BigipPropertySpec(
                name="mod-ssl-methods", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="notify-cert-status-to-virtual-server",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="ocsp-stapling", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="options", value_type="string"),
            BigipPropertySpec(name="none", value_type="string", in_sections=("options",)),
            BigipPropertySpec(
                name="no-session-resumption-on-renegotiation",
                value_type="boolean",
                in_sections=("options",),
            ),
            BigipPropertySpec(name="no-tls", value_type="boolean", in_sections=("options",)),
            BigipPropertySpec(
                name="single-dh-use", value_type="list", in_sections=("options",), repeated=True
            ),
            BigipPropertySpec(name="passphrase", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="peer-cert-mode",
                value_type="enum",
                enum_values=("auto", "ignore", "request", "require"),
            ),
            BigipPropertySpec(name="peer-no-renegotiate-timeout", value_type="integer"),
            BigipPropertySpec(
                name="proxy-ssl", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="proxy-ssl-passthrough", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="proxy-ca-cert", value_type="string"),
            BigipPropertySpec(name="proxy-ca-key", value_type="string"),
            BigipPropertySpec(name="proxy-ca-lifespan", value_type="integer"),
            BigipPropertySpec(name="proxy-ca-passphrase", value_type="string"),
            BigipPropertySpec(name="renegotiate-max-record-delay", value_type="integer"),
            BigipPropertySpec(name="renegotiate-period", value_type="integer"),
            BigipPropertySpec(name="renegotiate-size", value_type="integer"),
            BigipPropertySpec(
                name="renegotiation", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="retain-certificate", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(
                name="secure-renegotiation",
                value_type="enum",
                enum_values=("request", "require", "require-strict"),
            ),
            BigipPropertySpec(name="max-renegotiations-per-minute", value_type="integer"),
            BigipPropertySpec(name="max-aggregate-renegotiation-per-minute", value_type="integer"),
            BigipPropertySpec(name="server-name", value_type="string"),
            BigipPropertySpec(
                name="session-mirroring", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="session-ticket", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="session-ticket-timeout", value_type="integer"),
            BigipPropertySpec(name="sni-default", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="sni-require", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="source-ip-blacklist", value_type="string"),
            BigipPropertySpec(name="source-ip-whitelist", value_type="string"),
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
            BigipPropertySpec(name="hello-extension-includes", value_type="string"),
            BigipPropertySpec(
                name="none", value_type="string", in_sections=("hello-extension-includes",)
            ),
            BigipPropertySpec(
                name="strict-resume", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="unclean-shutdown", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ssl-sign-hash",
                value_type="enum",
                enum_values=("any", "sha1", "sha256", "sha384"),
            ),
            BigipPropertySpec(name="max-active-handshakes", value_type="integer"),
            BigipPropertySpec(
                name="data-0rtt",
                value_type="enum",
                enum_values=("disabled", "enabled-with-anti-replay", "enabled-no-anti-replay"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
