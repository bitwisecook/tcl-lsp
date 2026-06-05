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
            "ltm_dns_hpke_profile",
            module="ltm",
            object_types=("dns hpke profile",),
        ),
        header_types=(("ltm", "dns hpke profile"),),
        properties=(
            BigipPropertySpec(name="aead", value_type="string", default="AES-128-GCM"),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="expiration-period",
                value_type="integer",
                default="0 (zero), which indicates unset, and thus the hpke key does not expire",
            ),
            BigipPropertySpec(name="kdf", value_type="string", default="HKDF-SHA256"),
            BigipPropertySpec(name="kem", value_type="string", default="X25519"),
            BigipPropertySpec(
                name="rollover-period",
                value_type="integer",
                default="0 (zero), which indicates unset, and thus the hpke key does not roll over",
            ),
        ),
    )
