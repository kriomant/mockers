#[derive(Mock)]
pub trait AirConditioner {
    fn make_hotter(&mut self, by: i16);
    fn make_cooler(&mut self, by: i16);
    fn get_temperature(&self) -> i16;
}
