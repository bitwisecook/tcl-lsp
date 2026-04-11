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
            "sys_crypto_cert_order_manager",
            module="sys",
            object_types=("crypto cert-order-manager",),
        ),
        header_types=(("sys", "crypto cert-order-manager"),),
        properties=(
            BigipPropertySpec(name="additional-headers", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="authority",
                value_type="enum",
                enum_values=("comodo", "digicert", "godaddy", "symantec"),
            ),
            BigipPropertySpec(name="auto-renew", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(
                name="base-url", value_type="enum", allow_none=True, enum_values=("url", "none")
            ),
            BigipPropertySpec(name="ca-cert", value_type="string"),
            BigipPropertySpec(name="client-cert", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="client-key", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="client-key-passphrase", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="internal-proxy", value_type="string"),
            BigipPropertySpec(name="login-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="login-password", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="order-info", value_type="string"),
            BigipPropertySpec(
                name="validity-days",
                value_type="enum",
                allow_none=True,
                enum_values=("days", "none"),
            ),
        ),
    )
