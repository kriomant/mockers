#![feature(plugin_registrar, rustc_private)]

extern crate rustc_plugin;
extern crate mockers_codegen;

use rustc_plugin::Registry;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    mockers_codegen::register(reg);
}
