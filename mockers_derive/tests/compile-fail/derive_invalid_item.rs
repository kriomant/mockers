use mockers_derive::mocked;

#[mocked(derive("Clone"))]
trait A {}

fn main() {}
