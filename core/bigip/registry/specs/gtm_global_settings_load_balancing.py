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
            "gtm_global_settings_load_balancing",
            module="gtm",
            object_types=("global-settings load-balancing",),
        ),
        header_types=(("gtm", "global-settings load-balancing"),),
        properties=(
            BigipPropertySpec(
                name="failure-rcode",
                value_type="enum",
                enum_values=("noerror", "formerr", "servfail", "nxdomain", "notimpl", "refused"),
            ),
            BigipPropertySpec(name="failure-rcode-ttl", value_type="integer"),
            BigipPropertySpec(
                name="failure-rcode-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="ignore-path-ttl", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="respect-fallback-dependency", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="topology-longest-match", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="topology-prefer-edns0-client-subnet",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="verify-vs-availability", value_type="enum", enum_values=("no", "yes")
            ),
        ),
    )
