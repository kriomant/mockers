use mockers::Scenario;
use mockers_derive::mocked;

pub struct NonClonable;

#[mocked]
pub trait A {
    fn create0(&self) -> NonClonable;
    fn create1(&self, a0: ()) -> NonClonable;
    fn create2(&self, a0: (), a1: ()) -> NonClonable;
    fn create3(&self, a0: (), a1: (), a2: ()) -> NonClonable;
    fn create4(&self, a0: (), a1: (), a2: (), a3: ()) -> NonClonable;
}

#[test]
fn test_fn_mut() {
    let scenario = Scenario::new();
    let (mock, handle) = scenario.create_mock_for::<dyn A>();

    scenario.expect(handle.create0().and_call_clone(|| NonClonable).times(1));
    scenario.expect(
        handle.create1(())
              .and_call_clone(|_| NonClonable)
              .times(1),
    );
    scenario.expect(
        handle.create2((), ())
              .and_call_clone(|_, _| NonClonable)
              .times(1),
    );
    scenario.expect(
        handle.create3((), (), ())
              .and_call_clone(|_, _, _| NonClonable)
              .times(1),
    );
    scenario.expect(
        handle.create4((), (), (), ())
              .and_call_clone(|_, _, _, _| NonClonable)
              .times(1),
    );

    let _ = mock.create0();
    let _ = mock.create1(());
    let _ = mock.create2((), ());
    let _ = mock.create3((), (), ());
    let _ = mock.create4((), (), (), ());
}
