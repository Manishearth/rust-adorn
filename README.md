# rust-adorn

[![Build Status](https://travis-ci.org/Manishearth/rust-adorn.svg)](https://travis-ci.org/Manishearth/rust-adorn)

Python-style function decorators for Rust

## Decorate functions

Example usage:

```rust
use adorn::{adorn, make_decorator};

#[adorn(bar)]
fn foo(a: &mut u8, b: &mut u8, (c, _): (u8, u8)) {
    assert!(c == 4);
    *a = c;
    *b = c;
}


fn bar<F>(f: F, a: &mut u8, b: &mut u8, (c, d): (u8, u8)) where F: Fn(&mut u8, &mut u8, (u8, u8)) {
    assert!(c == 0 && d == 0);
    f(a, b, (4, 0));
    *b = 100;
}

fn main() {
    let mut x = 0;
    let mut y = 1;
    foo(&mut x, &mut y, (0, 0));
    assert!(x == 4 && y == 100);
}
```

In this case, `foo` will become:

```rust
fn foo(a: &mut u8, b: &mut u8, (c, d): (u8, u8)) {
    fn foo_inner(a: &mut u8, b: &mut u8, (c, _): (u8, u8)) {
        assert!(c == 4);
        *a = c;
        *b = c;
    }
    bar(foo_inner, a, b, (c, d))
}
```

In other words, calling `foo()` will actually call `bar()` wrapped around `foo()`.


There is a `#[make_decorator]` attribute to act as sugar for creating decorators. For example,

```rust
#[make_decorator(f)]
fn bar(a: &mut u8, b: &mut u8, (c, d): (u8, u8)) {
    assert!(c == 0 && d == 0);
    f(a, b, (4, 0)); // `f` was declared in the `make_decorator` annotation
    *b = 100;
}

```

desugars to 

```rust
fn bar<F>(f: F, a: &mut u8, b: &mut u8, (c, d): (u8, u8)) where F: Fn(&mut u8, &mut u8, (u8, u8)) {
    assert!(c == 0 && d == 0);
    f(a, b, (4, 0));
    *b = 100;
}
```

## Decorate nonstatic methods

Example usage:

```rust
use adorn::{adorn_method, make_decorator_method};

pub struct Test {
    a: u8,
    b: u8
}

impl Test {
    #[adorn_method(bar)]
    fn foo(&mut self, a: u8, b: u8) {
        assert!(a == 0 && b == 0);
        self.a = a;
        self.b = b;
    }
    
    fn bar<F>(&mut self, f: F, a: u8, b: u8) where F: Fn(Self, u8, u8) {
        assert!(a == 0 && b == 0);
        f(self, a, b);
        self.b = 100;
    }
}

fn main() {
    let mut t = Test {
        a: 1,
        b: 1,
    };
    t.foo(0, 0);
    assert!(t.a == 0 && t.b == 100);
}
```

In this case, `foo` will become:

```rust
impl Test {
    fn foo(&mut self, a: u8, b: u8) {
        let foo_inner = |s: &mut Self, a: u8, b: u8| {
            assert!(a == 0 && b == 0);
            s.a = a;
            s.b = b;
        };
        self.bar(foo_inner, a, b, (c, d))
    }
}
```

Similarly, a `#[make_decorator_method]` attribute is provided to create decorators. For example,

```rust
impl Test {
    #[make_decorator_method(f)]
    fn bar(&mut self, a: u8, b: u8) {
        assert!(a == 0 && b == 0);
        f(self, a, b); // `f` was declared in the `make_decorator_method` annotation
        self.b = 100;
    }
}
```

desugars to

```rust
impl Test{
    fn bar<F>(&mut self, f: F, a: u8, b: u8) where F: Fn(Self, u8, u8) {
        assert!(a == 0 && b == 0);
        f(self, a, b);
        self.b = 100;
    }
}
```

## Decorate static methods

Use `#[make_decorator_static]` and `#[adorn_static]` to make a static decorator and then use it to decorate a static method, for example

```rust
use adorn::{adorn_method, make_decorator_method};

pub struct Test {
    a: u8,
    b: u8
}

impl Test {
    #[adorn_static(bar)]
    fn foo(a: u8, b: u8) -> Self {
        assert!(a == 0 && b == 0);
        Self {
            a,
            b
        }
    }

    #[make_decorator_static(f)]
    fn bar(a: u8, b: u8) -> Self {
        assert!(a == 0 && b == 0);
        let mut retval = f(a, b);
        retval.b = 100;
        retval
    }
}

fn main() {
    let t = Test::foo(0, 0);
    assert!(t.a == 0 && t.b == 100);
}
```

The two static methods desugar to

```rust
impl Test {
    fn foo(a: u8, b: u8) -> Self {
        let foo_inner = |a: u8, b: u8| -> Self {
            assert!(a == 0 && b == 0);
            Self {
                a,
                b
            }
        };
        Self::bar(foo, a, b)
    }

    fn bar(f: F, a: u8, b: u8) -> Self where F: Fn(u8, u8) -> Self {
        assert!(a == 0 && b == 0);
        let mut retval = f(a, b);
        retval.b = 100;
        retval
    }
}
```
