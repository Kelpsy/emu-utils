use crate::{Bytes, MemValue, OwnedBytesCellPtr};
use core::{
    cell::Cell,
    convert::Infallible,
    mem::{size_of, MaybeUninit},
    ptr,
    simd::{LaneCount, Simd, SimdElement, SupportedLaneCount},
};

pub trait LoadableInPlace {
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error>;
}

pub trait Loadable: Sized {
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error>;
}

pub trait ReadSavestate: Sized {
    type Error;

    const TRANSIENT: bool;

    fn load_raw<T: MemValue>(&mut self) -> Result<T, Self::Error>;
    fn load_bytes(&mut self, len: usize) -> Result<*const u8, Self::Error>;

    fn invalid_enum() -> Self::Error;

    fn start_struct(&mut self) -> Result<(), Self::Error>;
    fn end_struct(&mut self) -> Result<(), Self::Error>;
    fn start_field(&mut self, ident: &[u8]) -> Result<(), Self::Error>;

    #[inline]
    fn load<T: Loadable>(&mut self) -> Result<T, Self::Error> {
        T::load(self)
    }

    #[inline]
    fn load_into<T: LoadableInPlace>(&mut self, value: &mut T) -> Result<(), Self::Error> {
        value.load_in_place(self)
    }
}

// Used for fast, unchecked in-memory savestates (i.e. rewinding).
pub struct TransientReadSavestate<'a> {
    save: &'a [u8],
    pos: u32,
}

impl<'a> TransientReadSavestate<'a> {
    /// # Safety
    /// The given save's length must be less than `0x1_0000_0000` bytes, and all subsequent reads
    /// must not go out of bounds.
    pub unsafe fn new(save: &'a [u8]) -> Self {
        TransientReadSavestate { save, pos: 0 }
    }
}

impl<'a> ReadSavestate for TransientReadSavestate<'a> {
    type Error = Infallible;

    const TRANSIENT: bool = true;

    fn invalid_enum() -> Self::Error {
        unreachable!();
    }

    #[inline]
    fn load_raw<T: MemValue>(&mut self) -> Result<T, Self::Error> {
        let start = self.pos as usize;
        self.pos = (start + size_of::<T>()) as u32;
        Ok(unsafe { T::read_ne(self.save.as_ptr().add(start) as *const T) })
    }

    #[inline]
    fn load_bytes(&mut self, len: usize) -> Result<*const u8, Self::Error> {
        let start = self.pos as usize;
        self.pos = (start + len) as u32;
        Ok(unsafe { self.save.as_ptr().add(start) })
    }

