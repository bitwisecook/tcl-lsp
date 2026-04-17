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
            "ltm_profile_tcp_analytics",
            module="ltm",
            object_types=("profile tcp-analytics",),
        ),
        header_types=(("ltm", "profile tcp-analytics"),),
        properties=(
            BigipPropertySpec(
                name="collected-by-client-side",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collected-by-server-side",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collected-stats-external-logging",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collected-stats-internal-logging",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collect-city", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="collect-continent", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="collect-country", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="collect-nexthop", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="collect-post-code", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="collect-region", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="collect-remote-host-ip",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collect-remote-host-subnet",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_tcp_analytics",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="external-logging-publisher", value_type="string"),
        ),
    )
