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
            "net_tunnels_fec",
            module="net",
            object_types=("tunnels fec",),
        ),
        header_types=(("net", "tunnels fec"),),
        properties=(
            BigipPropertySpec(name="decode-idle-timeout", value_type="integer"),
            BigipPropertySpec(name="decode-max-packets", value_type="integer"),
            BigipPropertySpec(name="decode-queues", value_type="integer"),
            BigipPropertySpec(name="defaults-from", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="encode-max-delay", value_type="integer"),
            BigipPropertySpec(name="keepalive-interval", value_type="integer"),
            BigipPropertySpec(name="lzo", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="repair-adaptive", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="repair-packets", value_type="integer"),
            BigipPropertySpec(
                name="source-adaptive", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="source-packets", value_type="integer"),
            BigipPropertySpec(name="udp-port", value_type="integer", min_value=0, max_value=65535),
        ),
    )
