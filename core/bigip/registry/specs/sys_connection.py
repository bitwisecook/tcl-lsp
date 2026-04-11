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
            "sys_connection",
            module="sys",
            object_types=("connection",),
        ),
        header_types=(("sys", "connection"),),
        properties=(
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="flow-accel-type", value_type="string"),
            BigipPropertySpec(name="age", value_type="integer"),
            BigipPropertySpec(name="cs-client-addr", value_type="string"),
            BigipPropertySpec(
                name="cs-client-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="cs-server-addr", value_type="string"),
            BigipPropertySpec(
                name="cs-server-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="connection-id", value_type="integer"),
            BigipPropertySpec(name="protocol", value_type="string"),
            BigipPropertySpec(name="save-to-file", value_type="string"),
            BigipPropertySpec(name="ss-client-addr", value_type="string"),
            BigipPropertySpec(
                name="ss-client-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="ss-server-addr", value_type="string"),
            BigipPropertySpec(
                name="ss-server-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(
                name="type", value_type="enum", enum_values=("any", "mirror", "self", "mptcp")
            ),
            BigipPropertySpec(name="virtual-server", value_type="string"),
        ),
    )
