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
            "sys_crypto_client",
            module="sys",
            object_types=("crypto client",),
        ),
        header_types=(("sys", "crypto client"),),
        properties=(
            BigipPropertySpec(name="addr", value_type="string"),
            BigipPropertySpec(name="heartbeat", value_type="integer"),
            BigipPropertySpec(name="max-retries", value_type="integer"),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="profiles",
                value_type="enum",
                repeated=True,
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="req-timeout", value_type="integer"),
            BigipPropertySpec(name="retry-interval", value_type="integer"),
        ),
    )
