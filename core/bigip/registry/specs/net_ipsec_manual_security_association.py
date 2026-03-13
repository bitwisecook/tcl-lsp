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
            "net_ipsec_manual_security_association",
            module="net",
            object_types=("ipsec manual-security-association",),
        ),
        header_types=(("net", "ipsec manual-security-association"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="auth-algorithm", value_type="string"),
            BigipPropertySpec(name="auth-key", value_type="string"),
            BigipPropertySpec(
                name="destination-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="encrypt-algorithm",
                value_type="enum",
                enum_values=("3des", "aes128", "aes192", "aes256", "null"),
            ),
            BigipPropertySpec(name="encrypt-key", value_type="string"),
            BigipPropertySpec(name="ipsec-policy", value_type="string"),
            BigipPropertySpec(name="protocol", value_type="string"),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="spi", value_type="string"),
        ),
    )
