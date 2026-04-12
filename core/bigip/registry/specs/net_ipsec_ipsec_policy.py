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
            "net_ipsec_ipsec_policy",
            module="net",
            object_types=("ipsec ipsec-policy",),
        ),
        header_types=(("net", "ipsec ipsec-policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ike-phase2-auth-algorithm",
                value_type="enum",
                enum_values=(
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "sha1",
                    "sha256",
                    "sha384",
                    "sha512",
                ),
            ),
            BigipPropertySpec(
                name="ike-phase2-encrypt-algorithm",
                value_type="enum",
                enum_values=(
                    "3des",
                    "aes128",
                    "aes192",
                    "aes256",
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "null",
                ),
            ),
            BigipPropertySpec(name="ike-phase2-lifetime", value_type="integer"),
            BigipPropertySpec(name="ike-phase2-lifetime-kilobytes", value_type="integer"),
            BigipPropertySpec(
                name="ike-phase2-perfect-forward-secrecy",
                value_type="enum",
                enum_values=(
                    "modp1024",
                    "modp1536",
                    "modp2048",
                    "modp3072",
                    "modp4096",
                    "modp6144",
                    "modp768",
                    "modp8192",
                ),
            ),
            BigipPropertySpec(
                name="ipcomp",
                value_type="enum",
                allow_none=True,
                enum_values=("deflate", "none", "null"),
            ),
            BigipPropertySpec(
                name="mode", value_type="enum", enum_values=("transport", "tunnel", "interface")
            ),
            BigipPropertySpec(name="protocol", value_type="string"),
            BigipPropertySpec(
                name="tunnel-local-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="tunnel-remote-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
        ),
    )
