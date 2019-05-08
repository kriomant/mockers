use mockers_derive::mocked;

#[mocked]
trait A {
    unsafe fn foo();
}

fn main() {}
