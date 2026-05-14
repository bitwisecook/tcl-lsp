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
            BigipPropertySpec(name="proxy-port", value_type="unknown"),
            BigipPropertySpec(name="proxy-server", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="time-out", value_type="unknown", default="8 seconds"),
            BigipPropertySpec(name="trusted-ca-bundle", value_type="unknown"),
            BigipPropertySpec(
                name="update-interval",
                value_type="unknown",
                default="0, which means the generated ca-bundle is not dynamically updated",
            ),
            BigipPropertySpec(
                name="update-now",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="include-url",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
