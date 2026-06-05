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
            "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
            module="apm",
            object_types=("policy agent endpoint-windows-browser-cache-cleaner",),
        ),
        header_types=(("apm", "policy agent endpoint-windows-browser-cache-cleaner"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="cache-clean-type",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "all-except-css-js", "all-except-img-css-js", "none"),
                default="all",
            ),
            BigipPropertySpec(
                name="clean-passwords",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="empty-recycle-bin",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="idle-timeout",
                value_type="enum",
                required=True,
                enum_values=("immediate", "indefinite"),
                default="0, which enforces no timeout",
            ),
            BigipPropertySpec(name="idle-timeout-screen-lock", value_type="unknown"),
            BigipPropertySpec(
                name="monitor-webtop",
                value_type="enum",
                enum_values=("disable", "enable"),
                default="false",
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(
                name="remove-connection-entry",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
        ),
    )
