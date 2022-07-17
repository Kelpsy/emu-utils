use crate::{Loadable, LoadableInPlace, ReadSavestate, Storable, WriteSavestate};
use core::mem::MaybeUninit;

#[derive(Clone, Copy)]
pub struct Fifo<T: Copy, const CAPACITY: usize> {
    buffer: [MaybeUninit<T>; CAPACITY],
    len: usize,
    read_pos: usize,
    write_pos: usize,
}

impl<T: Copy, const CAPACITY: usize> Loadable for Fifo<T, CAPACITY>
where
    T: Loadable,
{
    #[inline]
    fn load<S: ReadSavestate>(save: &mut S) -> Result<Self, S::Error> {
        save.start_struct()?;

        save.start_field(b"len")?;
        let len = save.load_raw::<u32>()? as usize;

        save.start_field(b"buffer")?;
        let mut buffer = [MaybeUninit::uninit(); CAPACITY];
        let slice = if S::TRANSIENT {
            unsafe { buffer.get_unchecked_mut(..len) }
        } else {
            &mut buffer[..len]
        };
        for elem in slice {
            *elem = MaybeUninit::new(save.load()?);
        }

        save.end_struct()?;

        Ok(Fifo {
            buffer,
            len,
            read_pos: 0,
            write_pos: len,
        })
    }
}

impl<T: Copy, const CAPACITY: usize> LoadableInPlace for Fifo<T, CAPACITY>
where
    T: Loadable,
{
    #[inline]
    fn load_in_place<S: ReadSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.start_struct()?;

        save.start_field(b"len")?;
        self.len = save.load_raw::<u32>()? as usize;

        save.start_field(b"buffer")?;
        let slice = if S::TRANSIENT {
            unsafe { self.buffer.get_unchecked_mut(..self.len) }
        } else {
            &mut self.buffer[..self.len]
        };
        for elem in slice {
            *elem = MaybeUninit::new(save.load()?);
        }

        save.end_struct()?;

        self.read_pos = 0;
        self.write_pos = self.len;

        Ok(())
    }
}

impl<T: Copy, const CAPACITY: usize> Storable for Fifo<T, CAPACITY>
where
    T: Storable,
{
    #[inline]
    fn store<S: WriteSavestate>(&mut self, save: &mut S) -> Result<(), S::Error> {
        save.start_struct()?;

        save.start_field(b"len")?;
        save.store_array_len(self.len)?;

        save.start_field(b"buffer")?;
        let mut i = self.read_pos;
        while i != self.write_pos {
            save.store(unsafe { self.buffer.get_unchecked_mut(i).assume_init_mut() })?;
            i += 1;
            if i == CAPACITY {
                i = 0;
            }
        }

        save.end_struct()?;

        Ok(())
    }
}

impl<T: Copy, const CAPACITY: usize> Fifo<T, CAPACITY> {
    #[inline]
    pub fn new() -> Self {
        Fifo {
            buffer: [MaybeUninit::uninit(); CAPACITY],
            len: 0,
            read_pos: 0,
            write_pos: 0,
        }
    }

    #[inline]
    pub fn into_raw(self) -> [MaybeUninit<T>; CAPACITY] {
        self.buffer
    }

    #[inline]
    pub fn as_raw(&self) -> &[MaybeUninit<T>; CAPACITY] {
        &self.buffer
    }

    #[inline]
    pub fn as_raw_mut(&mut self) -> &mut [MaybeUninit<T>; CAPACITY] {
        &mut self.buffer
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn read_pos(&self) -> usize {
        self.read_pos
    }

    #[inline]
    pub fn write_pos(&self) -> usize {
        self.write_pos
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len == CAPACITY
    }

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
        self.read_pos = 0;
        self.write_pos = 0;
    }

    /// # Safety
    /// [`self.is_full()`](Self::is_full) must be `false`.
    #[inline]
    pub unsafe fn write_unchecked(&mut self, value: T) {
        *self.buffer.get_unchecked_mut(self.write_pos) = MaybeUninit::new(value);
        self.write_pos += 1;
        if self.write_pos == CAPACITY {
            self.write_pos = 0;
        }
        self.len += 1;
    }

    #[inline]
    #[must_use]
    pub fn write(&mut self, value: T) -> Option<()> {
        if self.is_full() {
            return None;
        }
        unsafe { self.write_unchecked(value) };
        Some(())
    }

    /// # Safety
    /// [`self.is_empty()`](Self::is_empty) must be `false`.
    #[inline]
    pub unsafe fn read_unchecked(&mut self) -> T {
        let result = self.buffer.get_unchecked(self.read_pos).assume_init();
        self.read_pos += 1;
        if self.read_pos == CAPACITY {
            self.read_pos = 0;
        }
        self.len -= 1;
        result
    }

    #[inline]
    pub fn read(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(unsafe { self.read_unchecked() })
    }

    /// # Safety
    /// [`self.is_empty()`](Self::is_empty) must be `false`.
    #[inline]
    pub unsafe fn peek_unchecked(&self) -> T {
        self.buffer.get_unchecked(self.read_pos).assume_init()
    }

    #[inline]
    pub fn peek(&self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(unsafe { self.peek_unchecked() })
    }
}

impl<T: Copy, const CAPACITY: usize> Default for Fifo<T, CAPACITY> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
