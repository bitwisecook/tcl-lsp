"""Tk GUI diagnostic codes (TK-series). All internal."""

from core.common.codes import diag

TK1001 = diag("TK1001", "Tk widget command validation.", section="warning", internal=True)
TK1002 = diag("TK1002", "Tk geometry manager validation.", section="warning", internal=True)
TK1003 = diag("TK1003", "Tk event binding validation.", section="warning", internal=True)
