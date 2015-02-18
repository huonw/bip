# `bip`

[![Build Status](https://travis-ci.org/huonw/bip.png)](https://travis-ci.org/huonw/bip)

bip (`Box` in place) provides a fully generic in-place `map` for
the `Box` type, taking care to be panic-safe and not leak memory.

Example:

```rust
let x: Box<i32> = Box::new(1);
let y: Box<f32> = bip::map_in_place(y, |x| x as f32 + 1.0);

// y uses the same allocation as x
```

[Documentation](http://huonw.github.io/bip/bip/)
