//! Tcl core command specifications — one file per command.

mod dict;
mod eval_;
mod expr_;
mod for_;
mod foreach_;
mod if_;
mod incr_;
mod proc_;
mod puts_;
mod set_;
mod while_;

use crate::spec::CommandSpec;

/// Return all Tcl core command specifications.
#[must_use]
pub fn tcl_command_specs() -> Vec<CommandSpec> {
    vec![
        dict::spec(),
        eval_::spec(),
        expr_::spec(),
        for_::spec(),
        foreach_::spec(),
        if_::spec(),
        incr_::spec(),
        proc_::spec(),
        puts_::spec(),
        set_::spec(),
        while_::spec(),
    ]
}
