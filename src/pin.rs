use crate::{
    internals::{Contains, GetAny, Peano, TypeList},
    Vari,
    _alloc::{self, AllocStrategy},
    imp::UnpinTuple,
};

use core::pin::Pin;

mod imp;

#[repr(transparent)]
pub struct PinVari<L: TypeList, S: AllocStrategy<L> = _alloc::DefaultStrategy>(Vari<L, S>);

impl<L: TypeList, S: AllocStrategy<L>> PinVari<L, S> {
    pub unsafe fn into_inner_unchecked(self) -> Vari<L, S> {
        self.0
    }

    pub fn into_inner(self) -> Vari<L, S>
    where
        L: UnpinTuple,
    {
        unsafe { self.into_inner_unchecked() }
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        self.0.split().0
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.0.split().1
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
        self.0.is()
    }

    #[inline]
    pub fn get_any<'a>(&'a self) -> L::PinRef
    where
        L: GetAny<'a>,
    {
        let (ptr, index) = self.0.split();
        unsafe { L::_pin_get_any(ptr, index) }
    }

    #[inline]
    pub fn get_any_mut<'a>(&'a self) -> L::PinRefMut
    where
        L: GetAny<'a>,
    {
        let (ptr, index) = self.0.split();
        unsafe { L::_pin_get_any_mut(ptr, index) }
    }

    #[inline]
    pub fn get<'a, A, N>(&self) -> Pin<&A>
    where
        L: Contains<A, N>,
        N: Peano,
    {
        unsafe { Pin::new_unchecked(self.0.get()) }
    }

    #[inline]
    pub fn get_mut<'a, A, N>(&mut self) -> Pin<&mut A>
    where
        L: Contains<A, N>,
        N: Peano,
    {
        unsafe { Pin::new_unchecked(self.0.get_mut()) }
    }

    #[inline]
    pub fn try_get<'a, A, N>(&self) -> Option<Pin<&A>>
    where
        L: Contains<A, N>,
        N: Peano,
    {
        self.0.try_get().map(|x| unsafe { Pin::new_unchecked(x) })
    }

    #[inline]
    pub fn try_get_mut<'a, A, N>(&mut self) -> Option<Pin<&mut A>>
    where
        L: Contains<A, N>,
        N: Peano,
    {
        self.0
            .try_get_mut()
            .map(|x| unsafe { Pin::new_unchecked(x) })
    }

    pub fn set<N, A>(&mut self, value: A)
    where
        L: Contains<A, N>,
        N: Peano,
    {
        self.0.set(value)
    }

    #[cfg(feature = "nightly")]
    pub fn unsize<U: ?Sized>(&self) -> &U
    where
        L: imp::UnsizeAny<U, Output = *mut U>,
    {
        self.0.unsize()
    }

    #[cfg(feature = "nightly")]
    pub fn unsize_mut<U: ?Sized>(&mut self) -> &mut U
    where
        L: imp::UnsizeAny<U, Output = *mut U>,
    {
        self.0.unsize_mut()
    }
}

impl<L: TypeList, S: AllocStrategy<L>> From<Vari<L, S>> for PinVari<L, S> {
    #[inline]
    fn from(vari: Vari<L, S>) -> Self {
        Self(vari)
    }
}
