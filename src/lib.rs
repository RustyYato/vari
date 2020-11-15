#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(unsize, dropck_eyepatch))]

#[cfg(not(feature = "std"))]
extern crate alloc as std;

use core::marker::PhantomData;
use core::ptr::NonNull;

#[path = "alloc.rs"]
mod _alloc;
mod imp;
mod internals;
mod pin;

// TODO - docs
// TODO - add new allocation strategy, first allocate as much as required for biggest variant, then never reallocate

pub mod traits {
    pub use crate::_alloc::AllocStrategy;
    pub use crate::imp::UnpinTuple;
    #[cfg(feature = "nightly")]
    pub use crate::imp::UnsizeAny;
    pub use crate::internals::{
        Contains, GetAny, IntoInner, IntoSuperSet, Peano, TryIntoSubSet, TypeList,
    };
}

pub mod alloc {
    pub use crate::_alloc::{BiggestVariant, Minimal};
}

pub mod parts {
    pub use crate::internals::{CNil, CoProd, S, Z};
    include!(concat!(env!("OUT_DIR"), "/num.rs"));
}

include!(concat!(env!("OUT_DIR"), "/aliases.rs"));

pub use pin::PinVari;
use traits::*;

#[macro_export]
macro_rules! tlist {
    () => { $crate::parts::CNil };
    ($first:ty $(, $type:ty)* $(,)?) => {
        $crate::parts::CoProd<$first, $crate::tlist!($($type),*)>
    };
}

#[macro_export]
macro_rules! vari {
    ($($items:ty),* $(,)?) => { $crate::Vari<$crate::tlist!($($items),*)> };
}

#[doc(hidden)]
#[macro_export]
macro_rules! match_any_internal {
    (@internal ($value:expr) [$($output:tt)*] [
        [($nil:pat) ($nil_arm:expr)]
    ]) => {
        match $value {
            $($output)*
            $nil => $nil_arm,
        }
    };
    (@internal ($value:expr) [$($output:tt)*] [
        [($pat:pat) ($arm:expr)]
        $([($rest_pat:pat) ($rest_arm:expr)])*
    ]) => {
        $crate::match_any_internal! {
            @internal ($value) [
                $($output)*
                $pat => $arm,
            ] [
                $([($crate::parts::CoProd::Rest($rest_pat)) ($rest_arm)])*
            ]
        }
    };
}

#[macro_export]
macro_rules! match_any {
    (match $value:expr => {
        $($pat:pat => $arm:expr $(,)?)*
    }) => {
        $crate::match_any_internal! {
            @internal ($value) [] [$([($crate::parts::CoProd::Item($pat)) ($arm)])* [(nil) ({
                let _: $crate::parts::CNil = nil;
                match nil {}
            })]]
        }
    };
}

#[repr(C)]
pub struct Vari<L: TypeList, S: AllocStrategy<L> = _alloc::DefaultStrategy> {
    tagged_ptr: NonNull<()>,
    strategy: S,
    mark: PhantomData<L>,
}

#[cfg(not(feature = "nightly"))]
impl<L: TypeList, S: AllocStrategy<L>> Drop for Vari<L, S> {
    fn drop(&mut self) {
        let (ptr, index) = self.split();
        unsafe { internals::destroy::<L, S>(ptr, index, &self.strategy) }
    }
}

#[cfg(feature = "nightly")]
unsafe impl<#[may_dangle] L: TypeList> Drop for Vari<L> {
    fn drop(&mut self) {
        let (ptr, index) = self.split();
        unsafe { internals::destroy::<L>(ptr, index) }
    }
}

impl<L: TypeList> Vari<L> {
    #[inline]
    pub fn new<N, V>(value: V) -> Self
    where
        L: Contains<V, N>,
        N: Peano,
    {
        Self::new_with(move || value)
    }

    #[inline]
    pub fn new_with<N, V, F: FnOnce() -> V>(value: F) -> Self
    where
        L: Contains<V, N>,
        N: Peano,
    {
        Self::using_strategy_with(value, alloc::BiggestVariant)
    }
}

