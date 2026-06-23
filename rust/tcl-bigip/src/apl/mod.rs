//! F5 iApp APL (Application Presentation Language) — structural parser
//! and object model.

pub mod canonical;
pub mod iapp_diagnostics;
pub mod iapp_vars;
pub mod model;
pub mod parser;

pub use canonical::model_to_canonical;
pub use iapp_diagnostics::{validate_iapp_implementation, validate_iapp_presentation};
pub use iapp_vars::{IappVarRef, extract_iapp_var_refs};
pub use model::{
    AplField, AplInclude, AplModel, AplSection, AplTable, apl_name_to_tcl_var, tcl_var_to_apl_name,
};
pub use parser::parse_apl;
