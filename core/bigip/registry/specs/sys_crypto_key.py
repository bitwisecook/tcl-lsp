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
            BigipPropertySpec(name="admin-email-address", value_type="string"),
            BigipPropertySpec(
                name="cert-order-manager",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="check-status",
                value_type="enum",
                in_sections=("cert-order-manager",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="order-id",
                value_type="enum",
                in_sections=("cert-order-manager",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="order-passphrase",
                value_type="enum",
                in_sections=("cert-order-manager",),
                allow_none=True,
                enum_values=("none",),
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
                    "AACompromise",
                    "CACompromise",
                    "affiliationChanged",
                    "certificateHold",
                    "cessationOfOperation",
                    "keyCompromise",
                    "privilegeWithdrawn",
                    "removeFromCRL",
                    "superseded",
                    "unspecified",
                ),
            ),
            BigipPropertySpec(name="challenge-password", value_type="string"),
            BigipPropertySpec(name="city", value_type="string"),
            BigipPropertySpec(name="common-name", value_type="string"),
            BigipPropertySpec(name="consumer", value_type="unknown"),
            BigipPropertySpec(name="country", value_type="string"),
            BigipPropertySpec(
                name="curve-name",
                value_type="enum",
                enum_values=("prime256v1", "secp384r1", "secp521r1"),
            ),
            BigipPropertySpec(name="email-address", value_type="string"),
            BigipPropertySpec(
                name="key-size",
                value_type="enum",
                enum_values=("1024", "2048", "4096", "512"),
            ),
            BigipPropertySpec(
                name="key-type",
                value_type="enum",
                enum_values=("dsa-private", "ec-private", "rsa-private"),
            ),
            BigipPropertySpec(name="lifetime", value_type="unknown"),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="organization", value_type="string"),
            BigipPropertySpec(name="ou", value_type="string"),
            BigipPropertySpec(name="passphrase", value_type="unknown"),
            BigipPropertySpec(name="prompt-for-password", value_type="unknown"),
            BigipPropertySpec(
                name="security-type",
                value_type="enum",
                enum_values=("fips", "nethsm", "normal", "password"),
            ),
            BigipPropertySpec(name="state", value_type="string"),
            BigipPropertySpec(name="subject-alternative-name", value_type="string"),
        ),
    )
