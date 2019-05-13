use mockers_derive::mocked;

#[mocked(derive(Foo))]
trait A {}

fn main() {}
