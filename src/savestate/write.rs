use crate::{Bytes, MemValue, OwnedBytesCellPtr};
use core::{
    cell::Cell,
    convert::Infallible,
    mem::size_of,
    ptr,
    simd::{LaneCount, Simd, SimdElement, SupportedLaneCount},
};

pub trait Storable {
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error>;
}

pub trait WriteSavestate: Sized {
    type Error;

    const TRANSIENT: bool;

    fn store_array_len(&mut self, len: usize) -> Result<(), Self::Error>;
    fn store_raw<T: MemValue>(&mut self, value: T);
    fn store_bytes<const LEN: usize>(&mut self, bytes: &Bytes<LEN>);

    fn start_struct(&mut self) -> Result<(), Self::Error>;
    fn end_struct(&mut self) -> Result<(), Self::Error>;
    fn start_field(&mut self, ident: &'static [u8]) -> Result<(), Self::Error>;

    #[inline]
    fn store<T: Storable>(&mut self, value: &mut T) -> Result<(), Self::Error> {
        value.store(self)
    }
}

// Used for fast, unchecked in-memory savestates (i.e. rewinding).
pub struct TransientWriteSavestate<'a> {
    save: &'a mut Vec<u8>,
}

impl<'a> TransientWriteSavestate<'a> {
    pub fn new(save: &'a mut Vec<u8>) -> Self {
        TransientWriteSavestate { save }
    }
}

impl<'a> WriteSavestate for TransientWriteSavestate<'a> {
    type Error = Infallible;

    const TRANSIENT: bool = true;

    #[inline]
    fn store_array_len(&mut self, len: usize) -> Result<(), Self::Error> {
        self.store_raw(len as u32);
        Ok(())
    }

    #[inline]
    fn store_raw<T: MemValue>(&mut self, value: T) {
        unsafe {
            let pos = self.save.len();
            self.save.reserve(size_of::<T>());
            value.write_ne(self.save.as_mut_ptr().add(pos) as *mut T);
            self.save.set_len(self.save.len() + size_of::<T>());
        }
    }

