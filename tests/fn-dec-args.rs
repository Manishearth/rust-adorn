
#![feature(plugin, custom_attribute)]
#![plugin(adorn)]

#[adorn(bar(a = "", a = "a"))]
fn foo(a: &mut u8, b: &mut u8, (c, _): (u8, u8)) {
    assert!(c == 4);
    *a = c;
    *b = c;
}


fn bar<F>(f: F, str0: &'static str, str1: &'static str, a: &mut u8, b: &mut u8, (c, d): (u8, u8))
          where F: Fn(&mut u8, &mut u8, (u8, u8)) {
    assert!(c == 0 && d == 0);
    assert!(str0 == "");
    assert!(str1 == "a");
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