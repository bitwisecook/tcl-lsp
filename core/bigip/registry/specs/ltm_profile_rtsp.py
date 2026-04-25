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
            "ltm_profile_rtsp",
            module="ltm",
            object_types=("profile rtsp",),
        ),
        header_types=(("ltm", "profile rtsp"),),
        properties=(
            BigipPropertySpec(
                name="check-source", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_profile_rtsp",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="max-header-size", value_type="integer"),
            BigipPropertySpec(name="max-queued-data", value_type="integer"),
            BigipPropertySpec(
                name="multicast-redirect", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="proxy",
                value_type="enum",
                allow_none=True,
                enum_values=("external", "internal", "none"),
            ),
            BigipPropertySpec(name="proxy-header", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="real-http-persistence", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="rtcp-port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(name="rtp-port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="session-reconnect", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="unicast-redirect", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-profile", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
