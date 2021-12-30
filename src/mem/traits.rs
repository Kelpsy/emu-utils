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

mod sealed {
    pub trait MemValue {}

    macro_rules! impl_mem_value {
        ($($ty: ty),*) => {
            $(
                impl MemValue for $ty {}
            )*
        };
    }

    impl_mem_value!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);
}

pub trait MemValue: Sized + Copy + Zero + Fill8 {
    fn from_le_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self;
    fn from_be_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self;
    fn from_ne_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self;

    fn to_le_bytes(self) -> [u8; mem::size_of::<Self>()];
    fn to_be_bytes(self) -> [u8; mem::size_of::<Self>()];
    fn to_ne_bytes(self) -> [u8; mem::size_of::<Self>()];

    /// # Safety
    /// The given pointer must be [valid] for `Self` reads and point to a properly initialized value
    /// of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn read_le(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary, be [valid] for `Self` reads and
    /// point to a properly initialized value of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn read_le_aligned(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be [valid] for `Self` reads and point to a properly initialized value
    /// of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn read_be(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary, be [valid] for `Self` reads and
    /// point to a properly initialized value of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn read_be_aligned(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be [valid] for `Self` reads and point to a properly initialized value
    /// of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
    unsafe fn read_ne(ptr: *const Self) -> Self;
    /// # Safety
    /// The given pointer must be aligned to a `Self` boundary, be [valid] for `Self` reads and
    /// point to a properly initialized value of `Self`.
    ///
    /// [valid]: https://doc.rust-lang.org/stable/std/ptr/index.html#safety
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
    use core::mem;

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
                    fn from_le_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self {
                        <$ty>::from_le_bytes(bytes)
                    }

                    #[inline]
                    fn from_be_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self {
                        <$ty>::from_be_bytes(bytes)
                    }

                    #[inline]
                    fn from_ne_bytes(bytes: [u8; mem::size_of::<Self>()]) -> Self {
                        <$ty>::from_ne_bytes(bytes)
                    }

                    #[inline]
                    fn to_le_bytes(self) -> [u8; mem::size_of::<Self>()] {
                        <$ty>::to_le_bytes(self)
                    }

                    #[inline]
                    fn to_be_bytes(self) -> [u8; mem::size_of::<Self>()] {
                        <$ty>::to_be_bytes(self)
                    }

                    #[inline]
                    fn to_ne_bytes(self) -> [u8; mem::size_of::<Self>()] {
                        <$ty>::to_ne_bytes(self)
                    }

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
}
