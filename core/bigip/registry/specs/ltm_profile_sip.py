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
            "ltm_profile_sip",
            module="ltm",
            object_types=("profile sip",),
        ),
        header_types=(("ltm", "profile sip"),),
        properties=(
            BigipPropertySpec(
                name="alg-enable", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="community", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_sip",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dialog-aware", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="dialog-establishment-timeout", value_type="integer"),
            BigipPropertySpec(
                name="enable-sip-firewall", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="insert-record-route-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="insert-via-header", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-media-sessions", value_type="integer"),
            BigipPropertySpec(name="max-registrations", value_type="integer"),
            BigipPropertySpec(name="max-sessions-per-registration", value_type="integer"),
            BigipPropertySpec(name="max-size", value_type="integer"),
            BigipPropertySpec(name="registration-timeout", value_type="integer"),
            BigipPropertySpec(
                name="rtp-proxy-style",
                value_type="enum",
                enum_values=("symmetric", "restricted-by-ip-address", "any-location"),
            ),
            BigipPropertySpec(
                name="secure-via-header", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="security", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="sip-session-timeout", value_type="integer"),
            BigipPropertySpec(
                name="terminate-on-bye", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="user-via-header", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-profile", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
