extern crate adorn;
use adorn::{adorn_static, make_decorator_static};

struct Test {
    a: isize,
    b: isize
}

impl Test {
    #[adorn_static(bar)]
    fn foo(a: isize, b: isize) -> Self {
        assert!(a == 1 && b == 0);
        Self {a, b}
    }

    #[make_decorator_static(f)]
    fn bar(a: isize, b: isize) -> Self {
        assert!(a == 1 && b == 1);
        let mut retval = f(a, 0);
        retval.a = 4;
        retval
    }
}

#[test]
fn test() {
    let t = Test::foo(1, 1);
    assert!(t.a == 4 && t.b == 0);
}
