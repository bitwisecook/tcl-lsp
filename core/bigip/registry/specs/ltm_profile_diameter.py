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
                name="connection-prime", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_profile_diameter",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination-realm", value_type="string"),
            BigipPropertySpec(name="handshake-timeout", value_type="string"),
            BigipPropertySpec(
                name="host-ip-rewrite", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-retransmit-attempts", value_type="string"),
            BigipPropertySpec(name="max-watchdog-failure", value_type="string"),
            BigipPropertySpec(name="origin-host-to-client", value_type="string"),
            BigipPropertySpec(name="origin-host-to-server", value_type="string"),
            BigipPropertySpec(name="origin-realm-to-client", value_type="string"),
            BigipPropertySpec(name="origin-realm-to-server", value_type="string"),
            BigipPropertySpec(
                name="overwrite-destination-host",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="parent-avp", value_type="string"),
            BigipPropertySpec(name="persist-avp", value_type="string"),
            BigipPropertySpec(
                name="reset-on-timeout", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="retransmit-timeout", value_type="string"),
            BigipPropertySpec(name="watchdog-timeout", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
