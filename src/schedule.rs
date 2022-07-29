use crate::Savestate;
use core::{mem, ops::Add};

#[macro_export]
macro_rules! def_timestamp {
    (
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident
    ) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        $(#[$($attr)*])*
        $vis struct $name(pub $crate::schedule::RawTimestamp);
        
        impl ::core::ops::Add for $name {
            type Output = Self;
            #[inline]
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }
        
        impl ::core::convert::From<$crate::schedule::RawTimestamp> for $name {
            #[inline]
            fn from(v: $crate::schedule::RawTimestamp) -> Self {
                Self(v)
            }
        }
        
        impl ::core::convert::From<$name> for $crate::schedule::RawTimestamp {
            #[inline]
            fn from(v: $name) -> Self {
                v.0
            }
        }
    };
}

#[macro_export]
macro_rules! def_event_slots {
    (@__inner $esi: ty, $n: expr$(,)*) => {
        pub(super) const LEN: usize = $n;
    };
    (@__inner $esi: ty, $n: expr$(,)+ #[cfg($($cond: tt)*)] $ident: ident, $($other: tt)*) => {
        $crate::cfg_if::cfg_if! {
            if #[cfg($($cond)*)] {
                pub const $ident: $esi = <$esi>::new($n);
                def_event_slots!(@__inner $esi, $n + 1, $($other)*);
            } else {
                def_event_slots!(@__inner $esi, $n, $($other)*);
            }
        }
    };
    (@__inner $esi: ty, $n: expr$(,)+ $ident: ident, $($other: tt)*) => {
        pub const $ident: $esi = <$esi>::new($n);
        def_event_slots!(@__inner $esi, $n + 1, $($other)*);
    };
    (
        @__inner $esi: ty,
        $n: expr$(,)+
        #[cfg($($cond: tt)*)]
        $start_ident: ident..$end_ident: ident $len: expr,
        $($other: tt)*
    ) => {
        $crate::cfg_if::cfg_if! {
            if #[cfg($($cond)*)] {
                pub const $start_ident: $esi = <$esi>::new($n);
                pub const $end_ident: $esi = <$esi>::new($n + $len - 1);
                def_event_slots!(@__inner $esi, $n + $len, $($other)*);
            } else {
                def_event_slots!(@__inner $esi, $n, $($other)*);
            }
        }
    };
    (
        @__inner $esi: ty,
        $n: expr$(,)+
        $start_ident: ident..$end_ident: ident $len: expr,
        $($other: tt)*
    ) => {
        pub const $start_ident: $esi = <$esi>::new($n);
        pub const $end_ident: $esi = <$esi>::new($n + $len - 1);
        def_event_slots!(@__inner $esi, $n + $len, $($other)*);
    };
    ($esi: ty, $($contents: tt)*) => {
        def_event_slots!(@__inner $esi, 1, $($contents)*,);
    };
    ($vis: vis mod $mod_ident: ident, $esi: ty, $($contents: tt)*) => {
        $vis mod $mod_ident {
            use super::*;
            $crate::def_event_slots!(@__inner $esi, 1, $($contents)*,);
        }
    };
}

