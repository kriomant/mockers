use mockers_derive::mocked;

trait A {}

#[mocked]
trait B : A {}

fn main() {}
