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
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="cookie-expiration", value_type="integer", default="60 seconds"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_profile_sctp",),
                default="sctp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="heartbeat-interval",
                value_type="integer",
                default="30 seconds",
            ),
            BigipPropertySpec(name="heartbeat-max-burst", value_type="integer", default="1"),
            BigipPropertySpec(name="idle-timeout", value_type="integer", default="300 seconds"),
            BigipPropertySpec(name="in-streams", value_type="integer", default="1"),
            BigipPropertySpec(name="init-max-retries", value_type="integer", default="4"),
            BigipPropertySpec(name="ip-tos", value_type="integer", default="0"),
            BigipPropertySpec(name="link-qos", value_type="integer", default="0"),
            BigipPropertySpec(name="max-burst", value_type="integer", default="4"),
            BigipPropertySpec(name="out-streams", value_type="integer", default="2"),
            BigipPropertySpec(name="proxy-buffer-high", value_type="integer", default="16384"),
            BigipPropertySpec(name="proxy-buffer-low", value_type="integer", default="4096"),
            BigipPropertySpec(name="receive-chunks", value_type="integer", default="256"),
            BigipPropertySpec(
                name="receive-ordered",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="receive-window-size", value_type="integer", default="65535"),
            BigipPropertySpec(
                name="reset-on-timeout",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="rto-initial",
                value_type="integer",
                default="3000 milliseconds",
            ),
            BigipPropertySpec(name="rto-max", value_type="integer", default="60000 milliseconds"),
            BigipPropertySpec(name="rto-min", value_type="integer", default="1000 milliseconds"),
            BigipPropertySpec(
                name="sack-timeout",
                value_type="integer",
                default="200 milliseconds",
            ),
            BigipPropertySpec(name="secret", value_type="string"),
            BigipPropertySpec(name="send-buffer-size", value_type="integer", default="65536"),
            BigipPropertySpec(name="send-max-retries", value_type="integer", default="8"),
            BigipPropertySpec(
                name="send-partial",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="tcp-shutdown",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="transmit-chunks", value_type="integer", default="256"),
        ),
    )
