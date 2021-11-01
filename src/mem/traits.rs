use core::{mem, ptr};
use std::alloc::{alloc_zeroed, handle_alloc_error, Layout};

/// # Safety
/// Any given byte pattern must be valid when interpreted as `Self`.
pub unsafe trait Fill8 {}

#[inline]
pub fn fill_8<T: Fill8 + ?Sized>(v: &mut T, fill_value: u8) {
    unsafe { ptr::write_bytes(v as *mut _ as *mut u8, fill_value, mem::size_of_val(v)) }
}

unsafe impl<T, const LEN: usize> Fill8 for [T; LEN] where T: Fill8 {}
unsafe impl<T> Fill8 for [T] where T: Fill8 {}

/// # Safety
/// A 0 byte pattern must be valid when interpreted as `Self`.
pub unsafe trait Zero {}

unsafe impl<T> Zero for *const T {}
unsafe impl<T> Zero for *mut T {}
unsafe impl<T, const LEN: usize> Zero for [T; LEN] where T: Zero {}
unsafe impl<T> Zero for [T] where T: Zero {}

#[inline]
pub fn zeroed_box<T: Zero>() -> Box<T> {
    unsafe {
        let layout = Layout::new::<T>();
        let ptr = alloc_zeroed(layout);
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        Box::from_raw(ptr.cast())
    }
}

#[inline]
pub fn zero<T: Zero>() -> T {
    unsafe { mem::MaybeUninit::zeroed().assume_init() }
}

#[inline]
pub fn make_zero<T: Zero + ?Sized>(v: &mut T) {
    unsafe { ptr::write_bytes(v as *mut _ as *mut u8, 0, mem::size_of_val(v)) }
}

