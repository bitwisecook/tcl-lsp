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
            "ltm_monitor_sip",
            module="ltm",
            object_types=("monitor sip",),
        ),
        header_types=(("ltm", "monitor sip"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cert", value_type="unknown", allow_none=True, default="none"),
            BigipPropertySpec(
                name="cipherlist",
                value_type="string",
                default="DEFAULT:+SHA:+3DES:+kEDH",
            ),
            BigipPropertySpec(
                name="compatibility",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_sip",),
                default="sip",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(
                name="filter",
                value_type="enum",
                allow_none=True,
                enum_values=("any", "none", "status"),
            ),
            BigipPropertySpec(
                name="filter-neg",
                value_type="enum",
                allow_none=True,
                enum_values=("any", "none", "status"),
            ),
            BigipPropertySpec(
                name="headers",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="key", value_type="unknown", allow_none=True, default="none"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=(
                    "mr-sctp",
                    "mr-sips",
                    "mr-tcp",
                    "mr-tls",
                    "mr-udp",
                    "sips",
                    "tcp",
                    "tls",
                    "udp",
                ),
            ),
            BigipPropertySpec(name="request", value_type="string", default="none"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(
                name="up-interval",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            BigipPropertySpec(name="username", value_type="reference", allow_none=True),
            BigipPropertySpec(
                name="timeout",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="Foo2:",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="Foo3:",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="Foo4:",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
