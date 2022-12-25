//! _bindgen_-generated definitions of the plugin APIs.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(deref_nullptr)]
#![allow(clippy::transmute_int_to_bool)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

include!(concat!(env!("OUT_DIR"), "/mirabel.rs"));

// Manually implement blocked variables.
pub const LS_ERR: usize = usize::MAX;
pub const MOVE_NONE: move_code = move_code::MAX;
