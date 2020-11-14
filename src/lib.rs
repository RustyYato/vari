#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(unsize, dropck_eyepatch))]

#[cfg(not(feature = "std"))]
extern crate alloc as std;

use core::marker::PhantomData;
use core::pin::Pin;
use core::ptr::NonNull;

mod imp;
mod imp_pin;

mod internals;

#[cfg(test)]
mod tests;

// TODO - docs
// TODO - safety tests
// TODO - validity tests

use traits::*;
pub mod traits {
    pub use crate::imp::UnpinTuple;
    #[cfg(feature = "nightly")]
    pub use crate::imp::UnsizeAny;
    pub use crate::internals::{
        Contains, GetAny, IntoInner, IntoSuperSet, Peano, TryIntoSubSet, TypeList,
    };
}

pub mod parts {
    pub use crate::internals::{CNil, CoProd, S, Z};
    include!(concat!(env!("OUT_DIR"), "/num.rs"));
}

include!(concat!(env!("OUT_DIR"), "/aliases.rs"));

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

#[repr(transparent)]
pub struct Vari<T: TypeList> {
    tagged_ptr: NonNull<()>,
    mark: PhantomData<T>,
}

#[repr(transparent)]
pub struct PinVari<T: TypeList>(Vari<T>);

#[cfg(not(feature = "nightly"))]
impl<T: TypeList> Drop for Vari<T> {
    fn drop(&mut self) {
        let (ptr, tag) = self.untag();
        unsafe { internals::destroy::<T>(ptr, tag) }
    }
}

#[cfg(feature = "nightly")]
unsafe impl<#[may_dangle] T: TypeList> Drop for Vari<T> {
    fn drop(&mut self) {
        let (ptr, tag) = self.untag();
        unsafe { internals::destroy::<T>(ptr, tag) }
    }
}

impl<T: TypeList> PinVari<T> {
    pub unsafe fn into_inner_unchecked(self) -> Vari<T> {
        self.0
    }

    pub fn into_inner(self) -> Vari<T>
    where
        T: imp::UnpinTuple,
    {
        unsafe { self.into_inner_unchecked() }
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        self.0.untag().0
    }

    #[inline]
    pub fn tag(&self) -> usize {
        self.0.untag().1
    }

    #[inline]
    pub fn tag_for<A, N>() -> usize
    where
        T: Contains<A, N>,
        N: Peano,
    {
        N::VALUE
    }

    #[inline]
    pub fn is<A, N>(&self) -> bool
    where
        T: Contains<A, N>,
        N: Peano,
    {
        self.0.is()
    }

    #[inline]
    pub fn get_any<'a>(&'a self) -> T::PinRef
    where
        T: GetAny<'a>,
    {
        let (ptr, tag) = self.0.untag();
        unsafe { T::_pin_get_any(ptr, tag) }
    }

    #[inline]
    pub fn get_any_mut<'a>(&'a self) -> T::PinRefMut
    where
        T: GetAny<'a>,
    {
        let (ptr, tag) = self.0.untag();
        unsafe { T::_pin_get_any_mut(ptr, tag) }
    }

    #[inline]
    pub fn get<'a, A, N>(&self) -> Pin<&A>
    where
        T: Contains<A, N>,
        N: Peano,
    {
        unsafe { Pin::new_unchecked(self.0.get()) }
    }

    #[inline]
    pub fn get_mut<'a, A, N>(&mut self) -> Pin<&mut A>
    where
        T: Contains<A, N>,
        N: Peano,
    {
        unsafe { Pin::new_unchecked(self.0.get_mut()) }
    }

    #[inline]
    pub fn try_get<'a, A, N>(&self) -> Option<Pin<&A>>
    where
        T: Contains<A, N>,
        N: Peano,
    {
        self.0.try_get().map(|x| unsafe { Pin::new_unchecked(x) })
    }

    #[inline]
    pub fn try_get_mut<'a, A, N>(&mut self) -> Option<Pin<&mut A>>
    where
        T: Contains<A, N>,
        N: Peano,
    {
        self.0
            .try_get_mut()
            .map(|x| unsafe { Pin::new_unchecked(x) })
    }

    pub fn set<N, A>(&mut self, value: A)
    where
        T: Contains<A, N>,
        N: Peano,
    {
        self.0.set(value)
    }

    #[cfg(feature = "nightly")]
    pub fn unsize<U: ?Sized>(&self) -> &U
    where
        T: imp::UnsizeAny<U, Output = *mut U>,
    {
        self.0.unsize()
    }

    #[cfg(feature = "nightly")]
    pub fn unsize_mut<U: ?Sized>(&mut self) -> &mut U
    where
        T: imp::UnsizeAny<U, Output = *mut U>,
    {
        self.0.unsize_mut()
    }
}

impl<T: TypeList> Vari<T> {
    pub const TAG_BITS: u32 = T::SIZE_CLASS;

    #[inline]
    pub fn new<N, V>(value: V) -> Self
    where
        T: Contains<V, N>,
        N: Peano,
    {
        Self::new_with(move || value)
    }

    #[inline]
    pub fn new_with<N, V, F: FnOnce() -> V>(value: F) -> Self
    where
        T: Contains<V, N>,
        N: Peano,
    {
        Self {
            tagged_ptr: internals::new_with(value, T::ALIGN, N::VALUE),
            mark: PhantomData,
        }
    }

    pub fn pin(self) -> PinVari<T> {
        PinVari(self)
    }

