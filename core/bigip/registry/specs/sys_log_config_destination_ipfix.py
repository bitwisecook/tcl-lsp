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
            "sys_log_config_destination_ipfix",
            module="sys",
            object_types=("log-config destination ipfix",),
        ),
        header_types=(("sys", "log-config destination ipfix"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="pool-name", value_type="string"),
            BigipPropertySpec(
                name="protocol-version", value_type="enum", enum_values=("ipfix", "netflow-9")
            ),
            BigipPropertySpec(name="template-delete-delay", value_type="integer"),
            BigipPropertySpec(name="template-retransmit-interval", value_type="integer"),
            BigipPropertySpec(name="transport-profile", value_type="string"),
            BigipPropertySpec(name="serverssl-profile", value_type="string"),
        ),
    )
