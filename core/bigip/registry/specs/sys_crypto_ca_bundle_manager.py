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
            "sys_crypto_ca_bundle_manager",
            module="sys",
            object_types=("crypto ca-bundle-manager",),
        ),
        header_types=(("sys", "crypto ca-bundle-manager"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="proxy-server", value_type="string"),
            BigipPropertySpec(
                name="proxy-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="trusted-ca-bundle", value_type="string"),
            BigipPropertySpec(name="update-interval", value_type="string"),
            BigipPropertySpec(name="time-out", value_type="string"),
            BigipPropertySpec(name="update-now", value_type="enum", enum_values=("yes", "no")),
        ),
    )
