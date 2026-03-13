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
            "ltm_persistence_cookie",
            module="ltm",
            object_types=("persistence cookie",),
        ),
        header_types=(("ltm", "persistence cookie"),),
        properties=(
            BigipPropertySpec(
                name="always-send", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="cookie-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="cookie-encryption",
                value_type="enum",
                enum_values=("required", "preferred", "disabled"),
            ),
            BigipPropertySpec(
                name="cookie-encryption-passphrase", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_persistence_cookie",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="expiration", value_type="string"),
            BigipPropertySpec(
                name="httponly", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="secure", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="hash-length", value_type="integer"),
            BigipPropertySpec(name="hash-offset", value_type="integer"),
            BigipPropertySpec(
                name="match-across-pools", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="match-across-services", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="match-across-virtuals", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="method",
                value_type="enum",
                enum_values=("hash", "insert", "passive", "rewrite"),
            ),
            BigipPropertySpec(
                name="mirror", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="override-connection-limit",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="encrypt-cookie-poolname",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
        ),
    )