    #[inline]
    fn start_struct(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn end_struct(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn start_field(&mut self, _ident: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct StructInfo<'a> {
    fields: Vec<(&'a [u8], u32)>,
    end: u32,
    cur_field: u8, // Used to speed up lookup, assuming a linear field order
}

// Used for checked savestates that will be saved to disk, and need compatibility across field order
// changes, additions and deletions.
pub struct PersistentReadSavestate<'a> {
    save: &'a [u8],
    pos: u32,
    structs: Vec<StructInfo<'a>>,
}

impl<'a> PersistentReadSavestate<'a> {
    pub fn new(save: &'a [u8]) -> Result<Self, ()> {
        if save.len() > u32::MAX as usize {
            return Err(());
        }
        Ok(PersistentReadSavestate {
            save,
            pos: 0,
            structs: Vec::new(),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ReadError {
    FieldNotFound,
    UnexpectedEof,
    NoStructPresent,
    InvalidEnum,
}

impl<'a> ReadSavestate for PersistentReadSavestate<'a> {
    type Error = ReadError;

    const TRANSIENT: bool = false;

    fn invalid_enum() -> Self::Error {
        ReadError::InvalidEnum
    }

    #[inline]
    fn load_raw<T: MemValue>(&mut self) -> Result<T, Self::Error> {
        let start = self.pos as usize;
        let end = start + size_of::<T>();
        if end > self.save.len() {
            return Err(ReadError::UnexpectedEof);
        }
        self.pos = end as u32;
        Ok(unsafe { T::read_le(self.save.as_ptr().add(start) as *const T) })
    }

    #[inline]
    fn load_bytes(&mut self, len: usize) -> Result<*const u8, Self::Error> {
        let start = self.pos as usize;
        let end = start + len;
        if end > self.save.len() {
            return Err(ReadError::UnexpectedEof);
        }
        self.pos = end as u32;
        Ok(unsafe { self.save.as_ptr().add(start) })
    }

    #[inline]
    fn start_struct(&mut self) -> Result<(), Self::Error> {
        let mut pos = self.load_raw::<u32>()? as usize;
        let fields_len = *self.save.get(pos).ok_or(ReadError::UnexpectedEof)? as usize;
        pos += 1;

        let mut fields = Vec::with_capacity(fields_len);
        for _ in 0..fields_len {
            let ident_bytes: &'a [u8] = unsafe { self.save.get_unchecked(pos..) };
            let len = ident_bytes
                .iter()
                .position(|b| *b == 0)
                .unwrap_or(ident_bytes.len());
            let ident_bytes = &ident_bytes[..len];
            let ident_end = pos + len + 1;

            pos = ident_end + 4;
            if pos > self.save.len() {
                return Err(ReadError::UnexpectedEof);
            }

            fields.push((ident_bytes, unsafe {
                u32::read_le(self.save.as_ptr().add(ident_end) as *const u32)
            }));
        }

        self.structs.push(StructInfo {
            fields,
            end: pos as u32,
            cur_field: 0,
        });
        Ok(())
    }

    #[inline]
    fn end_struct(&mut self) -> Result<(), Self::Error> {
        match self.structs.pop() {
            Some(struct_info) => {
                self.pos = struct_info.end;
                Ok(())
            }
            None => Err(ReadError::NoStructPresent),
        }
    }

    #[inline]
    fn start_field(&mut self, ident: &[u8]) -> Result<(), Self::Error> {
        let cur_struct = self.structs.last_mut().ok_or(ReadError::NoStructPresent)?;
        let mut i = cur_struct.cur_field;
        let len = cur_struct.fields.len() as u8;
        loop {
            let field = cur_struct.fields[i as usize];
            i += 1;
            if i == len {
                i = 0;
            }
            if field.0 == ident {
                cur_struct.cur_field = i;
                self.pos = field.1;
                return Ok(());
            }
            if i == cur_struct.cur_field {
                return Err(ReadError::FieldNotFound);
            }
        }
    }
}

macro_rules! impl_loadable_raw {
    () => {};

    ($ty: ty as bits $conv_ty: ty $(, $($others: tt)*)?) => {
        impl Loadable for $ty {
            #[inline]
            fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
                save.load_raw::<$conv_ty>().map(|v| <$ty>::from_bits(v))
            }
        }

        impl LoadableInPlace for $ty {
            #[inline]
            fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                *self = <$ty>::from_bits(save.load_raw::<$conv_ty>()?);
                Ok(())
            }
        }

        impl_loadable_raw!($($($others)*)*);
    };

    ($ty: ty as $conv_ty: ty $(, $($others: tt)*)?) => {
        impl Loadable for $ty {
            #[inline]
            fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
                save.load_raw::<$conv_ty>().map(|v| v as $ty)
            }
        }

        impl LoadableInPlace for $ty {
            #[inline]
            fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                *self = save.load_raw::<$conv_ty>()? as $ty;
                Ok(())
            }
        }

        impl_loadable_raw!($($($others)*)*);
    };

    ($ty: ty $(, $($others: tt)*)?) => {
        impl Loadable for $ty {
            #[inline]
            fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
                save.load_raw()
            }
        }

        impl LoadableInPlace for $ty {
            #[inline]
            fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                *self = save.load_raw()?;
                Ok(())
            }
        }

        impl_loadable_raw!($($($others)*)*);
    };
}

#[rustfmt::skip]
impl_loadable_raw!(
    u8, u16, u32, u64, u128, usize as u32,
    i8, i16, i32, i64, i128, isize as i32,
    f32 as bits u32, f64 as bits u64
);

macro_rules! impl_loadable_tuples {
    ($(($($ty: ident, $index: tt),*)),*) => {
        $(
            impl<$($ty),*> Loadable for ($($ty,)*) where $($ty: Loadable),* {
                #[inline]
                fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
                    Ok(($(save.load::<$ty>()?,)*))
                }
            }

            impl<$($ty),*> LoadableInPlace for ($($ty,)*) where $($ty: LoadableInPlace),* {
                #[inline]
                fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                    $(
                        self.$index.load_in_place(save)?;
                    )*
                    Ok(())
                }
            }
        )*
    };
}

impl_loadable_tuples!(
    (A, 0),
    (A, 0, B, 1),
    (A, 0, B, 1, C, 2),
    (A, 0, B, 1, C, 2, D, 3),
    (A, 0, B, 1, C, 2, D, 3, E, 4),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8, J, 9),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8, J, 9, K, 10),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8, J, 9, K, 10, L, 11),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8, J, 9, K, 10, L, 11, M, 12),
    (A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8, J, 9, K, 10, L, 11, M, 12, N, 13),
    (
        A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8, J, 9, K, 10, L, 11, M, 12, N, 13, O,
        14
    ),
    (
        A, 0, B, 1, C, 2, D, 3, E, 4, F, 5, G, 6, H, 7, I, 8, J, 9, K, 10, L, 11, M, 12, N, 13, O,
        14, P, 15
    )
);

