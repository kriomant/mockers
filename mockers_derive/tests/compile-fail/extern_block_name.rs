use mockers_derive::mocked;

#[mocked]
extern {
    fn foo();
}

fn main() {}
