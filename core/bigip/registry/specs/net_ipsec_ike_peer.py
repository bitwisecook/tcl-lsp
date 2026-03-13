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
            "net_ipsec_ike_peer",
            module="net",
            object_types=("ipsec ike-peer",),
        ),
        header_types=(("net", "ipsec ike-peer"),),
        properties=(
            BigipPropertySpec(name="address-list", value_type="string"),
            BigipPropertySpec(name="ca-cert-file", value_type="string"),
            BigipPropertySpec(name="crl-file", value_type="string"),
            BigipPropertySpec(name="debug-payloads", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dpd-delay", value_type="integer"),
            BigipPropertySpec(
                name="generate-policy", value_type="enum", enum_values=("off", "on", "unique")
            ),
            BigipPropertySpec(name="ip4-dhcp", value_type="string"),
            BigipPropertySpec(name="ip6-dhcp", value_type="string"),
            BigipPropertySpec(name="ip4-dns", value_type="string"),
            BigipPropertySpec(name="ip6-dns", value_type="string"),
            BigipPropertySpec(name="ip-macro", value_type="string"),
            BigipPropertySpec(name="lifetime", value_type="string"),
            BigipPropertySpec(
                name="local-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("main", "aggressive")),
            BigipPropertySpec(name="my-cert-file", value_type="string"),
            BigipPropertySpec(name="my-cert-key-file", value_type="string"),
            BigipPropertySpec(name="my-cert-key-passphrase", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="my-id-type",
                value_type="reference",
                enum_values=("address", "asn1dn", "fqdn", "keyid-tag", "user-fqdn"),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="my-id-value", value_type="string"),
            BigipPropertySpec(
                name="nat-traversal", value_type="enum", enum_values=("on", "off", "force")
            ),
            BigipPropertySpec(name="ocsp-cert-validator", value_type="string"),
            BigipPropertySpec(name="ocsp-lifetime", value_type="string"),
            BigipPropertySpec(name="ocsp-jitter-percent", value_type="string"),
            BigipPropertySpec(name="ocsp-ha-reauth", value_type="string"),
            BigipPropertySpec(
                name="ocsp-reauth-fail-open", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="passive", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="peer-dynamic-ip", value_type="string"),
            BigipPropertySpec(name="peers-cert-file", value_type="string"),
            BigipPropertySpec(
                name="peers-cert-type",
                value_type="enum",
                allow_none=True,
                enum_values=("certfile", "none"),
            ),
            BigipPropertySpec(
                name="peers-id-type",
                value_type="reference",
                enum_values=("address", "asn1dn", "fqdn", "keyid-tag", "user-fqdn"),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="peers-id-value", value_type="string"),
            BigipPropertySpec(
                name="phase1-auth-method",
                value_type="enum",
                enum_values=(
                    "pre-shared-key",
                    "rsa-signature",
                    "dss",
                    "ecdsa-256",
                    "ecdsa-384",
                    "ecdsa-521",
                ),
            ),
            BigipPropertySpec(
                name="phase1-encrypt-algorithm",
                value_type="enum",
                enum_values=(
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "3des",
                    "aes",
                    "blowfish",
                    "camellia",
                    "cast128",
                    "des",
                ),
            ),
            BigipPropertySpec(
                name="phase1-hash-algorithm",
                value_type="enum",
                enum_values=(
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "md5",
                    "sha1",
                    "sha256",
                    "sha384",
                    "sha512",
                ),
            ),
            BigipPropertySpec(
                name="phase1-perfect-forward-secrecy",
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
                    "ecp256",
                    "ecp384",
                    "ecp521",
                ),
            ),
            BigipPropertySpec(name="preshared-key", value_type="string"),
            BigipPropertySpec(name="preshared-key-encrypted", value_type="string"),
            BigipPropertySpec(
                name="prf", value_type="enum", enum_values=("sha1", "sha256", "sha384", "sha512")
            ),
            BigipPropertySpec(
                name="proxy-support",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="remote-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="remote-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="replay-window-size", value_type="integer"),
            BigipPropertySpec(name="state", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="traffic-selector", value_type="string"),
            BigipPropertySpec(name="verify-cert", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(
                name="version",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
        ),
    )
