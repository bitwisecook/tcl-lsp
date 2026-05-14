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
            "wom_remote_endpoint",
            module="wom",
            object_types=("remote-endpoint",),
        ),
        header_types=(("wom", "remote-endpoint"),),
        properties=(
            BigipPropertySpec(name="address", value_type="string"),
            BigipPropertySpec(
                name="allow-routing",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="dedup-action",
                value_type="enum",
                allow_none=True,
                enum_values=("cache-refresh", "none"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="endpoint",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="internal-forwarding",
                value_type="enum",
                enum_values=("default", "disabled", "enabled"),
            ),
            BigipPropertySpec(name="ip-encap-mtu", value_type="integer"),
            BigipPropertySpec(
                name="ip-encap-profile",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="ip-encap-type",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "gre", "ipip", "ipsec", "none"),
            ),
            BigipPropertySpec(
                name="origin",
                value_type="enum",
                enum_values=("configured", "discovered", "manually-saved", "persistable"),
            ),
            BigipPropertySpec(
                name="server-ssl",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
                references=("ltm_profile_server_ssl",),
            ),
            BigipPropertySpec(
                name="snat",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "local", "none", "remote"),
            ),
            BigipPropertySpec(
                name="tunnel-encrypt",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="tunnel-port", value_type="integer"),
        ),
    )
