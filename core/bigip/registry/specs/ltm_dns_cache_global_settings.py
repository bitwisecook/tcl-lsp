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
            "ltm_dns_cache_global_settings",
            module="ltm",
            object_types=("dns cache global-settings",),
        ),
        header_types=(("ltm", "dns cache global-settings"),),
        properties=(
            BigipPropertySpec(name="cache-maximum-ttl", value_type="integer"),
            BigipPropertySpec(name="cache-minimum-ttl", value_type="integer"),
            BigipPropertySpec(name="resolver-edns-buffer-size", value_type="integer"),
            BigipPropertySpec(name="serve-expired", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="serve-expired-ttl", value_type="integer"),
            BigipPropertySpec(
                name="serve-expired-ttl-reset", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(name="serve-expired-reply-ttl", value_type="integer"),
            BigipPropertySpec(name="serve-expired-client-timeout", value_type="integer"),
        ),
    )
