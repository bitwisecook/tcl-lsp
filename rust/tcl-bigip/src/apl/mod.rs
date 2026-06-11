//! F5 iApp APL (Application Presentation Language) — structural parser
//! and object model. Rust port of `dialects/f5/bigip/apl_model.py`.

pub mod canonical;
pub mod model;
pub mod parser;

pub use canonical::model_to_canonical;
pub use model::{
    apl_name_to_tcl_var, tcl_var_to_apl_name, AplField, AplInclude, AplModel, AplSection, AplTable,
};
pub use parser::parse_apl;