#[macro_export]
macro_rules! def_event_slot_index {
    (
        $priv_mod_ident: ident, $event_slots_mod_ident: ident,
        $(#[$($attr: tt)*])* $vis: vis struct $name: ident($inner: ty)
    ) => {
        mod $priv_mod_ident {
            use super::*;
            $crate::bounded_int!(
                $(#[$($attr)*])*
                $vis struct $name($inner),
                max ($event_slots_mod_ident::LEN - 1) as $inner
            );
            $crate::bounded_int_savestate!($name($inner));
        }
        pub use $priv_mod_ident::*;
        
        impl ::core::convert::From<usize> for $name {
            #[inline]
            fn from(v: usize) -> Self {
                assert!(v < $event_slots_mod_ident::LEN);
                unsafe { Self::new_unchecked(v as $inner) }
            }
        }
        
        impl ::core::convert::From<$name> for usize {
            #[inline]
            fn from(v: $name) -> Self {
                v.get() as usize
            }
        }
    };
}

pub type RawTimestamp = u64;
pub type SignedTimestamp = i64;

#[derive(Clone, Copy, Savestate)]
struct EventSlot<
    T: Copy + Ord + Add + From<RawTimestamp> + Into<RawTimestamp>,
    E: Copy + Eq + Default,
    ESI: Copy + Eq + From<usize> + Into<usize>,
> {
    time: T,
    event: E,
    prev_i: ESI,
    next_i: ESI,
}

#[derive(Clone, Savestate)]
pub struct Schedule<
    T: Copy + Ord + Add + From<RawTimestamp> + Into<RawTimestamp>,
    E: Copy + Eq + Default,
    ESI: Copy + Eq + From<usize> + Into<usize>,
    const EVENT_SLOTS: usize,
> {
    slots: [EventSlot<T, E, ESI>; EVENT_SLOTS],
    next_event_time: T,
}

impl<
        T: Copy + Ord + Add + From<RawTimestamp> + Into<RawTimestamp>,
        E: Copy + Eq + Default,
        ESI: Copy + Eq + From<usize> + Into<usize>,
        const EVENT_SLOTS: usize,
    > Schedule<T, E, ESI, EVENT_SLOTS>
{
    pub fn new() -> Self {
        let mut slots = [EventSlot {
            time: T::from(0),
            event: E::default(),
            prev_i: ESI::from(0),
            next_i: ESI::from(0),
        }; EVENT_SLOTS];
        slots[0].time = T::from(RawTimestamp::MAX);
        Schedule {
            slots,
            next_event_time: T::from(RawTimestamp::MAX),
        }
    }

    #[inline]
    pub fn next_event(&self) -> Option<E> {
        let next_i = self.slots[0].next_i.into();
        if next_i == 0 {
            None
        } else {
            Some(self.slots[next_i].event)
        }
    }

    #[inline]
    pub fn next_event_time(&self) -> T {
        self.next_event_time
    }

    pub fn pop_pending_event(&mut self, cur_time: T) -> Option<(E, T)> {
        if cur_time < self.next_event_time {
            return None;
        }
        let slot = &mut self.slots[self.slots[0].next_i.into()];
        slot.time = T::from(0);
        let event = slot.event;
        let next_i = slot.next_i;
        self.slots[0].next_i = next_i;
        let next_slot = &mut self.slots[next_i.into()];
        next_slot.prev_i = ESI::from(0);
        Some((
            event,
            mem::replace(&mut self.next_event_time, next_slot.time),
        ))
    }

    #[inline]
    pub fn set_event(&mut self, slot_index: ESI, event: E) {
        self.slots[slot_index.into()].event = event;
    }

    /// # Panics
    /// May panic if the event at the specified slot is currently scheduled.
    #[allow(clippy::shadow_unrelated)]
    pub fn schedule(&mut self, slot_index: ESI, time: T) {
        let slot = &mut self.slots[slot_index.into()];
        debug_assert!(slot.time == T::from(0));
        slot.time = time;
        if time <= self.next_event_time {
            let next_i = self.slots[0].next_i;
            let slot = &mut self.slots[slot_index.into()];
            slot.prev_i = ESI::from(0);
            slot.next_i = next_i;
            self.slots[next_i.into()].prev_i = slot_index;
            self.slots[0].next_i = slot_index;
            self.next_event_time = time;
        } else {
            let mut next_i = self.slots[self.slots[0].next_i.into()].next_i;
            loop {
                let next_slot = &mut self.slots[next_i.into()];
                if time < next_slot.time {
                    let prev_i = next_slot.prev_i;
                    next_slot.prev_i = slot_index;
                    let slot = &mut self.slots[slot_index.into()];
                    slot.prev_i = prev_i;
                    slot.next_i = next_i;
                    self.slots[prev_i.into()].next_i = slot_index;
                    break;
                }
                next_i = next_slot.next_i;
            }
        }
    }

    /// # Panics
    /// May panic if the event at the specified slot is not currently scheduled.
    #[inline]
    pub fn cancel(&mut self, slot_index: ESI) {
        let slot = &mut self.slots[slot_index.into()];
        debug_assert!(slot.time != T::from(0));
        slot.time = T::from(0);
        let prev_i = slot.prev_i;
        let next_i = slot.next_i;
        self.slots[prev_i.into()].next_i = next_i;
        let next_slot = &mut self.slots[next_i.into()];
        next_slot.prev_i = prev_i;
        if prev_i.into() == 0 {
            let new_next_event_time = next_slot.time;
            self.next_event_time = new_next_event_time;
        }
    }

    #[inline]
    pub fn is_scheduled(&self, slot_index: ESI) -> bool {
        self.slots[slot_index.into()].time != T::from(0)
    }
}

impl<
        T: Copy + Ord + Add + From<RawTimestamp> + Into<RawTimestamp>,
        E: Copy + Eq + Default,
        ESI: Copy + Eq + From<usize> + Into<usize>,
        const EVENT_SLOTS: usize,
    > Default for Schedule<T, E, ESI, EVENT_SLOTS>
{
    fn default() -> Self {
        Self::new()
    }
}
