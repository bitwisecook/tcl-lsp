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
            "gtm_listener_doh_server",
            module="gtm",
            object_types=("listener-doh-server",),
        ),
        header_types=(("gtm", "listener-doh-server"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="advertise", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "enabled", "disabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="fallback-persistence", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ip-protocol", value_type="string"),
            BigipPropertySpec(
                name="last-hop-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(name="mask", value_type="list", repeated=True),
            BigipPropertySpec(name="persist", value_type="string"),
            BigipPropertySpec(
                name="default",
                value_type="enum",
                in_sections=("persist",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="pool", value_type="reference", allow_none=True, references=("ltm_pool",)
            ),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="profiles",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="context",
                value_type="enum",
                in_sections=("profiles",),
                enum_values=("all", "clientside", "serverside"),
            ),
            BigipPropertySpec(
                name="rules",
                value_type="reference",
                repeated=True,
                allow_none=True,
                references=("gtm_rule",),
            ),
            BigipPropertySpec(name="source-address-translation", value_type="string"),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
                in_sections=("source-address-translation",),
                allow_none=True,
                references=("ltm_snatpool", "ltm_pool"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("source-address-translation",),
                allow_none=True,
                enum_values=("automap", "snat", "none"),
            ),
            BigipPropertySpec(
                name="source-port",
                value_type="enum",
                enum_values=("change", "preserve"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="translate-address",
                value_type="enum",
                enum_values=("enabled", "disabled"),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="translate-port",
                value_type="enum",
                enum_values=("enabled", "disabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="vlans", value_type="reference", allow_none=True, references=("net_vlan",)
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