    #[inline]
    fn untag(&self) -> (*mut (), usize) {
        fn untag(tagged_ptr: *mut (), mask: usize) -> (*mut (), usize) {
            let tagged_ptr = tagged_ptr as usize;
            ((tagged_ptr & !mask) as *mut (), tagged_ptr & mask)
        }
        untag(self.tagged_ptr.as_ptr(), T::ALIGN - 1)
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        self.untag().0
    }

    #[inline]
    pub fn tag(&self) -> usize {
        self.untag().1
    }

    #[inline]
    pub fn tag_for<A, N>() -> usize
    where
        T: Contains<A, N>,
        N: Peano,
    {
        N::VALUE
    }

    #[inline]
    pub fn is<A, N>(&self) -> bool
    where
        T: Contains<A, N>,
        N: Peano,
    {
        N::VALUE == self.tag()
    }

    unsafe fn convert<O: TypeList>(self, other_tag: usize) -> Vari<O> {
        let (ptr, tag) = self.untag();
        core::mem::forget(self);

        let layout = T::layout(tag, T::ALIGN);
        let super_layout = O::layout(other_tag, O::ALIGN);

        assert_eq!(layout.size(), super_layout.size());

        if layout == super_layout {
            let tagged_ptr = NonNull::new_unchecked(((ptr as usize) | other_tag) as *mut ());

            Vari {
                tagged_ptr,
                mark: PhantomData,
            }
        } else {
            let size = layout.size();
            let _dealloc = internals::DeallocOnDrop(ptr.cast(), layout);

            Vari {
                tagged_ptr: internals::raw_new_with(
                    |out| out.copy_from_nonoverlapping(ptr, size),
                    super_layout,
                    O::ALIGN,
                    other_tag,
                ),
                mark: PhantomData,
            }
        }
    }

    pub fn into_superset<O: TypeList, I>(self) -> Vari<O>
    where
        T: internals::IntoSuperSet<O, I>,
    {
        let tag = self.tag();
        unsafe { self.convert(T::convert_index(tag)) }
    }

    pub fn try_into_subset<O: TypeList, I>(self) -> Result<Vari<O>, Self>
    where
        T: internals::TryIntoSubSet<O, I>,
    {
        match T::convert_index(self.tag(), 0) {
            Some(sub_index) => unsafe { Ok(self.convert(sub_index)) },
            None => Err(self),
        }
    }

    pub fn from_subset<O: TypeList, I>(vari: Vari<O>) -> Self
    where
        O: internals::IntoSuperSet<T, I>,
    {
        vari.into_superset()
    }

    pub fn try_from_superset<O: TypeList, I>(vari: Vari<O>) -> Result<Self, Vari<O>>
    where
        O: internals::TryIntoSubSet<T, I>,
    {
        vari.try_into_subset()
    }

    #[inline]
    pub fn get_any<'a>(&'a self) -> T::Ref
    where
        T: GetAny<'a>,
    {
        let (ptr, tag) = self.untag();
        unsafe { T::_get_any(ptr, tag) }
    }

    #[inline]
    pub fn get_any_mut<'a>(&'a self) -> T::RefMut
    where
        T: GetAny<'a>,
    {
        let (ptr, tag) = self.untag();
        unsafe { T::_get_any_mut(ptr, tag) }
    }

    #[inline]
    pub fn into_inner(self) -> T
    where
        T: IntoInner,
    {
        let (ptr, tag) = self.untag();
        core::mem::forget(self);
        unsafe {
            let _dealloc = internals::DeallocOnDrop(ptr, T::layout(tag, T::ALIGN));
            T::_into_inner(ptr, tag)
        }
    }

    #[inline]
    pub fn get<'a, A, N>(&self) -> &A
    where
        T: Contains<A, N>,
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
        T: Contains<A, N>,
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
        T: Contains<A, N>,
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
        T: Contains<A, N>,
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
        T: Contains<A, N>,
        N: Peano,
    {
        struct WriteOnDrop<T>(*mut (), Option<T>);

        impl<T> Drop for WriteOnDrop<T> {
            fn drop(&mut self) {
                unsafe { self.0.cast::<T>().write(self.1.take().unwrap()) }
            }
        }

        let (ptr, index) = self.untag();
        let layout = unsafe { T::layout(index, T::ALIGN) };
        if layout == unsafe { internals::layout::<A>(T::ALIGN) } {
            unsafe {
                let _write = WriteOnDrop(ptr, Some(value));
                self.tagged_ptr = NonNull::new_unchecked((ptr as usize | N::VALUE) as *mut ());
                T::drop_in_place(ptr, index);
            }
        } else {
            *self = Self::new(value);
        }
    }

    #[cfg(feature = "nightly")]
    pub fn unsize<U: ?Sized>(&self) -> &U
    where
        T: imp::UnsizeAny<U, Output = *mut U>,
    {
        let (ptr, tag) = self.untag();
        unsafe { &*T::apply_raw(ptr, tag, imp::UnsizeImp::<U>(PhantomData)) }
    }

    #[cfg(feature = "nightly")]
    pub fn unsize_mut<U: ?Sized>(&mut self) -> &mut U
    where
        T: imp::UnsizeAny<U, Output = *mut U>,
    {
        let (ptr, tag) = self.untag();
        unsafe { &mut *T::apply_raw(ptr, tag, imp::UnsizeImp::<U>(PhantomData)) }
    }
}
