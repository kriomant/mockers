#[cfg(test)] mod tests;

include!(concat!(env!("OUT_DIR"), "/types.rs"));

pub fn set_temperature_20(cond: &mut AirConditioner) {
    let t = cond.get_temperature();
    if t < 20 {
        cond.make_hotter(20 - t);
    } else {
        cond.make_cooler(t - 20);
    }
}