pub trait MemValue: Sized + Copy + Zero + Fill8 {
    /// # Safety
    /// The given pointer must be [valid] for `Self` reads and point to a properly initialized value
    /// of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    #[must_use]
    unsafe fn read_le(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary, be [valid] for `Self` reads and
    /// point to a properly initialized value of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    #[must_use]
    unsafe fn read_le_aligned(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be [valid] for `Self` reads and point to a properly initialized value
    /// of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    #[must_use]
    unsafe fn read_be(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary, be [valid] for `Self` reads and
    /// point to a properly initialized value of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    #[must_use]
    unsafe fn read_be_aligned(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be [valid] for `Self` reads and point to a properly initialized value
    /// of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    #[must_use]
    unsafe fn read_ne(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary, be [valid] for `Self` reads and
    /// point to a properly initialized value of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    #[must_use]
    unsafe fn read_ne_aligned(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be [valid] for `Self` writes.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn write_le(self, ptr: *mut Self);
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary and be [valid] for `Self` writes.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn write_le_aligned(self, ptr: *mut Self);
    /// # Safety
    /// The given pointer must be [valid] for `Self` writes.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn write_be(self, ptr: *mut Self);
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary and be [valid] for `Self` writes.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn write_be_aligned(self, ptr: *mut Self);
    /// # Safety
    /// The given pointer must be [valid] for `Self` writes.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn write_ne(self, ptr: *mut Self);
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary and be [valid] for `Self` writes.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn write_ne_aligned(self, ptr: *mut Self);
}

mod impl_primitive {
    use super::{Fill8, MemValue, Zero};
    use core::{mem, ptr};

    macro_rules! impl_unsafe_trait {
        ($tr: ty; $($ty: ty),*) => {
            $(
                unsafe impl $tr for $ty {}
            )*
        };
    }

    impl_unsafe_trait!(Fill8; u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);
    impl_unsafe_trait!(Zero; u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize, bool, char, f32, f64);

    macro_rules! impl_mem_value {
        ($($ty: ty),*) => {
            $(
                impl MemValue for $ty {
                    #[inline]
                    unsafe fn read_le(ptr: *const Self) -> Self {
                        #[cfg(target_endian = "little")]
                        { ptr.read_unaligned() }
                        #[cfg(not(target_endian = "little"))]
                        Self::from_le_bytes((ptr as *const [u8; mem::size_of::<Self>()]).read())
                    }

                    #[inline]
                    unsafe fn read_le_aligned(ptr: *const Self) -> Self {
                        #[cfg(target_endian = "little")]
                        { ptr.read() }
                        #[cfg(not(target_endian = "little"))]
                        Self::from_le_bytes((ptr as *const [u8; mem::size_of::<Self>()]).read())
                    }

                    #[inline]
                    unsafe fn read_be(ptr: *const Self) -> Self {
                        #[cfg(target_endian = "big")]
                        { ptr.read_unaligned() }
                        #[cfg(not(target_endian = "big"))]
                        Self::from_be_bytes((ptr as *const [u8; mem::size_of::<Self>()]).read())
                    }

                    #[inline]
                    unsafe fn read_be_aligned(ptr: *const Self) -> Self {
                        #[cfg(target_endian = "big")]
                        { ptr.read() }
                        #[cfg(not(target_endian = "big"))]
                        Self::from_be_bytes((ptr as *const [u8; mem::size_of::<Self>()]).read())
                    }

                    #[inline]
                    unsafe fn read_ne(ptr: *const Self) -> Self {
                        ptr.read_unaligned()
                    }

                    #[inline]
                    unsafe fn read_ne_aligned(ptr: *const Self) -> Self {
                        ptr.read()
                    }

                    #[inline]
                    unsafe fn write_le(self, ptr: *mut Self) {
                        #[cfg(target_endian = "little")]
                        ptr.write_unaligned(self);
                        #[cfg(not(target_endian = "little"))]
                        (ptr as *mut [u8; mem::size_of::<Self>()]).write(self.to_le_bytes())
                    }

                    #[inline]
                    unsafe fn write_le_aligned(self, ptr: *mut Self) {
                        #[cfg(target_endian = "little")]
                        ptr.write(self);
                        #[cfg(not(target_endian = "little"))]
                        (ptr as *mut [u8; mem::size_of::<Self>()]).write(self.to_le_bytes())
                    }

                    #[inline]
                    unsafe fn write_be(self, ptr: *mut Self) {
                        #[cfg(target_endian = "big")]
                        ptr.write_unaligned(self);
                        #[cfg(not(target_endian = "big"))]
                        (ptr as *mut [u8; mem::size_of::<Self>()]).write(self.to_be_bytes())
                    }

                    #[inline]
                    unsafe fn write_be_aligned(self, ptr: *mut Self) {
                        #[cfg(target_endian = "big")]
                        ptr.write(self);
                        #[cfg(not(target_endian = "big"))]
                        (ptr as *mut [u8; mem::size_of::<Self>()]).write(self.to_be_bytes())
                    }

                    #[inline]
                    unsafe fn write_ne(self, ptr: *mut Self) {
                        ptr.write_unaligned(self);
                    }

                    #[inline]
                    unsafe fn write_ne_aligned(self, ptr: *mut Self) {
                        ptr.write(self);
                    }
                }
            )*
        };
    }

    impl_mem_value!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);

    macro_rules! read_arr {
        ($T: ty, $LEN: expr, $ptr: expr, $fn_ident: ident) => {{
            let mut result = mem::MaybeUninit::uninit();
            let mut result_ptr = result.as_mut_ptr() as *mut T;
            let mut ptr = $ptr as *const T;
            for _ in 0..$LEN {
                ptr::write(result_ptr, <$T>::$fn_ident(ptr));
                result_ptr = result_ptr.add(1);
                ptr = ptr.add(1);
            }
            result.assume_init()
        }};
    }

    macro_rules! write_arr {
        ($value: expr, $T: ty, $LEN: expr, $ptr: expr, $fn_ident: ident) => {{
            let mut ptr = $ptr as *mut T;
            for i in 0..$LEN {
                $value[i].$fn_ident(ptr);
                ptr = ptr.add(1);
            }
        }};
    }

    impl<T: MemValue, const LEN: usize> MemValue for [T; LEN] {
        #[inline]
        unsafe fn read_le(ptr: *const Self) -> Self {
            read_arr!(T, LEN, ptr, read_le)
        }
        #[inline]
        unsafe fn read_le_aligned(ptr: *const Self) -> Self {
            read_arr!(T, LEN, ptr, read_le_aligned)
        }
        #[inline]
        unsafe fn read_be(ptr: *const Self) -> Self {
            read_arr!(T, LEN, ptr, read_be)
        }
        #[inline]
        unsafe fn read_be_aligned(ptr: *const Self) -> Self {
            read_arr!(T, LEN, ptr, read_be_aligned)
        }
        #[inline]
        unsafe fn read_ne(ptr: *const Self) -> Self {
            read_arr!(T, LEN, ptr, read_ne)
        }
        #[inline]
        unsafe fn read_ne_aligned(ptr: *const Self) -> Self {
            read_arr!(T, LEN, ptr, read_ne_aligned)
        }
        #[inline]
        unsafe fn write_le(self, ptr: *mut Self) {
            write_arr!(self, T, LEN, ptr, write_le)
        }
        #[inline]
        unsafe fn write_le_aligned(self, ptr: *mut Self) {
            write_arr!(self, T, LEN, ptr, write_le_aligned)
        }
        #[inline]
        unsafe fn write_be(self, ptr: *mut Self) {
            write_arr!(self, T, LEN, ptr, write_be)
        }
        #[inline]
        unsafe fn write_be_aligned(self, ptr: *mut Self) {
            write_arr!(self, T, LEN, ptr, write_be_aligned)
        }
        #[inline]
        unsafe fn write_ne(self, ptr: *mut Self) {
            write_arr!(self, T, LEN, ptr, write_ne)
        }
        #[inline]
        unsafe fn write_ne_aligned(self, ptr: *mut Self) {
            write_arr!(self, T, LEN, ptr, write_ne_aligned)
        }
    }
}
