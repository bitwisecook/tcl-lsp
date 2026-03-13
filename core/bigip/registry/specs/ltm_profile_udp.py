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
            "ltm_profile_udp",
            module="ltm",
            object_types=("profile udp",),
        ),
        header_types=(("ltm", "profile udp"),),
        properties=(
            BigipPropertySpec(
                name="allow-no-payload", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="buffer-max-bytes", value_type="integer"),
            BigipPropertySpec(name="buffer-max-packets", value_type="integer"),
            BigipPropertySpec(
                name="datagram-load-balancing",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_udp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="ip-tos-to-client", value_type="integer"),
            BigipPropertySpec(name="link-qos-to-client", value_type="integer"),
            BigipPropertySpec(
                name="no-checksum", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="proxy-mss", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ip-ttl-mode",
                value_type="enum",
                enum_values=("proxy", "preserve", "decrement", "set"),
            ),
            BigipPropertySpec(name="ip-ttl-v4", value_type="integer"),
            BigipPropertySpec(name="ip-ttl-v6", value_type="integer"),
            BigipPropertySpec(
                name="ip-df-mode",
                value_type="enum",
                enum_values=("pmtu", "preserve", "set", "clear"),
            ),
            BigipPropertySpec(name="send-buffer-size", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
