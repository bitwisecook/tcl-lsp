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
            "cm_device",
            module="cm",
            object_types=("device",),
        ),
        header_types=(("cm", "device"),),
        properties=(
            BigipPropertySpec(name="comment", value_type="string"),
            BigipPropertySpec(
                name="configsync-ip",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="contact", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ha-capacity", value_type="integer"),
            BigipPropertySpec(name="location", value_type="string"),
            BigipPropertySpec(
                name="mgmt-unicast-mode",
                value_type="enum",
                enum_values=("both", "ipv4", "ipv6"),
            ),
            BigipPropertySpec(
                name="mirror-ip",
                value_type="enum",
                enum_values=("any6",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="mirror-secondary-ip",
                value_type="enum",
                enum_values=("any6",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="multicast-interface", value_type="string"),
            BigipPropertySpec(name="multicast-ip", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(name="multicast-port", value_type="integer"),
            BigipPropertySpec(
                name="unicast-address",
                value_type="block",
                allow_none=True,
                shape_kind="list",
                block=(
                    BigipPropertySpec(
                        name="effective-ip",
                        value_type="string",
                        in_sections=("unicast-address",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="effective-port",
                        value_type="unknown",
                        in_sections=("unicast-address",),
                    ),
                    BigipPropertySpec(
                        name="ip",
                        value_type="string",
                        in_sections=("unicast-address",),
                        shape_kind="ip-address",
                    ),
                    BigipPropertySpec(
                        name="port",
                        value_type="unknown",
                        in_sections=("unicast-address",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="effective-ip",
                value_type="string",
                in_sections=("unicast-address",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="effective-port",
                value_type="unknown",
                in_sections=("unicast-address",),
            ),
            BigipPropertySpec(
                name="ip",
                value_type="string",
                in_sections=("unicast-address",),
                shape_kind="ip-address",
            ),
            BigipPropertySpec(name="port", value_type="unknown", in_sections=("unicast-address",)),
            BigipPropertySpec(
                name="version",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="active-modules",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="base-mac",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="build",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="chassis-id",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="edition",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="hostname",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="management-ip",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="marketing-name",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="optional-modules",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="platform-id",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="product",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="time-zone",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="cert",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="key",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="self-device",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="time-limited-modules",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
