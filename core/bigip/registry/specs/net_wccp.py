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
            "net_wccp",
            module="net",
            object_types=("wccp",),
        ),
        header_types=(("net", "wccp"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cache-timeout", value_type="integer", default="10"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="services",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="alt-hash-fields",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("dest-ip", "none", "src-ip"),
                    ),
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("services",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="hash-fields",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("dest-ip", "none", "src-ip"),
                    ),
                    BigipPropertySpec(
                        name="password",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("none",),
                    ),
                    BigipPropertySpec(
                        name="port-type",
                        value_type="enum",
                        in_sections=("services",),
                        allow_none=True,
                        enum_values=("dest", "none", "source"),
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="ports", value_type="integer", in_sections=("services",)
                    ),
                    BigipPropertySpec(
                        name="priority",
                        value_type="integer",
                        in_sections=("services",),
                        default="100",
                    ),
                    BigipPropertySpec(
                        name="protocol",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("tcp", "udp"),
                        default="tcp",
                    ),
                    BigipPropertySpec(
                        name="redirection-method",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("gre", "l2"),
                    ),
                    BigipPropertySpec(
                        name="return-method",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("gre", "l2"),
                    ),
                    BigipPropertySpec(
                        name="routers",
                        value_type="list",
                        in_sections=("services",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="traffic-assign",
                        value_type="enum",
                        in_sections=("services",),
                        enum_values=("hash", "mask"),
                    ),
                    BigipPropertySpec(
                        name="tunnel-local-address",
                        value_type="string",
                        in_sections=("services",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="tunnel-remote-addresses",
                        value_type="list",
                        in_sections=("services",),
                        list_operators=frozenset(("add", "delete", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="weight",
                        value_type="integer",
                        in_sections=("services",),
                        default="50",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="alt-hash-fields",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest-ip", "none", "src-ip"),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("services",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="hash-fields",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest-ip", "none", "src-ip"),
            ),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="port-type",
                value_type="enum",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest", "none", "source"),
                default="none",
            ),
            BigipPropertySpec(name="ports", value_type="integer", in_sections=("services",)),
            BigipPropertySpec(
                name="priority",
                value_type="integer",
                in_sections=("services",),
                default="100",
            ),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                in_sections=("services",),
                enum_values=("tcp", "udp"),
                default="tcp",
            ),
            BigipPropertySpec(
                name="redirection-method",
                value_type="enum",
                in_sections=("services",),
                enum_values=("gre", "l2"),
            ),
            BigipPropertySpec(
                name="return-method",
                value_type="enum",
                in_sections=("services",),
                enum_values=("gre", "l2"),
            ),
            BigipPropertySpec(
                name="routers",
                value_type="list",
                in_sections=("services",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="traffic-assign",
                value_type="enum",
                in_sections=("services",),
                enum_values=("hash", "mask"),
            ),
            BigipPropertySpec(
                name="tunnel-local-address",
                value_type="string",
                in_sections=("services",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="tunnel-remote-addresses",
                value_type="list",
                in_sections=("services",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="weight",
                value_type="integer",
                in_sections=("services",),
                default="50",
            ),
        ),
    )
