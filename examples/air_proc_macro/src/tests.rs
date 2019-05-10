use super::{set_temperature_20, AirConditioner};
use mockers::Scenario;

#[test]
fn test_set_temperature_20() {
    let scenario = Scenario::new();
    let (mut cond, _) = scenario.create_mock_for::<AirConditioner>();

    scenario.expect(cond.get_temperature_call().and_return(16));
    scenario.expect(cond.make_hotter_call(4).and_return(()));

    set_temperature_20(&mut cond);
}
