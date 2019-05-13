use super::{set_temperature_20, AirConditioner};
use mockers::Scenario;

#[test]
fn test_set_temperature_20() {
    let scenario = Scenario::new();
    let (mut cond, handle) = scenario.create_mock_for::<AirConditioner>();

    scenario.expect(handle.get_temperature().and_return(16));
    scenario.expect(handle.make_hotter(4).and_return(()));

    set_temperature_20(&mut cond);
}
