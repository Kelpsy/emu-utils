use core::mem::MaybeUninit;

#[derive(Clone, Copy)]
pub struct Fifo<T: Copy, const CAPACITY: usize> {
    buffer: [MaybeUninit<T>; CAPACITY],
    len: usize,
    read_pos: usize,
    write_pos: usize,
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
