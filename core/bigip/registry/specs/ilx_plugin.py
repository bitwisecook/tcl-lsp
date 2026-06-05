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
            "ilx_plugin",
            module="ilx",
            object_types=("plugin",),
        ),
        header_types=(("ilx", "plugin"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference", default="none"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disabled", value_type="unknown"),
            BigipPropertySpec(name="enabled", value_type="unknown"),
            BigipPropertySpec(
                name="extensions",
                value_type="list",
                repeated=True,
                block=(
                    BigipPropertySpec(
                        name="command-arguments",
                        value_type="unknown",
                        in_sections=("extensions",),
                        usage_flags=frozenset(("optional",)),
                    ),
                    BigipPropertySpec(
                        name="command-options",
                        value_type="unknown",
                        in_sections=("extensions",),
                    ),
                    BigipPropertySpec(
                        name="concurrency-mode",
                        value_type="enum",
                        in_sections=("extensions",),
                        enum_values=("dedicated", "single"),
                    ),
                    BigipPropertySpec(
                        name="data-groups",
                        value_type="reference",
                        in_sections=("extensions",),
                        repeated=True,
                        allow_none=True,
                        references=(
                            "ltm_data_group_external",
                            "ltm_data_group_internal",
                            "sys_file_data_group",
                        ),
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="debug-port-range-high",
                        value_type="unknown",
                        in_sections=("extensions",),
                    ),
                    BigipPropertySpec(
                        name="debug-port-range-low",
                        value_type="unknown",
                        in_sections=("extensions",),
                    ),
                    BigipPropertySpec(
                        name="description",
                        value_type="string",
                        in_sections=("extensions",),
                    ),
                    BigipPropertySpec(
                        name="heartbeat-interval",
                        value_type="unknown",
                        in_sections=("extensions",),
                    ),
                    BigipPropertySpec(
                        name="ilx-logging",
                        value_type="enum",
                        in_sections=("extensions",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                        default="disabled",
                    ),
                    BigipPropertySpec(
                        name="log-publisher",
                        value_type="reference",
                        in_sections=("extensions",),
                    ),
                    BigipPropertySpec(
                        name="max-restarts",
                        value_type="integer",
                        in_sections=("extensions",),
                        default="5",
                    ),
                    BigipPropertySpec(
                        name="restart-interval",
                        value_type="integer",
                        in_sections=("extensions",),
                        default="60 seconds",
                    ),
                    BigipPropertySpec(
                        name="trace-level",
                        value_type="integer",
                        in_sections=("extensions",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="command-arguments",
                value_type="unknown",
                in_sections=("extensions",),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="command-options",
                value_type="unknown",
                in_sections=("extensions",),
            ),
            BigipPropertySpec(
                name="concurrency-mode",
                value_type="enum",
                in_sections=("extensions",),
                enum_values=("dedicated", "single"),
            ),
            BigipPropertySpec(
                name="data-groups",
                value_type="reference",
                in_sections=("extensions",),
                repeated=True,
                allow_none=True,
                references=(
                    "ltm_data_group_external",
                    "ltm_data_group_internal",
                    "sys_file_data_group",
                ),
                default="none",
            ),
            BigipPropertySpec(
                name="debug-port-range-high",
                value_type="unknown",
                in_sections=("extensions",),
            ),
            BigipPropertySpec(
                name="debug-port-range-low",
                value_type="unknown",
                in_sections=("extensions",),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("extensions",)),
            BigipPropertySpec(
                name="heartbeat-interval",
                value_type="unknown",
                in_sections=("extensions",),
            ),
            BigipPropertySpec(
                name="ilx-logging",
                value_type="enum",
                in_sections=("extensions",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="reference",
                in_sections=("extensions",),
            ),
            BigipPropertySpec(
                name="max-restarts",
                value_type="integer",
                in_sections=("extensions",),
                default="5",
            ),
            BigipPropertySpec(
                name="restart-interval",
                value_type="integer",
                in_sections=("extensions",),
                default="60 seconds",
            ),
            BigipPropertySpec(
                name="trace-level",
                value_type="integer",
                in_sections=("extensions",),
            ),
            BigipPropertySpec(name="from-workspace", value_type="reference"),
            BigipPropertySpec(name="log-publisher", value_type="reference"),
        ),
    )
