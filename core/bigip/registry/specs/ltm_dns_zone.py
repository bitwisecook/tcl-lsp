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
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="dns-express-allow-notify",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="dns-express-enabled", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(
                name="dns-express-notify-action",
                value_type="enum",
                enum_values=("consume", "bypass", "repeat"),
            ),
            BigipPropertySpec(
                name="dns-express-notify-tsig-verify", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(name="dns-express-server", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="response-policy", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="server-tsig-key", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="transfer-clients",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
