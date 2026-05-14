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
            "wom_local_endpoint",
            module="wom",
            object_types=("local-endpoint",),
        ),
        header_types=(("wom", "local-endpoint"),),
        properties=(
            BigipPropertySpec(
                name="addresses",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="allow-nat",
                value_type="enum",
                enum_values=("disabled", "enabled"),
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
                enum_values=("disabled", "enabled"),
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
                enum_values=("gre", "ipip", "ipsec", "none"),
            ),
            BigipPropertySpec(name="no-route", value_type="enum", enum_values=("drop", "passthru")),
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
                enum_values=("local", "none", "remote"),
            ),
            BigipPropertySpec(name="tunnel-port", value_type="integer"),
        ),
    )
