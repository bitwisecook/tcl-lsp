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
            "sys_crypto_key",
            module="sys",
            object_types=("crypto key",),
        ),
        header_types=(("sys", "crypto key"),),
        properties=(
            BigipPropertySpec(name="challenge-password", value_type="string"),
            BigipPropertySpec(
                name="admin-email-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="city", value_type="string"),
            BigipPropertySpec(name="common-name", value_type="string"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(
                name="curve-name",
                value_type="enum",
                enum_values=("prime256v1", "secp384r1", "secp521r1"),
            ),
            BigipPropertySpec(
                name="email-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="key-size", value_type="enum", enum_values=("512", "1024", "2048", "4096")
            ),
            BigipPropertySpec(
                name="key-type",
                value_type="enum",
                enum_values=("dsa-private", "ec-private", "rsa-private"),
            ),
            BigipPropertySpec(name="lifetime", value_type="string"),
            BigipPropertySpec(name="organization", value_type="string"),
            BigipPropertySpec(name="ou", value_type="string"),
            BigipPropertySpec(name="passphrase", value_type="string"),
            BigipPropertySpec(
                name="security-type",
                value_type="enum",
                enum_values=("fips", "normal", "password", "nethsm"),
            ),
            BigipPropertySpec(name="state", value_type="string"),
            BigipPropertySpec(name="subject-alternative-name", value_type="string"),
            BigipPropertySpec(
                name="cert-order-manager",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="check-status",
                value_type="enum",
                in_sections=("cert-order-manager",),
                enum_values=("yes", "no"),
            ),
            BigipPropertySpec(
                name="order-id",
                value_type="integer",
                in_sections=("cert-order-manager",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="order-passphrase",
                value_type="boolean",
                in_sections=("cert-order-manager",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="order-type",
                value_type="enum",
                in_sections=("cert-order-manager",),
                enum_values=("cancel", "new", "renew", "revoke"),
            ),
            BigipPropertySpec(
                name="revoke-reason",
                value_type="enum",
                in_sections=("cert-order-manager",),
                enum_values=(
                    "aacompromise",
                    "affiliationchanged",
                    "cessationofoperation",
                    "removefromcrl",
                    "unspecified",
                    "cacompromise",
                    "certificatehold",
                    "keycompromise",
                    "privilegewithdrawn",
                    "superseded",
                ),
            ),
            BigipPropertySpec(name="install", value_type="string"),
            BigipPropertySpec(name="from-local-file", value_type="string"),
            BigipPropertySpec(name="from-url", value_type="string"),
        ),
    )
