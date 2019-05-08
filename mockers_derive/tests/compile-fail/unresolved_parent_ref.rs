use mockers_derive::mocked;

trait A {}

#[mocked(refs = "A => ::A")]
trait B : A {}

fn main() {}
