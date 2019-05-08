use mockers_derive::mocked;

#[mocked]
trait A {
    extern "C" fn foo();
}

fn main() {}
