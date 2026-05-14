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
            BigipPropertySpec(name="cache-timeout", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="services",
                value_type="reference",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="alt-hash-fields",
                value_type="reference",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest-ip", "dest-port", "src-ip", "src-port"),
                list_operators=frozenset(("none",)),
            ),
            BigipPropertySpec(
                name="hash-fields",
                value_type="reference",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest-ip", "dest-port", "src-ip", "src-port"),
                list_operators=frozenset(("none",)),
            ),
            BigipPropertySpec(
                name="password", value_type="boolean", in_sections=("services",), allow_none=True
            ),
            BigipPropertySpec(
                name="port-type",
                value_type="reference",
                in_sections=("services",),
                allow_none=True,
                enum_values=("dest", "source"),
                list_operators=frozenset(("none",)),
            ),
            BigipPropertySpec(name="ports", value_type="integer", in_sections=("services",)),
            BigipPropertySpec(name="priority", value_type="integer", in_sections=("services",)),
            BigipPropertySpec(
                name="protocol",
                value_type="enum",
                in_sections=("services",),
                enum_values=("tcp", "udp"),
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
                value_type="reference",
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
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="tunnel-remote-addresses",
                value_type="reference",
                in_sections=("services",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="weight", value_type="integer", in_sections=("services",)),
        ),
    )
