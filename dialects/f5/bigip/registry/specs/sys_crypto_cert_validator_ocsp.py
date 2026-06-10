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
            "sys_crypto_cert_validator_ocsp",
            module="sys",
            object_types=("crypto cert-validator ocsp",),
        ),
        header_types=(("sys", "crypto cert-validator ocsp"),),
        properties=(
            BigipPropertySpec(
                name="cache-error-timeout",
                value_type="integer",
                default="3600 seconds",
            ),
            BigipPropertySpec(
                name="cache-timeout",
                value_type="integer",
                default="indefinite, indicating that the response validity period takes precedence",
            ),
            BigipPropertySpec(name="clock-skew", value_type="integer", default="300"),
            BigipPropertySpec(name="concurrent-connections-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dns-resolver",
                value_type="reference",
                references=("net_dns_resolver",),
            ),
            BigipPropertySpec(name="proxy-server-pool", value_type="reference"),
            BigipPropertySpec(name="responder-url", value_type="string"),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="sign-hash",
                value_type="enum",
                enum_values=("sha1", "sha256"),
                default="sha256",
            ),
            BigipPropertySpec(name="signer-cert", value_type="reference"),
            BigipPropertySpec(name="signer-key", value_type="reference"),
            BigipPropertySpec(name="signer-key-passphrase", value_type="string"),
            BigipPropertySpec(name="status-age", value_type="integer", default="86400 seconds"),
            BigipPropertySpec(
                name="strict-resp-cert-check",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="timeout", value_type="integer", default="8"),
            BigipPropertySpec(name="trusted-responders", value_type="reference"),
        ),
    )