impl<L: TypeList> Vari<L, alloc::Minimal> {
    #[inline]
    pub fn minimal<N, V>(value: V) -> Self
    where
        L: Contains<V, N>,
        N: Peano,
    {
        Self::minimal_with(move || value)
    }

    #[inline]
    pub fn minimal_with<N, V, F: FnOnce() -> V>(value: F) -> Self
    where
        L: Contains<V, N>,
        N: Peano,
    {
        Self::using_strategy_with(value, alloc::Minimal)
    }
}

impl<L: TypeList, S: AllocStrategy<L>> Vari<L, S> {
    pub const TAG_BITS: u32 = L::SIZE_CLASS;

    #[inline]
    pub fn using_strategy<N, V>(value: V, strategy: S) -> Self
    where
        L: Contains<V, N>,
        N: Peano,
    {
        Self::using_strategy_with(move || value, strategy)
    }

    #[inline]
    pub fn using_strategy_with<N, V, F>(value: F, strategy: S) -> Self
    where
        F: FnOnce() -> V,
        L: Contains<V, N>,
        N: Peano,
    {
        Self {
            tagged_ptr: internals::new_with(value, L::ALIGN, N::VALUE, &strategy),
            strategy,
            mark: PhantomData,
        }
    }

    pub fn pin(self) -> PinVari<L, S> {
        PinVari::from(self)
    }

    #[inline]
    fn split(&self) -> (*mut (), usize) {
        fn split(tagged_ptr: *mut (), mask: usize) -> (*mut (), usize) {
            let tagged_ptr = tagged_ptr as usize;
            ((tagged_ptr & !mask) as *mut (), tagged_ptr & mask)
        }
        split(self.tagged_ptr.as_ptr(), L::ALIGN - 1)
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        self.split().0
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.split().1
    }

    #[inline]
    pub fn index_of<A, N>() -> usize
    where
        L: Contains<A, N>,
        N: Peano,
    {
        N::VALUE
    }

    #[inline]
    pub fn is<A, N>(&self) -> bool
    where
        L: Contains<A, N>,
        N: Peano,
    {
        N::VALUE == self.index()
    }

    unsafe fn convert<O>(self, other_index: usize) -> Vari<O, S>
    where
        O: TypeList,
        S: AllocStrategy<O>,
    {
        let (ptr, index) = self.split();
        let strategy = core::ptr::read(&self.strategy);
        core::mem::forget(self);

        let layout = AllocStrategy::<L>::layout(&strategy, index);
        let super_layout = AllocStrategy::<O>::layout(&strategy, other_index);

        assert_eq!(layout.size(), super_layout.size());

        let tagged_ptr = if layout == super_layout {
            NonNull::new_unchecked(((ptr as usize) | other_index) as *mut ())
        } else {
            let size = layout.size();
            let _dealloc = internals::DeallocOnDrop(ptr.cast(), layout);
            internals::raw_new_with(
                |out| out.copy_from_nonoverlapping(ptr, size),
                super_layout,
                O::ALIGN,
                other_index,
            )
        };

        Vari {
            tagged_ptr,
            strategy,
            mark: PhantomData,
        }
    }

    pub fn into_superset<O, I>(self) -> Vari<O, S>
    where
        O: TypeList,
        S: AllocStrategy<O>,
        L: internals::IntoSuperSet<O, I>,
    {
        let index = self.index();
        unsafe { self.convert(L::convert_index(index)) }
    }

    pub fn try_into_subset<O, I>(self) -> Result<Vari<O, S>, Self>
    where
        O: TypeList,
        S: AllocStrategy<O>,
        L: internals::TryIntoSubSet<O, I>,
    {
        match L::convert_index(self.index(), 0) {
            Some(sub_index) => unsafe { Ok(self.convert(sub_index)) },
            None => Err(self),
        }
    }

    pub fn from_subset<O, I>(vari: Vari<O, S>) -> Self
    where
        O: TypeList + internals::IntoSuperSet<L, I>,
        S: AllocStrategy<O>,
    {
        vari.into_superset()
    }

    pub fn try_from_superset<O, I>(vari: Vari<O, S>) -> Result<Self, Vari<O, S>>
    where
        O: TypeList + internals::TryIntoSubSet<L, I>,
        S: AllocStrategy<O>,
    {
        vari.try_into_subset()
    }