    #[inline]
    fn store_bytes<const LEN: usize>(&mut self, bytes: &Bytes<LEN>) {
        unsafe {
            let pos = self.save.len();
            self.save.reserve(LEN);
            ptr::copy_nonoverlapping(bytes.as_ptr(), self.save.as_mut_ptr().add(pos), LEN);
            self.save.set_len(self.save.len() + LEN);
        }
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
    fn start_field(&mut self, _ident: &'static [u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct StructInfo {
    start_pos: u32,
    fields: Vec<(&'static [u8], u32)>,
}

pub struct PersistentWriteSavestate<'a> {
    save: &'a mut Vec<u8>,
    structs: Vec<StructInfo>,
}

impl<'a> PersistentWriteSavestate<'a> {
    #[inline]
    pub fn new(save: &'a mut Vec<u8>) -> Self {
        PersistentWriteSavestate {
            save,
            structs: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WriteError {
    NoStructPresent,
    TooManyFields,
    SaveTooLarge,
}

impl<'a> WriteSavestate for PersistentWriteSavestate<'a> {
    type Error = WriteError;

    const TRANSIENT: bool = false;

    #[inline]
    fn store_array_len(&mut self, len: usize) -> Result<(), Self::Error> {
        self.store_raw(u32::try_from(len).map_err(|_| WriteError::TooManyFields)?);
        Ok(())
    }

    #[inline]
    fn store_raw<T: MemValue>(&mut self, value: T) {
        unsafe {
            let pos = self.save.len();
            self.save.reserve(size_of::<T>());
            value.write_le(self.save.as_mut_ptr().add(pos) as *mut T);
            self.save.set_len(self.save.len() + size_of::<T>());
        }
    }

    #[inline]
    fn store_bytes<const LEN: usize>(&mut self, bytes: &Bytes<LEN>) {
        unsafe {
            let pos = self.save.len();
            self.save.reserve(LEN);
            ptr::copy_nonoverlapping(bytes.as_ptr(), self.save.as_mut_ptr().add(pos), LEN);
            self.save.set_len(self.save.len() + LEN);
        }
    }

    #[inline]
    fn start_struct(&mut self) -> Result<(), Self::Error> {
        let start_pos = u32::try_from(self.save.len()).map_err(|_| WriteError::SaveTooLarge)?;
        self.save.extend_from_slice(&[0; 4]);
        self.structs.push(StructInfo {
            start_pos,
            fields: Vec::new(),
        });

        Ok(())
    }

    #[inline]
    fn end_struct(&mut self) -> Result<(), Self::Error> {
        let cur_struct = self.structs.pop().ok_or(WriteError::NoStructPresent)?;

        let field_info_pos =
            u32::try_from(self.save.len()).map_err(|_| WriteError::SaveTooLarge)?;
        unsafe {
            field_info_pos
                .write_le(self.save.as_mut_ptr().add(cur_struct.start_pos as usize) as *mut u32);
        }

        self.save
            .push(u8::try_from(cur_struct.fields.len()).map_err(|_| WriteError::TooManyFields)?);

        for (ident, pos) in cur_struct.fields {
            self.save.extend_from_slice(ident);
            self.save.push(0);
            self.store_raw(pos);
        }

        Ok(())
    }

    #[inline]
    fn start_field(&mut self, ident: &'static [u8]) -> Result<(), Self::Error> {
        let cur_struct = self.structs.last_mut().ok_or(WriteError::NoStructPresent)?;

        let pos = u32::try_from(self.save.len()).map_err(|_| WriteError::SaveTooLarge)?;
        cur_struct.fields.push((ident, pos));

        Ok(())
    }
}

macro_rules! impl_storable_raw {
    () => {};

    ($ty: ty as bits $(, $($others: tt)*)?) => {
        impl Storable for $ty {
            #[inline]
            fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                save.store_raw(self.to_bits());
                Ok(())
            }
        }

        impl_storable_raw!($($($others)*)*);
    };

    ($ty: ty as $conv_ty: ty $(, $($others: tt)*)?) => {
        impl Storable for $ty {
            #[inline]
            fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                save.store_raw(*self as $conv_ty);
                Ok(())
            }
        }

        impl_storable_raw!($($($others)*)*);
    };

    ($ty: ty $(, $($others: tt)*)?) => {
        impl Storable for $ty {
            #[inline]
            fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                save.store_raw(*self);
                Ok(())
            }
        }

        impl_storable_raw!($($($others)*)*);
    };
}

#[rustfmt::skip]
impl_storable_raw!(
    u8, u16, u32, u64, u128, usize as u32,
    i8, i16, i32, i64, i128, isize as i32,
    f32 as bits, f64 as bits
);

macro_rules! impl_storable_tuples {
    ($(($($ty: ident, $index: tt),*)),*) => {
        $(
            impl<$($ty),*> Storable for ($($ty,)*) where $($ty: Storable),* {
                #[inline]
                fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
                    $(save.store(&mut self.$index)?;)*
                    Ok(())
                }
            }
        )*
    };
}

impl_storable_tuples!(
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

impl<T> Storable for Vec<T>
where
    T: Storable,
{
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.store_array_len(self.len())?;
        for elem in self {
            elem.store(save)?;
        }
        Ok(())
    }
}

impl<T, const LEN: usize> Storable for [T; LEN]
where
    T: Storable,
{
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        for elem in self {
            elem.store(save)?;
        }
        Ok(())
    }
}

impl<T: SimdElement, const LANES: usize> Storable for Simd<T, LANES>
where
    T: Storable,
    LaneCount<LANES>: SupportedLaneCount,
{
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        for elem in self.as_mut_array() {
            elem.store(save)?;
        }
        Ok(())
    }
}

impl<const LEN: usize> Storable for Bytes<LEN> {
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.store_bytes(self);
        Ok(())
    }
}

impl<const LEN: usize> Storable for OwnedBytesCellPtr<LEN> {
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.store_bytes(unsafe { &*self.as_bytes_ptr() });
        Ok(())
    }
}

impl<T> Storable for Box<T>
where
    T: Storable,
{
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.store::<T>(&mut *self)
    }
}

impl<T> Storable for Cell<T>
where
    T: Storable,
{
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.store(self.get_mut())
    }
}

impl<T> Storable for Option<T>
where
    T: Storable,
{
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        if let Some(value) = self {
            save.store_raw(1_u8);
            save.store(value)?;
        } else {
            save.store_raw(0_u8);
        }
        Ok(())
    }
}

impl Storable for () {
    #[inline]
    fn store<S: WriteSavestate>(&mut self, _save: &mut S) -> Result<(), S::Error> {
        Ok(())
    }
}

impl Storable for bool {
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.store_raw(*self as u8);
        Ok(())
    }
}

#[inline]
pub fn store_slice<S: WriteSavestate, T: Storable>(slice: &mut [T], save: &mut S) -> Result<(), S::Error> {
    for elem in slice {
        save.store(elem)?;
    }
    Ok(())
}
