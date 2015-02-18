#![feature(unsafe_destructor, alloc)]
#![cfg_attr(test, feature(std_misc))]

//! bip (`Box` in place) provides a fully generic in-place `map` for
//! the `Box` type, taking care to be panic-safe and not leak memory.
//!
//! [Available on
//! crates.io](http://crates.io/crates/bip). [Source](https://github.com/huonw/bip).

use std::rt::heap;
use std::{mem,ptr};

// avoid memory leaks by freeing the memory, without
// running the destructor of the contents
struct Dropper<T> {
    ptr: *mut T
}

#[unsafe_destructor]
impl<T> Drop for Dropper<T> {
    fn drop(&mut self) {
        unsafe {
            heap::deallocate(self.ptr as *mut u8, mem::size_of::<T>(), mem::align_of::<T>());
        }
    }
}

/// Execute `f` on the data in `x`, replacing the output into the same
/// allocation.
///
/// This is semantically equivalent to `Box::new(f(*x))`, but avoids
/// the allocation by reusing the memory of `x` directly. `map` will
/// not cause unsafety or leak memory if `f` panics.
///
/// `T` and `U` must have the same size, and the alignment (measured
/// by `std::mem::min_align_of`) of `T` must be at least as large as
/// that of `U`. A violation of either of these requirements will
/// result in a runtime panic.
///
/// # Example
///
/// ```rust
/// let x = Box::new(1_i32);
/// let address = &*x as *const _ as usize;
///
/// let new_x = bip::map_in_place(x, |a| a as f32 + 1.0);
/// assert_eq!(*new_x, 2.0);
///
/// assert_eq!(address, &*new_x as *const _ as usize);
/// ```
pub fn map_in_place<T, U, F>(x: Box<T>, f: F) -> Box<U> where F: FnOnce(T) -> U {
    assert!(mem::size_of::<T>() == mem::size_of::<U>(),
            "map_in_place: `T` and `U` are of different sizes");
    assert!(mem::align_of::<T>() >= mem::align_of::<U>(),
            "map_in_place: alignment of `U` is too large");

    unsafe {
        let dropper = Dropper {
            ptr: mem::transmute(x)
        };
        let old = ptr::read(dropper.ptr);
        let new = f(old);
        ptr::write(dropper.ptr as *mut U, new);

        let ret: Box<U> = mem::transmute(dropper.ptr);

        mem::forget(dropper);
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::map_in_place;
    use std::thread::Thread;
    use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

    #[derive(PartialEq, Debug)]
    struct NotCopy(i32);
    #[derive(PartialEq, Debug)]
    struct NotCopy2(f32);

    #[test]
    fn smoke() {
        assert_eq!(map_in_place(Box::new(NotCopy(1)), |x| NotCopy2(x.0 as f32 + 1.0)),
                   Box::new(NotCopy2(2.0)));
    }

    #[test]
    fn in_place() {
        let x = Box::new(NotCopy(1));
        let address_x = &*x as *const _;
        let y = map_in_place(x, |x| NotCopy(x.0 + 1));
        let address_y = &*y as *const _;

        assert_eq!(address_x, address_y);
    }

    #[test]
    fn destructor_count() {
        static COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

        struct Foo { _x: u8 }
        impl Drop for Foo {
            fn drop(&mut self) {
                COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }
        struct Bar { _x: u8 }
        impl Drop for Bar {
            fn drop(&mut self) {
                COUNT.fetch_add(10_000, Ordering::SeqCst);
            }
        }

        COUNT.store(0, Ordering::SeqCst);
        let value = Box::new(Foo { _x: 1 });

        let _ = Thread::scoped(|| {
            map_in_place(value, |_| -> Bar { panic!() });
        }).join();
        assert_eq!(COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[should_fail]
    fn mismatching_sizes() {
        map_in_place(Box::new(1i32), |_| 0i16);
    }
    #[test]
    #[should_fail]
    fn insufficient_alignment() {
        map_in_place(Box::new([0u8; 8]), |_| 0u64);
    }
}
