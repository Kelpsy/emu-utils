#[macro_export]
macro_rules! bounded_int_common {
    (@__doc_comment_wrapper $doc: expr, $($other: tt)*) => {
        #[doc = $doc]
        $($other)*
    };
    (@__doc_comment_not_tail) => {
        "`"
    };
    (@__doc_comment_not_tail not $not_value: expr) => {
        concat!("` or `", stringify!($not_value), "`")
    };
    (@__doc_comment_not_tail not $not_value_a: expr$(, not $not_value_b: expr),+) => {
        concat!(
            "`, `",
            stringify!($not_value_a),
            $crate::bounded_int_common!(@__doc_comment_not_tail $(not $not_value_b),*),
        )
    };
    (@__doc_comment_not) => {
        ""
    };
    (@__doc_comment_not not $not_value_first: expr $(, not $not_value: expr)*) => {
        concat!(
            " and not `",
            stringify!($not_value_first),
            $crate::bounded_int_common!(@__doc_comment_not_tail $(not $not_value),*),
        )
    };
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        min $min_value: expr,
        max $max_value: expr
        $(, not $not_value: expr)*$(,)?
    ) => {
        $(#[$($attr)*])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        $vis struct $name($inner);

        impl $name {
            $crate::bounded_int_common!(
                @__doc_comment_wrapper
                concat!(
                    "# Safety\n`value` must be in the `",
                    stringify!($min_value),
                    "..=",
                    stringify!($max_value),
                    "` range",
                    $crate::bounded_int_common!(@__doc_comment_not $(not $not_value),*),
                    ".",
                ),
                #[inline]
                pub const unsafe fn new_unchecked(value: $inner) -> Self {
                    $name(value)
                }
            );

            #[inline]
            #[allow(clippy::int_plus_one)]
            pub const fn new_checked(value: $inner) -> Option<Self> {
                if value >= $min_value && value <= $max_value $(&& value != $not_value)* {
                    Some(unsafe { Self::new_unchecked(value) })
                } else {
                    None
                }
            }

            #[inline]
            #[allow(clippy::int_plus_one)]
            pub const fn new(value: $inner) -> Self {
                assert!(value >= $min_value && value <= $max_value $(&& value != $not_value)*);
                unsafe { Self::new_unchecked(value) }
            }
        }

        impl From<$inner> for $name {
            #[inline]
            fn from(other: $inner) -> Self {
                Self::new(other)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(other: $name) -> Self {
                other.get()
            }
        }
    };
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        min $min_value: expr
        $(, not $not_value: expr)*$(,)?
    ) => {
        $(#[$($attr)*])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        $vis struct $name($inner);

        impl $name {
            $crate::bounded_int_common!(
                @__doc_comment_wrapper
                concat!(
                    "# Safety\n`value` must be greater than or equal to `",
                    stringify!($min_value),
                    "`",
                    $crate::bounded_int_common!(@__doc_comment_not $(not $not_value),*),
                    ".",
                ),
                #[inline]
                pub const unsafe fn new_unchecked(value: $inner) -> Self {
                    $name(value)
                }
            );

            #[inline]
            pub const fn new_checked(value: $inner) -> Option<Self> {
                if value >= $min_value $(&& value != $not_value)* {
                    Some(unsafe { Self::new_unchecked(value) })
                } else {
                    None
                }
            }

            #[inline]
            pub const fn new(value: $inner) -> Self {
                assert!(value >= $min_value $(&& value != $not_value)*);
                unsafe { Self::new_unchecked(value) }
            }
        }

        impl From<$inner> for $name {
            #[inline]
            fn from(other: $inner) -> Self {
                Self::new(other)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(other: $name) -> Self {
                other.get()
            }
        }
    };
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        max $max_value: expr
        $(, not $not_value: expr)*$(,)?
    ) => {
        $(#[$($attr)*])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        $vis struct $name($inner);

        impl $name {
            $crate::bounded_int_common!(
                @__doc_comment_wrapper
                concat!(
                    "# Safety\n`value` must be less than or equal to `",
                    stringify!($max_value),
                    "`",
                    $crate::bounded_int_common!(@__doc_comment_not $(not $not_value),*),
                    ".",
                ),
                #[inline]
                pub const unsafe fn new_unchecked(value: $inner) -> Self {
                    $name(value)
                }
            );

            #[inline]
            #[allow(clippy::int_plus_one)]
            pub const fn new_checked(value: $inner) -> Option<Self> {
                if value <= $max_value $(&& value != $not_value)* {
                    Some(unsafe { Self::new_unchecked(value) })
                } else {
                    None
                }
            }

            #[inline]
            #[allow(clippy::int_plus_one)]
            pub const fn new(value: $inner) -> Self {
                assert!(value <= $max_value $(&& value != $not_value)*);
                unsafe { Self::new_unchecked(value) }
            }
        }

        impl From<$inner> for $name {
            #[inline]
            fn from(other: $inner) -> Self {
                Self::new(other)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(other: $name) -> Self {
                other.get()
            }
        }
    };
}

#[macro_export]
macro_rules! bounded_int {
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        min $min_value: expr,
        max $max_value: expr
        $(, not $not_value: expr)*$(,)?
    ) => {
        $crate::bounded_int_common!(
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner),
            min $min_value,
            max $max_value
            $(, not $not_value)*
        );

        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if self.0 < $min_value || self.0 > $max_value $(|| self.0 == $not_value)* {
                    unsafe { core::hint::unreachable_unchecked() }
                }
                self.0
            }
        }
    };
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        min $min_value: literal
        $(, not $not_value: expr)*$(,)?
    ) => {
        $crate::bounded_int_common!(
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner),
            min $min_value
            $(, not $not_value)*
        );

        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if self.0 < $min_value $(|| self.0 == $not_value)* {
                    unsafe { core::hint::unreachable_unchecked() }
                }
                self.0
            }
        }
    };
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        max $max_value: expr
        $(, not $not_value: expr)*$(,)?
    ) => {
        $crate::bounded_int_common!(
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner),
            max $max_value
            $(, not $not_value)*
        );

        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if self.0 > $max_value $(|| self.0 == $not_value)* {
                    unsafe { core::hint::unreachable_unchecked() }
                }
                self.0
            }
        }
    };
}

