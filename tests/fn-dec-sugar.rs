extern crate adorn;

use adorn::{adorn, make_decorator};

#[adorn(bar)]
fn foo(a: &mut u8, b: &mut u8, (c, _): (u8, u8)) {
    assert!(c == 4);
    *a = c;
    *b = c;
}

#[make_decorator(f)]
fn bar(a: &mut u8, b: &mut u8, (c, d): (u8, u8)) {
    assert!(c == 0 && d == 0);
    f(a, b, (4, 0));
    *b = 100;
}

#[test]
fn test() {
    let mut x = 0;
    let mut y = 1;
    foo(&mut x, &mut y, (0, 0));
    assert!(x == 4 && y == 100);
}
