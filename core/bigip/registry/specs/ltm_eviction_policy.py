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
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="high-water", value_type="integer"),
            BigipPropertySpec(name="low-water", value_type="integer", default="85 percent"),
            BigipPropertySpec(
                name="slow-flow",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("slow-flow",),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                    BigipPropertySpec(
                        name="eviction-type",
                        value_type="enum",
                        in_sections=("slow-flow",),
                        enum_values=("count", "percent"),
                    ),
                    BigipPropertySpec(
                        name="grace-period",
                        value_type="integer",
                        in_sections=("slow-flow",),
                    ),
                    BigipPropertySpec(
                        name="maximum", value_type="integer", in_sections=("slow-flow",)
                    ),
                    BigipPropertySpec(
                        name="threshold-bps",
                        value_type="integer",
                        in_sections=("slow-flow",),
                    ),
                    BigipPropertySpec(
                        name="throttling",
                        value_type="enum",
                        in_sections=("slow-flow",),
                        enum_values=("disabled", "enabled"),
                        shape_kind="boolean",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("slow-flow",),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="eviction-type",
                value_type="enum",
                in_sections=("slow-flow",),
                enum_values=("count", "percent"),
            ),
            BigipPropertySpec(
                name="grace-period",
                value_type="integer",
                in_sections=("slow-flow",),
            ),
            BigipPropertySpec(name="maximum", value_type="integer", in_sections=("slow-flow",)),
            BigipPropertySpec(
                name="threshold-bps",
                value_type="integer",
                in_sections=("slow-flow",),
            ),
            BigipPropertySpec(
                name="throttling",
                value_type="enum",
                in_sections=("slow-flow",),
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="strategies",
                value_type="list",
                block=(
                    BigipPropertySpec(
                        name="bias-bytes",
                        value_type="list",
                        in_sections=("strategies",),
                    ),
                    BigipPropertySpec(
                        name="bias-idle",
                        value_type="unknown",
                        in_sections=("strategies",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="bias-oldest",
                        value_type="unknown",
                        in_sections=("strategies",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="low-priority-geographies",
                        value_type="list",
                        in_sections=("strategies",),
                    ),
                    BigipPropertySpec(
                        name="low-priority-port",
                        value_type="unknown",
                        in_sections=("strategies",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="low-priority-route-domain",
                        value_type="unknown",
                        in_sections=("strategies",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="low-priority-virtual-server",
                        value_type="unknown",
                        in_sections=("strategies",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="bias-bytes",
                value_type="list",
                in_sections=("strategies",),
                block=(
                    BigipPropertySpec(
                        name="delay",
                        value_type="integer",
                        in_sections=("strategies", "bias-bytes"),
                    ),
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("strategies", "bias-bytes"),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="delay",
                value_type="integer",
                in_sections=("strategies", "bias-bytes"),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("strategies", "bias-bytes"),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="bias-idle",
                value_type="unknown",
                in_sections=("strategies",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("strategies", "bias-idle"),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("strategies", "bias-idle"),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="bias-oldest",
                value_type="unknown",
                in_sections=("strategies",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("strategies", "bias-oldest"),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("strategies", "bias-oldest"),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="low-priority-geographies",
                value_type="list",
                in_sections=("strategies",),
                block=(
                    BigipPropertySpec(
                        name="countries",
                        value_type="list",
                        in_sections=("strategies", "low-priority-geographies"),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("strategies", "low-priority-geographies"),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="countries",
                value_type="list",
                in_sections=("strategies", "low-priority-geographies"),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("strategies", "low-priority-geographies"),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="low-priority-port",
                value_type="unknown",
                in_sections=("strategies",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("strategies", "low-priority-port"),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                    BigipPropertySpec(
                        name="ports",
                        value_type="list",
                        in_sections=("strategies", "low-priority-port"),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("strategies", "low-priority-port"),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="ports",
                value_type="list",
                in_sections=("strategies", "low-priority-port"),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("strategies", "low-priority-port", "ports"),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="port-number",
                        value_type="reference",
                        in_sections=("strategies", "low-priority-port", "ports"),
                    ),
                    BigipPropertySpec(
                        name="protocol",
                        value_type="enum",
                        in_sections=("strategies", "low-priority-port", "ports"),
                        enum_values=("any", "sctp", "tcp", "udp"),
                        default="any",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("strategies", "low-priority-port", "ports"),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="port-number",
                value_type="reference",
                in_sections=("strategies", "low-priority-port", "ports"),
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                in_sections=("strategies", "low-priority-port", "ports"),
                enum_values=("any", "sctp", "tcp", "udp"),
                default="any",
            ),
            BigipPropertySpec(
                name="low-priority-route-domain",
                value_type="unknown",
                in_sections=("strategies",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("strategies", "low-priority-route-domain"),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                    BigipPropertySpec(
                        name="names",
                        value_type="list",
                        in_sections=("strategies", "low-priority-route-domain"),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("strategies", "low-priority-route-domain"),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="names",
                value_type="list",
                in_sections=("strategies", "low-priority-route-domain"),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="low-priority-virtual-server",
                value_type="unknown",
                in_sections=("strategies",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="enabled",
                        value_type="enum",
                        in_sections=("strategies", "low-priority-virtual-server"),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="false",
                    ),
                    BigipPropertySpec(
                        name="names",
                        value_type="list",
                        in_sections=("strategies", "low-priority-virtual-server"),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("strategies", "low-priority-virtual-server"),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="names",
                value_type="list",
                in_sections=("strategies", "low-priority-virtual-server"),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
        ),
    )
