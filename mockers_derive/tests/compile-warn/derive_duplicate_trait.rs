use mockers_derive::mocked;

#[mocked(derive(Clone, Clone))]
trait A {}

fn main() {}

