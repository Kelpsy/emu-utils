#[allow(clippy::missing_safety_doc)]
pub trait UnsafeFrom<T> {
    unsafe fn from(_: T) -> Self;
}

impl<T, U> UnsafeFrom<U> for T
where
    T: From<U>,
{
    #[inline]
    unsafe fn from(other: U) -> Self {
        Self::from(other)
    }
}

pub trait BitRange<T> {
    fn bit_range<const START: usize, const END: usize>(self) -> T;
    #[must_use]
    fn set_bit_range<const START: usize, const END: usize>(self, value: T) -> Self;
}

pub trait Bit {
    fn bit<const BIT: usize>(self) -> bool;
    #[must_use]
    fn set_bit<const BIT: usize>(self, value: bool) -> Self;
}

macro_rules! impl_bitrange {
    ($storage: ty, $value: ty) => {
        impl BitRange<$value> for $storage {
            #[inline]
            fn bit_range<const START: usize, const END: usize>(self) -> $value {
                const VALUE_BIT_LEN: usize = core::mem::size_of::<$value>() << 3;
                let selected = END - START;
                ((self >> START) as $value) << (VALUE_BIT_LEN - selected)
                    >> (VALUE_BIT_LEN - selected)
            }

            #[inline]
            fn set_bit_range<const START: usize, const END: usize>(self, value: $value) -> Self {
                const VALUE_BIT_LEN: usize = core::mem::size_of::<$value>() << 3;
                let selected = END - START;
                let mask = (if selected == VALUE_BIT_LEN {
                    <$value>::MAX
                } else {
                    ((1 as $value) << selected) - 1
                } as $storage)
                    << START;
                (self & !mask) | ((value as $storage) << START & mask)
            }
        }
    };
}

macro_rules! impl_bitrange_for_permutations {
    ((),($($bitrange_ty: ty),*)) => {};
    (($t: ty),($($bitrange_ty: ty),*)) => {
        $(
            impl_bitrange!($t, $bitrange_ty);
        )*
    };
    (($t_head: ty, $($t_rest: ty),*),($($bitrange_ty: ty),*)) => {
        impl_bitrange_for_permutations!(($t_head), ($($bitrange_ty),*));
        impl_bitrange_for_permutations!(($($t_rest),*), ($($bitrange_ty),*));
    };
}

impl_bitrange_for_permutations!(
    (u8, u16, u32, u64, u128, i8, i16, i32, i64, i128),
    (u8, u16, u32, u64, u128, i8, i16, i32, i64, i128)
);

macro_rules! impl_bit {
    ($t: ty) => {
        impl Bit for $t {
            #[inline(always)]
            fn bit<const BIT: usize>(self) -> bool {
                self & 1 << BIT != 0
            }

            #[inline(always)]
            #[must_use]
            fn set_bit<const BIT: usize>(self, value: bool) -> Self {
                (self & !(1 << BIT)) | (value as $t) << BIT
            }
        }
    };
}

macro_rules! impl_bit_for_types {
    ($($t: ty),*) => {
        $(impl_bit!($t);)*
    };
}

impl_bit_for_types!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
