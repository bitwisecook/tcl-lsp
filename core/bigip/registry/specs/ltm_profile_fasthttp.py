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
            "ltm_profile_fasthttp",
            module="ltm",
            object_types=("profile fasthttp",),
        ),
        header_types=(("ltm", "profile fasthttp"),),
        properties=(
            BigipPropertySpec(name="client-close-timeout", value_type="integer"),
            BigipPropertySpec(name="connpool-idle-timeout-override", value_type="integer"),
            BigipPropertySpec(name="connpool-max-reuse", value_type="integer"),
            BigipPropertySpec(name="connpool-max-size", value_type="integer"),
            BigipPropertySpec(name="connpool-min-size", value_type="integer"),
            BigipPropertySpec(
                name="connpool-replenish", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="connpool-step", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_fasthttp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="force-http-10-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="hardware-syn-cookie", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="header-insert", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="http-11-close-workarounds",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="insert-xforwarded-for", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="layer-7", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-header-size", value_type="integer"),
            BigipPropertySpec(name="max-requests", value_type="integer"),
            BigipPropertySpec(name="mss-override", value_type="integer"),
            BigipPropertySpec(name="receive-window-size", value_type="string"),
            BigipPropertySpec(
                name="reset-on-timeout", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="server-close-timeout", value_type="integer"),
            BigipPropertySpec(
                name="server-sack", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="server-timestamp", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="unclean-shutdown", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
