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
            BigipPropertySpec(name="additional-headers", value_type="string", allow_none=True),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="authority",
                value_type="enum",
                enum_values=("comodo", "digicert", "godaddy", "symantec"),
            ),
            BigipPropertySpec(name="auto-renew", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="base-url",
                value_type="enum",
                allow_none=True,
                enum_values=("URL", "none"),
            ),
            BigipPropertySpec(name="ca-cert", value_type="unknown"),
            BigipPropertySpec(
                name="client-cert",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="client-key",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="client-key-passphrase", value_type="string", allow_none=True),
            BigipPropertySpec(name="edit-order-info", value_type="unknown"),
            BigipPropertySpec(name="internal-proxy", value_type="unknown"),
            BigipPropertySpec(name="login-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="login-password", value_type="string", allow_none=True),
            BigipPropertySpec(name="order-info", value_type="string"),
            BigipPropertySpec(
                name="validity-days",
                value_type="enum",
                allow_none=True,
                enum_values=("days", "none"),
            ),
        ),
    )
