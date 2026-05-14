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
            "ltm_dns_zone",
            module="ltm",
            object_types=("dns zone",),
        ),
        header_types=(("ltm", "dns zone"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="dns-express-allow-notify",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="dns-express-enabled",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="dns-express-notify-action",
                value_type="enum",
                enum_values=("bypass", "consume", "repeat"),
            ),
            BigipPropertySpec(
                name="dns-express-notify-tsig-verify",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="dns-express-server",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="response-policy", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="server-tsig-key",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="transfer-clients",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
