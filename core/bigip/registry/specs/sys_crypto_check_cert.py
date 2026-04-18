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
            "sys_crypto_check_cert",
            module="sys",
            object_types=("crypto check-cert",),
        ),
        header_types=(("sys", "crypto check-cert"),),
        properties=(
            BigipPropertySpec(
                name="ignore-large-cert-bundles",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="log", value_type="enum", enum_values=("enabled", "disabled")),
            BigipPropertySpec(
                name="stdout", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="verbose", value_type="enum", enum_values=("enabled", "disabled")
            ),
        ),
    )