    #[inline]
    pub fn get_any<'a>(&'a self) -> L::Ref
    where
        L: GetAny<'a>,
    {
        let (ptr, index) = self.split();
        unsafe { L::_get_any(ptr, index) }
    }

    #[inline]
    pub fn get_any_mut<'a>(&'a self) -> L::RefMut
    where
        L: GetAny<'a>,
    {
        let (ptr, index) = self.split();
        unsafe { L::_get_any_mut(ptr, index) }
    }

    #[inline]
    pub fn into_inner(self) -> L
    where
        L: IntoInner,
    {
        let (ptr, index) = self.split();
        let strategy = unsafe { core::ptr::read(&self.strategy) };
        core::mem::forget(self);
        unsafe {
            let _dealloc = internals::DeallocOnDrop(ptr, strategy.layout(index));
            L::_into_inner(ptr, index)
        }
    }

    #[inline]
    pub fn get<'a, A, N>(&self) -> &A
    where
        L: Contains<A, N>,
        N: Peano,
    {
        assert!(
            self.is(),
            "Vari doesn't contain {}",
            core::any::type_name::<A>()
        );
        unsafe { &*(self.as_ptr() as *mut A) }
    }

    #[inline]
    pub fn get_mut<'a, A, N>(&mut self) -> &mut A
    where
        L: Contains<A, N>,
        N: Peano,
    {
        assert!(
            self.is(),
            "Vari doesn't contain {}",
            core::any::type_name::<A>()
        );
        unsafe { &mut *(self.as_ptr() as *mut A) }
    }

    #[inline]
    pub fn try_get<'a, A, N>(&self) -> Option<&A>
    where
        L: Contains<A, N>,
        N: Peano,
    {
        if self.is() {
            unsafe { Some(&*(self.as_ptr() as *mut A)) }
        } else {
            None
        }
    }

    #[inline]
    pub fn try_get_mut<'a, A, N>(&mut self) -> Option<&mut A>
    where
        L: Contains<A, N>,
        N: Peano,
    {
        if self.is() {
            unsafe { Some(&mut *(self.as_ptr() as *mut A)) }
        } else {
            None
        }
    }

    pub fn set<N, A>(&mut self, value: A)
    where
        L: Contains<A, N>,
        N: Peano,
    {
        self.set_with(move || value)
    }

    pub fn set_with<N, A, F>(&mut self, value: F)
    where
        F: FnOnce() -> A,
        L: Contains<A, N>,
        N: Peano,
    {
        struct WriteOnDrop<L>(*mut (), Option<L>);

        impl<L> Drop for WriteOnDrop<L> {
            fn drop(&mut self) {
                unsafe { self.0.cast::<L>().write(self.1.take().unwrap()) }
            }
        }

        let (ptr, index) = self.split();
        if unsafe { self.strategy.matches_type_layout::<A>(index) } {
            unsafe {
                let _write = WriteOnDrop(ptr, Some(value()));
                self.tagged_ptr = NonNull::new_unchecked((ptr as usize | N::VALUE) as *mut ());
                L::drop_in_place(ptr, index);
            }
        } else {
            self.tagged_ptr = internals::new_with(value, L::ALIGN, N::VALUE, &self.strategy);
            unsafe {
                internals::destroy(ptr, index, &self.strategy);
            }
        }
    }

    #[cfg(feature = "nightly")]
    pub fn unsize<U: ?Sized>(&self) -> &U
    where
        L: imp::UnsizeAny<U, Output = *mut U>,
    {
        let (ptr, index) = self.split();
        unsafe { &*L::apply_raw(ptr, index, imp::UnsizeImp::<U>(PhantomData)) }
    }

    #[cfg(feature = "nightly")]
    pub fn unsize_mut<U: ?Sized>(&mut self) -> &mut U
    where
        L: imp::UnsizeAny<U, Output = *mut U>,
    {
        let (ptr, index) = self.split();
        unsafe { &mut *L::apply_raw(ptr, index, imp::UnsizeImp::<U>(PhantomData)) }
    }
}
