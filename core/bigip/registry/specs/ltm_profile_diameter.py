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
            "ltm_profile_diameter",
            module="ltm",
            object_types=("profile diameter",),
        ),
        header_types=(("ltm", "profile diameter"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="connection-prime",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_profile_diameter",),
                default="diameter",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-realm",
                value_type="string",
                default="none",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="handshake-timeout", value_type="unknown", default="10"),
            BigipPropertySpec(
                name="host-ip-rewrite",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="max-retransmit-attempts", value_type="unknown", default="1"),
            BigipPropertySpec(name="max-watchdog-failure", value_type="unknown", default="10"),
            BigipPropertySpec(name="origin-host-to-client", value_type="string", default="none"),
            BigipPropertySpec(name="origin-host-to-server", value_type="string", default="none"),
            BigipPropertySpec(name="origin-realm-to-client", value_type="string", default="none"),
            BigipPropertySpec(name="origin-realm-to-server", value_type="string", default="none"),
            BigipPropertySpec(
                name="overwrite-destination-host",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="parent-avp", value_type="string", default="none"),
            BigipPropertySpec(name="persist-avp", value_type="string", default="session-id"),
            BigipPropertySpec(
                name="reset-on-timeout",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="retransmit-timeout", value_type="unknown", default="10"),
            BigipPropertySpec(
                name="watchdog-timeout",
                value_type="unknown",
                default="0, which means BIG-IP will not send a device watchdog request to either client or server side",
            ),
        ),
    )
