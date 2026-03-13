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
            "ltm_eviction_policy",
            module="ltm",
            object_types=("eviction-policy",),
        ),
        header_types=(("ltm", "eviction-policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="low-water", value_type="integer"),
            BigipPropertySpec(name="high-water", value_type="integer"),
            BigipPropertySpec(name="slow-flow", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("slow-flow",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="eviction-type",
                value_type="enum",
                in_sections=("slow-flow",),
                enum_values=("count", "percent"),
            ),
            BigipPropertySpec(
                name="grace-period", value_type="integer", in_sections=("slow-flow",)
            ),
            BigipPropertySpec(name="maximum", value_type="integer", in_sections=("slow-flow",)),
            BigipPropertySpec(
                name="threshold-bps", value_type="integer", in_sections=("slow-flow",)
            ),
            BigipPropertySpec(
                name="throttling",
                value_type="enum",
                in_sections=("slow-flow",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="strategies", value_type="string"),
            BigipPropertySpec(name="bias-bytes", value_type="string", in_sections=("strategies",)),
            BigipPropertySpec(name="delay", value_type="integer", in_sections=("bias-bytes",)),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("bias-bytes",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(name="bias-idle", value_type="string", in_sections=("strategies",)),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("bias-idle",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(name="bias-oldest", value_type="string", in_sections=("strategies",)),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("bias-oldest",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="low-priority-geographies", value_type="string", in_sections=("strategies",)
            ),
            BigipPropertySpec(
                name="countries",
                value_type="enum",
                in_sections=("low-priority-geographies",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("low-priority-geographies",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="low-priority-port",
                value_type="integer",
                in_sections=("strategies",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("low-priority-port",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="ports",
                value_type="enum",
                in_sections=("low-priority-port",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="port-number", value_type="integer", in_sections=("ports",)),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                in_sections=("ports",),
                enum_values=("any", "sctp", "tcp", "udp"),
            ),
            BigipPropertySpec(name="low-priority-route-domain", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("low-priority-route-domain",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="names",
                value_type="enum",
                in_sections=("low-priority-route-domain",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="low-priority-virtual-server", value_type="string"),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("low-priority-virtual-server",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="names",
                value_type="enum",
                in_sections=("low-priority-virtual-server",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
        ),
    )
