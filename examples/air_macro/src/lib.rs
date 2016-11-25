#![feature(plugin, custom_derive)]
#![plugin(mockers_macros)]

mod context;
#[cfg(test)] extern crate mockers;
#[cfg(test)] mod tests;
pub use context::AirConditioner;

pub fn set_temperature_20(cond: &mut AirConditioner) {
    let t = cond.get_temperature();
    if t < 20 {
        cond.make_hotter(20 - t);
    } else {
        cond.make_cooler(t - 20);
    }
}