impl<T> Loadable for Vec<T>
where
    T: Loadable,
{
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        let len = save.load_raw::<u32>()? as usize;
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(save.load()?);
        }
        Ok(result)
    }
}

impl<T, const LEN: usize> Loadable for [T; LEN]
where
    T: Loadable,
{
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        let mut result = MaybeUninit::uninit_array();
        for elem in &mut result {
            *elem = MaybeUninit::new(save.load()?);
        }
        Ok(unsafe { MaybeUninit::array_assume_init(result) })
    }
}

impl<T, const LEN: usize> LoadableInPlace for [T; LEN]
where
    T: LoadableInPlace,
{
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        for elem in self {
            save.load_into(elem)?;
        }
        Ok(())
    }
}

impl<T: SimdElement, const LANES: usize> Loadable for Simd<T, LANES>
where
    T: Loadable,
    LaneCount<LANES>: SupportedLaneCount,
{
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        let mut result = MaybeUninit::uninit_array();
        for elem in &mut result {
            *elem = MaybeUninit::new(save.load()?);
        }
        Ok(Self::from_array(unsafe {
            MaybeUninit::array_assume_init(result)
        }))
    }
}

impl<T: SimdElement, const LANES: usize> LoadableInPlace for Simd<T, LANES>
where
    T: LoadableInPlace,
    LaneCount<LANES>: SupportedLaneCount,
{
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        for elem in self.as_mut_array() {
            save.load_into(elem)?;
        }
        Ok(())
    }
}

impl<const LEN: usize> Loadable for Bytes<LEN> {
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        save.load_bytes(LEN)
            .map(|v| unsafe { (&*(v as *const Bytes<LEN>)).clone() })
    }
}

impl<const LEN: usize> LoadableInPlace for Bytes<LEN> {
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        unsafe {
            ptr::copy_nonoverlapping(save.load_bytes(LEN)?, self.as_mut_ptr(), LEN);
        }
        Ok(())
    }
}

impl<const LEN: usize> Loadable for OwnedBytesCellPtr<LEN> {
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        let bytes = OwnedBytesCellPtr::new_zeroed();
        unsafe {
            ptr::copy_nonoverlapping(save.load_bytes(LEN)?, bytes.as_ptr(), LEN);
        }
        Ok(bytes)
    }
}

impl<const LEN: usize> LoadableInPlace for OwnedBytesCellPtr<LEN> {
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        unsafe {
            ptr::copy_nonoverlapping(save.load_bytes(LEN)?, self.as_ptr(), LEN);
        }
        Ok(())
    }
}

impl<T> Loadable for Box<T>
where
    T: Loadable,
{
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        save.load().map(Box::new)
    }
}

impl<T> LoadableInPlace for Box<T>
where
    T: LoadableInPlace,
{
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.load_into::<T>(&mut *self)
    }
}

impl<T> Loadable for Cell<T>
where
    T: Loadable,
{
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        save.load().map(Cell::new)
    }
}

impl<T> LoadableInPlace for Cell<T>
where
    T: LoadableInPlace,
{
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.load_into(self.get_mut())
    }
}

impl<T> Loadable for Option<T>
where
    T: Loadable,
{
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        Ok(if save.load_raw::<u8>()? == 0 {
            None
        } else {
            Some(save.load()?)
        })
    }
}

impl<T> LoadableInPlace for Option<T>
where
    T: Loadable,
{
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        *self = Self::load(save)?;
        Ok(())
    }
}

impl Loadable for () {
    #[inline]
    fn load<S: ReadSavestate>(_save: &mut S) -> Result<Self, S::Error> {
        Ok(())
    }
}

impl LoadableInPlace for () {
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, _save: &mut S) -> Result<(), S::Error> {
        Ok(())
    }
}

impl Loadable for bool {
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        save.load_raw::<u8>().map(|v| v != 0)
    }
}

impl LoadableInPlace for bool {
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        *self = save.load_raw::<u8>()? != 0;
        Ok(())
    }
}

#[inline]
pub fn load_slice_in_place<S: ReadSavestate, T: LoadableInPlace>(
    slice: &mut [T],
    save: &mut S,
) -> Result<(), S::Error> {
    for elem in slice {
        save.load_into(elem)?;
    }
    Ok(())
}
