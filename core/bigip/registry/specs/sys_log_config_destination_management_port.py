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
            "sys_log_config_destination_management_port",
            module="sys",
            object_types=("log-config destination management-port",),
        ),
        header_types=(("sys", "log-config destination management-port"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ip-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(name="protocol", value_type="enum", enum_values=("tcp", "udp")),
        ),
    )
