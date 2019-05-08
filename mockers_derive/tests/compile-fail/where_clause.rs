use mockers_derive::mocked;

#[mocked]
trait A where Self : 'static {}

fn main() {}