// TODO: `#![feature(rustc_attrs)]` seems to be very unstable at the moment, maybe re-enable the
// scalar valid range attributes once that's no longer the case.

#[macro_export]
macro_rules! bounded_int_lit {
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        min $min_value: literal,
        max $max_value: literal
        $(, not $not_value: expr)*$(,)?
    ) => {
        $crate::bounded_int_common!(
            // #[rustc_layout_scalar_valid_range_start($min_value)]
            // #[rustc_layout_scalar_valid_range_end($max_value)]
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner),
            min $min_value,
            max $max_value
            $(, not $not_value)*
        );

        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if self.0 < $min_value || self.0 > $max_value $(|| self.0 == $not_value)* {
                    unsafe { core::hint::unreachable_unchecked() }
                }
                self.0
            }
        }
    };
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        min $min_value: literal
        $(, not $not_value: expr)*$(,)?
    ) => {
        $crate::bounded_int_common!(
            // #[rustc_layout_scalar_valid_range_start($min_value)]
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner),
            min $min_value
            $(, not $not_value)*
        );

        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if self.0 < $min_value $(|| self.0 == $not_value)* {
                    unsafe { core::hint::unreachable_unchecked() }
                }
                self.0
            }
        }
    };
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty),
        max $max_value: literal
        $(, not $not_value: expr)*$(,)?
    ) => {
        $crate::bounded_int_common!(
            // #[rustc_layout_scalar_valid_range_end($max_value)]
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner),
            max $max_value
            $(, not $not_value)*
        );

        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if self.0 > $max_value $(|| self.0 == $not_value)* {
                    unsafe { core::hint::unreachable_unchecked() }
                }
                self.0
            }
        }
    };
}

#[macro_export]
macro_rules! bounded_int_step {
    ($name: ident($inner: ty), min $min_value: expr, max $max_value: expr) => {
        impl core::iter::Step for $name {
            fn steps_between(start: &Self, end: &Self) -> Option<usize> {
                end.get().checked_sub(start.get()).map(|v| v as usize)
            }

            fn forward_checked(start: Self, count: usize) -> Option<Self> {
                (start.get() as usize).checked_add(count).and_then(|v| {
                    if v > $max_value as usize {
                        None
                    } else {
                        Some(unsafe { Self::new_unchecked(v as $inner) })
                    }
                })
            }

            fn backward_checked(start: Self, count: usize) -> Option<Self> {
                (start.get() as usize).checked_sub(count).and_then(|v| {
                    if v < $min_value as usize {
                        None
                    } else {
                        Some(unsafe { Self::new_unchecked(v as $inner) })
                    }
                })
            }

            fn forward(start: Self, count: usize) -> Self {
                unsafe {
                    Self::new_unchecked(
                        (start.get() as usize + count).min($max_value as usize) as $inner
                    )
                }
            }

            fn backward(start: Self, count: usize) -> Self {
                unsafe {
                    Self::new_unchecked(
                        (start.get() as usize + count).max($min_value as usize) as $inner
                    )
                }
            }

            unsafe fn forward_unchecked(start: Self, count: usize) -> Self {
                Self::new_unchecked(start.0.wrapping_add(count as $inner))
            }

            unsafe fn backward_unchecked(start: Self, count: usize) -> Self {
                Self::new_unchecked(start.0.wrapping_sub(count as $inner))
            }
        }
    };
}
