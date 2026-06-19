//! `eda_mentor` command specifications.

mod add_list;
mod add_log;
mod add_wave;
mod bc;
mod bd;
mod be;
mod bl;
mod bp;
mod calibre;
mod calibre_drc;
mod calibre_lvs;
mod calibre_pex;
mod change;
mod coverage;
mod describe;
mod drivers;
mod examine;
mod find;
mod force;
mod formal_analyze;
mod formal_compile;
mod formal_verify;
mod init_signal_driver;
mod init_signal_spy;
mod onbreak;
mod qrun;
mod qverilog;
mod qvhdl;
mod qwave;
mod readers;
mod release;
mod restart;
mod resume;
mod run;
mod signal_force;
mod signal_release;
mod toggle;
mod transcript;
mod vcom;
mod vcover;
mod vdel;
mod virtual_;
mod vlib;
mod vlog;
mod vmap;
mod vopt;
mod vsim;
mod wave;
mod when;

use crate::spec::CommandSpec;

/// Return all `eda_mentor` command specifications.
#[must_use]
pub fn eda_mentor_command_specs() -> Vec<CommandSpec> {
    vec![
        add_list::spec(),
        add_log::spec(),
        add_wave::spec(),
        bc::spec(),
        bd::spec(),
        be::spec(),
        bl::spec(),
        bp::spec(),
        calibre::spec(),
        calibre_drc::spec(),
        calibre_lvs::spec(),
        calibre_pex::spec(),
        change::spec(),
        coverage::spec(),
        describe::spec(),
        drivers::spec(),
        examine::spec(),
        find::spec(),
        force::spec(),
        formal_analyze::spec(),
        formal_compile::spec(),
        formal_verify::spec(),
        init_signal_driver::spec(),
        init_signal_spy::spec(),
        onbreak::spec(),
        qrun::spec(),
        qverilog::spec(),
        qvhdl::spec(),
        qwave::spec(),
        readers::spec(),
        release::spec(),
        restart::spec(),
        resume::spec(),
        run::spec(),
        signal_force::spec(),
        signal_release::spec(),
        toggle::spec(),
        transcript::spec(),
        vcom::spec(),
        vcover::spec(),
        vdel::spec(),
        virtual_::spec(),
        vlib::spec(),
        vlog::spec(),
        vmap::spec(),
        vopt::spec(),
        vsim::spec(),
        wave::spec(),
        when::spec(),
    ]
}
