#[macro_export]
macro_rules! bounded_int_common {
    (@__doc_comment_wrapper $doc: expr, $($other: tt)*) => {
        #[doc = $doc]
        $($other)*
    };

    (@__safety_comment_concat_tail $sep: expr, $last_sep: expr) => {
        ""
    };

    (@__safety_comment_concat_tail $sep: expr, $last_sep: expr, $last: expr) => {
        concat!($last_sep, $last)
    };

    (@__safety_comment_concat_tail $sep: expr, $last_sep: expr, $next: expr $(, $others: expr)+) => {
        concat!($sep, $next, $crate::bounded_int_common!(
            @__safety_comment_concat_tail
            $sep, $last_sep $(, $others)*
        ))
    };

    (@__safety_comment_concat $sep: expr, $last_sep: expr, $next: expr $(, $others: expr)*) => {
        concat!($next, $crate::bounded_int_common!(
            @__safety_comment_concat_tail
            $sep, $last_sep $(, $others)*
        ))
    };

    (@__safety_comment_not $($not_value: expr),+) => {
        $crate::bounded_int_common!(
            @__safety_comment_concat ", ", " or " $(, concat!("`", stringify!($not_value), "`"))*
        )
    };

    (@__safety_comment_mask $mask_value: expr) => {
        concat!("`value & ", stringify!($mask_value), "` must be 0")
    };

    (@__safety_comment_and_mask_not $(, mask $mask_value: expr)?) => {
        concat!("", $(", and", $crate::bounded_int_common!(@__safety_comment_mask $mask_value))*)
    };

    (@__safety_comment_and_mask_not $(, mask $mask_value: expr)?, not [$($not_value: expr),+]) => {
        concat!(
            " and not ", $crate::bounded_int_common!(@__safety_comment_not $($not_value),*),
            $("; ", $crate::bounded_int_common!(@__safety_comment_mask $mask_value))*
        )
    };

    (@__safety_comment, mask $mask_value: expr) => {
        $crate::bounded_int_common!(@__safety_comment_mask $mask_value)
    };

    (
        @__safety_comment
        $(, mask $mask_value: expr)?, not [$($not_value: expr),+]
    ) => {
        concat!(
            "`value` must not be ",
            $crate::bounded_int_common!(@__safety_comment_not $($not_value),*),
            $(", and", $crate::bounded_int_common!(@__safety_comment_mask $mask_value),)*
        )
    };

    (
        @__safety_comment, min $min_value: expr
        $(, mask $mask_value: expr)? $(, not [$($not_value: expr),+])?
    ) => {
        concat!(
            "`value` must be greater than or equal to `", stringify!($min_value), "`",
            $crate::bounded_int_common!(
                @__safety_comment_and_mask_not $(, mask $mask_value)* $(, not [$($not_value),*])*
            ),
        )
    };

    (
        @__safety_comment, max $max_value: expr
        $(, mask $mask_value: expr)? $(, not [$($not_value: expr),+])?
    ) => {
        concat!(
            "`value` must be less than or equal to `", stringify!($max_value), "`",
            $crate::bounded_int_common!(
                @__safety_comment_and_mask_not $(, mask $mask_value)* $(, not [$($not_value),*])*
            ),
        )
    };

    (
        @__safety_comment, min $min_value: expr, max $max_value: expr
        $(, mask $mask_value: expr)? $(, not [$($not_value: expr),+])?
    ) => {
        concat!(
            "`value` must be in the `",
            stringify!($min_value), "..=", stringify!($max_value), "` range",
            $crate::bounded_int_common!(
                @__safety_comment_and_mask_not $(, mask $mask_value)* $(, not [$($not_value),*])*
            ),
        )
    };

    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty)
        $(, min $min_value: expr)?
        $(, max $max_value: expr)?
        $(, mask $mask_value: expr)?
        $(, not [$($not_value: expr),+])?
    ) => {
        $(#[$($attr)*])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        $vis struct $name($inner);

        #[allow(unused_comparisons)]
        impl $name {
            $crate::bounded_int_common!(
                @__doc_comment_wrapper
                concat!(
                    "# Safety\n",
                    $crate::bounded_int_common!(
                        @__safety_comment
                        $(, min $min_value)*
                        $(, max $max_value)*
                        $(, mask $mask_value)*
                        $(, not [$($not_value),*])*
                    ),
                    "."
                ),
                #[inline]
                pub const unsafe fn new_unchecked(value: $inner) -> Self {
                    $name(value)
                }
            );

            #[inline]
            #[allow(clippy::int_plus_one)]
            pub const fn new_checked(value: $inner) -> Option<Self> {
                if true
                    $(&& value >= $min_value)*
                    $(&& value <= $max_value)*
                    $(&& value & !$mask_value == 0)*
                    $($(&& value != $not_value)*)*
                {
                    Some(unsafe { Self::new_unchecked(value) })
                } else {
                    None
                }
            }

            #[inline]
            #[allow(clippy::int_plus_one)]
            pub const fn new(value: $inner) -> Self {
                assert!(true
                    $(&& value >= $min_value)*
                    $(&& value <= $max_value)*
                    $(&& value & !$mask_value == 0)*
                    $($(&& value != $not_value)*)*
                );
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
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty)
        $(, min $min_value: expr)?
        $(, max $max_value: expr)?
        $(, mask $mask_value: expr)?
        $(, not [$($not_value: expr),+])?
    ) => {
        $crate::bounded_int_common!(
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner)
            $(, min $min_value)*
            $(, max $max_value)*
            $(, mask $mask_value)*
            $(, not [$($not_value),*])*
        );

        #[allow(unused_comparisons)]
        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if false
                    $(|| self.0 < $min_value)*
                    $(|| self.0 > $max_value)*
                    $(|| self.0 & !$mask_value != 0)*
                    $($(|| self.0 == $not_value)*)*
                {
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
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty)
        $(, min $min_value: literal)?
        $(, max $max_value: literal)?
        $(, mask $mask_value: expr)?
        $(, not [$($not_value: expr),+])?
    ) => {
        $crate::bounded_int_common!(
            // $(#[rustc_layout_scalar_valid_range_start($min_value)])*
            // $(#[rustc_layout_scalar_valid_range_end($max_value)])*
            $(#[$(#[$($attr)*])*])* $vis struct $name($inner)
            $(, min $min_value)*
            $(, max $max_value)*
            $(, mask $mask_value)*
            $(, not [$($not_value),*])*
        );

        #[allow(unused_comparisons)]
        impl $name {
            #[inline]
            pub const fn get(self) -> $inner {
                if false
                    $(|| self.0 < $min_value)*
                    $(|| self.0 > $max_value)*
                    $(|| self.0 & !$mask_value != 0)*
                    $($(|| self.0 == $not_value)*)*
                {
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
            #[inline]
            fn steps_between(start: &Self, end: &Self) -> Option<usize> {
                end.get().checked_sub(start.get()).map(|v| v as usize)
            }

            #[inline]
            fn forward_checked(start: Self, count: usize) -> Option<Self> {
                (start.get() as usize).checked_add(count).and_then(|v| {
                    if v > $max_value as usize {
                        None
                    } else {
                        Some(unsafe { Self::new_unchecked(v as $inner) })
                    }
                })
            }

            #[inline]
            fn backward_checked(start: Self, count: usize) -> Option<Self> {
                (start.get() as usize).checked_sub(count).and_then(|v| {
                    if v < $min_value as usize {
                        None
                    } else {
                        Some(unsafe { Self::new_unchecked(v as $inner) })
                    }
                })
            }

            #[inline]
            fn forward(start: Self, count: usize) -> Self {
                unsafe {
                    Self::new_unchecked(
                        (start.get() as usize + count).min($max_value as usize) as $inner
                    )
                }
            }

            #[inline]
            fn backward(start: Self, count: usize) -> Self {
                unsafe {
                    Self::new_unchecked(
                        (start.get() as usize + count).max($min_value as usize) as $inner
                    )
                }
            }

            #[inline]
            unsafe fn forward_unchecked(start: Self, count: usize) -> Self {
                Self::new_unchecked(start.0.wrapping_add(count as $inner))
            }

            #[inline]
            unsafe fn backward_unchecked(start: Self, count: usize) -> Self {
                Self::new_unchecked(start.0.wrapping_sub(count as $inner))
            }
        }
    };
}

#[macro_export]
macro_rules! bounded_int_savestate {
    ($name: ident($inner: ty)) => {
        impl $crate::Loadable for $name {
            #[inline]
            fn load<S: $crate::ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
                save.load::<$inner>()
                    .and_then(|v| Self::new_checked(v).ok_or_else(|| S::invalid_enum()))
            }
        }
        
        impl $crate::LoadableInPlace for $name {
            #[inline]
            fn load_in_place<S: $crate::ReadSavestate>(
                &mut self,
                save: &mut S,
            ) -> Result<(), S::Error> {
                *self = save.load()?;
                Ok(())
            }
        }

        impl $crate::Storable for $name {
            #[inline]
            fn store<S: $crate::WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                save.store(&mut self.get())
            }
        }
    };
}
