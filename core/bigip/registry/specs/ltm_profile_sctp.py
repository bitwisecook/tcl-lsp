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
            "ltm_profile_sctp",
            module="ltm",
            object_types=("profile sctp",),
        ),
        header_types=(("ltm", "profile sctp"),),
        properties=(
            BigipPropertySpec(name="cookie-expiration", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_profile_sctp",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="heartbeat-interval", value_type="integer"),
            BigipPropertySpec(name="heartbeat-max-burst", value_type="integer"),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="in-streams", value_type="integer"),
            BigipPropertySpec(name="init-max-retries", value_type="integer"),
            BigipPropertySpec(name="ip-tos", value_type="integer"),
            BigipPropertySpec(name="link-qos", value_type="integer"),
            BigipPropertySpec(name="max-burst", value_type="integer"),
            BigipPropertySpec(name="out-streams", value_type="integer"),
            BigipPropertySpec(name="proxy-buffer-high", value_type="integer"),
            BigipPropertySpec(name="proxy-buffer-low", value_type="integer"),
            BigipPropertySpec(name="receive-chunks", value_type="integer"),
            BigipPropertySpec(
                name="receive-ordered", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="receive-window-size", value_type="integer"),
            BigipPropertySpec(
                name="reset-on-timeout", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="rto-initial", value_type="integer"),
            BigipPropertySpec(name="rto-max", value_type="integer"),
            BigipPropertySpec(name="rto-min", value_type="integer"),
            BigipPropertySpec(name="sack-timeout", value_type="integer"),
            BigipPropertySpec(name="secret", value_type="string"),
            BigipPropertySpec(name="send-buffer-size", value_type="integer"),
            BigipPropertySpec(name="send-max-retries", value_type="integer"),
            BigipPropertySpec(
                name="send-partial", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="tcp-shutdown", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="transmit-chunks", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
