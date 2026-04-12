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
            BigipPropertySpec(name="cache-error-timeout", value_type="integer"),
            BigipPropertySpec(name="cache-timeout", value_type="integer"),
            BigipPropertySpec(name="concurrent-connections-limit", value_type="integer"),
            BigipPropertySpec(name="clock-skew", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dns-resolver", value_type="string"),
            BigipPropertySpec(
                name="proxy-server-pool", value_type="reference", references=("ltm_pool",)
            ),
            BigipPropertySpec(name="responder-url", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(name="sign-hash", value_type="enum", enum_values=("sha1", "sha256")),
            BigipPropertySpec(name="signer-cert", value_type="string"),
            BigipPropertySpec(name="signer-key", value_type="string"),
            BigipPropertySpec(name="signer-key-passphrase", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="status-age", value_type="integer"),
            BigipPropertySpec(
                name="strict-resp-cert-check",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="trusted-responders", value_type="boolean", allow_none=True),
        ),
    )
