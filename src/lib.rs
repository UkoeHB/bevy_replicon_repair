//documentation
#![allow(rustdoc::redundant_explicit_links)]
#![doc = include_str!("../README.md")]
#[allow(unused_imports)]
use crate as bevy_replicon_repair;

//module tree
mod app_ext;
mod client_plugin;
mod repair_rules;
mod retain;
mod server_plugin;

//API exports
pub use crate::app_ext::*;
pub use crate::client_plugin::*;
pub use crate::repair_rules::*;
pub use crate::retain::*;
pub use crate::server_plugin::*;
