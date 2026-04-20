"""Tk widget and geometry manager command definitions.

Import all command modules here so their ``@register`` decorators fire.
"""

from __future__ import annotations

import builtins
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ..models import CommandSpec

# Import command modules to trigger @register decorators
from . import (
    bell,  # noqa: F401
    bind,  # noqa: F401
    button,  # noqa: F401
    canvas,  # noqa: F401
    checkbutton,  # noqa: F401
    clipboard,  # noqa: F401
    destroy,  # noqa: F401
    entry,  # noqa: F401
    event_,  # noqa: F401
    focus,  # noqa: F401
    font_,  # noqa: F401
    frame,  # noqa: F401
    grab,  # noqa: F401
    grid,  # noqa: F401
    image,  # noqa: F401
    label,  # noqa: F401
    labelframe,  # noqa: F401
    listbox,  # noqa: F401
    lower_,  # noqa: F401
    menu,  # noqa: F401
    menubutton,  # noqa: F401
    message,  # noqa: F401
    option_,  # noqa: F401
    pack,  # noqa: F401
    panedwindow,  # noqa: F401
    place,  # noqa: F401
    radiobutton,  # noqa: F401
    raise_,  # noqa: F401
    scale,  # noqa: F401
    scrollbar,  # noqa: F401
    selection,  # noqa: F401
    spinbox,  # noqa: F401
    text,  # noqa: F401
    tk_,  # noqa: F401
    tk_chooseColor,  # noqa: F401
    tk_chooseDirectory,  # noqa: F401
    tk_getOpenFile,  # noqa: F401
    tk_getSaveFile,  # noqa: F401
    tk_messageBox,  # noqa: F401
    tk_popup,  # noqa: F401
    toplevel,  # noqa: F401
    ttk_button,  # noqa: F401
    ttk_combobox,  # noqa: F401
    ttk_entry,  # noqa: F401
    ttk_frame,  # noqa: F401
    ttk_label,  # noqa: F401
    ttk_notebook,  # noqa: F401
    ttk_progressbar,  # noqa: F401
    ttk_scale,  # noqa: F401
    ttk_separator,  # noqa: F401
    ttk_sizegrip,  # noqa: F401
    ttk_style,  # noqa: F401
    ttk_treeview,  # noqa: F401
    winfo,  # noqa: F401
    wm,  # noqa: F401
)
from ._base import _REGISTRY


def tk_command_specs() -> tuple[CommandSpec, ...]:
    """Return Tk command specs from all registered classes.

    Tk commands set ``warn_missing_import=False`` because ``wish``
    auto-loads Tk — requiring an explicit ``package require Tk`` would
    generate noisy false positives.
    """
    from dataclasses import replace

    return tuple(replace(cls.spec(), warn_missing_import=False) for cls in _REGISTRY)
