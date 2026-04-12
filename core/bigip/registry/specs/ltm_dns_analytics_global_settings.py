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
            "ltm_dns_analytics_global_settings",
            module="ltm",
            object_types=("dns analytics global-settings",),
        ),
        header_types=(("ltm", "dns analytics global-settings"),),
        properties=(
            BigipPropertySpec(
                name="collect-client-ip", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collect-query-name", value_type="enum", enum_values=("enabled", "disabled")
            ),
        ),
    )
