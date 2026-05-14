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
            "apm_configuration_captcha",
            module="apm",
            object_types=("configuration captcha",),
        ),
        header_types=(("apm", "configuration captcha"),),
        properties=(
            BigipPropertySpec(
                name="captcha-data-size",
                value_type="enum",
                enum_values=("data-size-compact", "data-size-normal"),
            ),
            BigipPropertySpec(
                name="captcha-data-theme",
                value_type="enum",
                enum_values=("data-theme-dark", "data-theme-light"),
            ),
            BigipPropertySpec(
                name="captcha-data-type",
                value_type="enum",
                enum_values=("data-type-audio", "data-type-image"),
            ),
            BigipPropertySpec(
                name="captcha-theme",
                value_type="enum",
                enum_values=(
                    "theme-blackglass",
                    "theme-clean",
                    "theme-custom",
                    "theme-red",
                    "theme-white",
                ),
            ),
            BigipPropertySpec(name="challenge-url", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="exposition-threshold", value_type="integer"),
            BigipPropertySpec(name="noscript-url", value_type="string"),
            BigipPropertySpec(name="private-key", value_type="unknown"),
            BigipPropertySpec(
                name="proceed-on-verification-error",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="public-key", value_type="unknown"),
            BigipPropertySpec(name="secret", value_type="unknown"),
            BigipPropertySpec(name="site-key", value_type="unknown"),
            BigipPropertySpec(name="track-by-ip", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(
                name="track-by-username",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="verification-url", value_type="string"),
        ),
    )
