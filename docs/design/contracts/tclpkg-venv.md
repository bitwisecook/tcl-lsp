# tclpkg virtual environments

## Symptom

``tclsh`` inside a venv does not find installed packages, or activation
scripts fail in a particular shell.

## Decision rules / contracts

1. ``tcl venv create .venv`` produces ``bin/``, ``lib/``, ``tclvenv.cfg``.
2. ``bin/tclsh`` is a POSIX shell wrapper that always sets ``TCLLIBPATH``
   before exec-ing the pinned tclsh — non-interactive use works without
   manual activation.
3. Activation scripts for bash/zsh (``bin/activate``) and fish
   (``bin/activate.fish``) set ``TCLLIBPATH``, ``PATH``, ``TCL_VENV``,
   and ``PS1``.
4. ``deactivate`` is a function/alias that restores the saved environment.
5. ``tclvenv.cfg`` records ``tcl_version``, ``tcl_executable``, ``prompt``,
   ``include-system-site-packages``, and ``project_root``.
6. ``tcl venv update --tcl VERSION`` rewrites the wrapper and config without
   touching ``lib/``.
7. ``tcl venv delete`` refuses to delete the currently-active venv unless
   ``--force`` is given.
8. ``tcl venv run .venv -- <command>`` sets up the env and execs the command
   without manual activation.
9. ``tclsh`` discovery reuses ``core.tcl_discovery.find_tclsh()``.

## File-path anchors

- ``tooling/tclpkg/venv.py`` — ``create_venv()``, ``delete_venv()``, ``read_venv_config()``
- ``tooling/cli/verbs/venv.py`` — CLI handlers for all venv verbs
- ``shared/tcl_discovery.py:26`` — ``find_tclsh()``
