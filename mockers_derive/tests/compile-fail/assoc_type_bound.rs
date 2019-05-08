use mockers_derive::mocked;

#[mocked]
trait A {
    type Item : Sized;
}

fn main() {}
