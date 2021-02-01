extern crate adorn;
use adorn::{adorn_method, make_decorator_method};

struct Test {
    a: isize,
    b: isize
}

impl Test {
    #[adorn_method(bar)]
    fn foo(&mut self, a: isize, b: isize) -> Self {
        let mut c = |_self, __self| -> Self {
            self.a = _self;
            self.a = __self;
            Self {
                a,
                b
            }
        } ;
        c(3,2)
    }

    #[make_decorator_method(f)]
    fn bar(&mut self, a: isize, b: isize) -> Self {
        let mut retval = f(self, a, b);
        retval.a = 3;
        retval
    }

    #[adorn_method(bar1)]
    fn foo1(mut self) -> Self {
        self.a = 1;
        self
    }

    #[make_decorator_method(f)]
    fn bar1(mut self) -> Self {
        let mut retval = f(self);
        retval.b = 2;
        retval
    }
}

#[test]
fn test() {
    let mut t1 = Test {a: 0, b: 1};
    let t2 = t1.foo(5, 6);
    assert!(t1.a == 2 && t1.b == 1 && t2.a == 3 && t2.b == 6);

    let t3 = Test {a: 3, b: 4};
    let t4 = t3.foo1();
    assert!(t4.a == 1 && t4.b == 2);
}
