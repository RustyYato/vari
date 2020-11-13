use super::*;

use core::cmp::Ordering;
use core::fmt;
use core::future::Future;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
#[cfg(feature = "nightly")]
use core::marker::Unsize;
use core::pin::Pin;
use core::task::{Context, Poll};

impl<T: TypeList> Unpin for Vari<T> {}

impl<T: CloneImp + TypeList> Clone for Vari<T> {
    #[inline]
    fn clone(&self) -> Self {
        let (ptr, tag) = self.untag();
        unsafe {
            Self {
                tagged_ptr: T::clone(ptr, T::ALIGN, tag),
                mark: PhantomData,
            }
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        let (ptr, tag) = self.untag();
        let (src_ptr, src_tag) = source.untag();

        unsafe {
            T::clone_from::<T>(ptr, tag, src_ptr, src_tag, src_tag, &mut self.tagged_ptr);
        }
    }
}

pub struct DebugImp<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl<T: fmt::Debug> Func<T> for DebugImp<'_, '_> {
    type Output = fmt::Result;
    #[inline]
    fn call(self, value: &T) -> fmt::Result {
        value.fmt(self.0)
    }
}

impl<T: TypeList + for<'a, 'b> Apply<DebugImp<'a, 'b>, Output = fmt::Result>> fmt::Debug
    for Vari<T>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ptr, tag) = self.untag();
        unsafe { T::apply(ptr, tag, DebugImp(f)) }
    }
}

pub struct PartialEqImp(*mut ());
impl<T: PartialEq> Func<T> for PartialEqImp {
    type Output = bool;
    #[inline]
    fn call(self, value: &T) -> bool {
        unsafe { value == &*(self.0 as *const T) }
    }
}

pub struct EqImp(());
impl<T: PartialEq> Func<T> for EqImp {
    type Output = ();
}

impl<T> Eq for Vari<T> where T: TypeList + Apply<PartialEqImp, Output = bool> + Apply<EqImp> {}
impl<T> PartialEq for Vari<T>
where
    T: TypeList + Apply<PartialEqImp, Output = bool>,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let (ptr, tag) = self.untag();
        let (optr, otag) = other.untag();
        tag == otag && unsafe { T::apply(ptr, tag, PartialEqImp(optr)) }
    }
}

pub struct PartialOrdImp(*mut ());
impl<T: PartialOrd> Func<T> for PartialOrdImp {
    type Output = Option<Ordering>;
    #[inline]
    fn call(self, value: &T) -> Self::Output {
        unsafe { value.partial_cmp(&*(self.0 as *const T)) }
    }
}
impl<T> PartialOrd for Vari<T>
where
    T: TypeList
        + Apply<PartialEqImp, Output = bool>
        + Apply<EqImp>
        + Apply<PartialOrdImp, Output = Option<Ordering>>,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (ptr, tag) = self.untag();
        let (optr, otag) = other.untag();
        match tag.cmp(&otag) {
            Ordering::Equal => unsafe { T::apply(ptr, tag, PartialOrdImp(optr)) },
            cmp => Some(cmp),
        }
    }
}

pub struct OrdImp(*mut ());
impl<T: Ord> Func<T> for OrdImp {
    type Output = Ordering;
    #[inline]
    fn call(self, value: &T) -> Self::Output {
        unsafe { value.cmp(&*(self.0 as *const T)) }
    }
}
impl<T> Ord for Vari<T>
where
    T: TypeList
        + Apply<PartialEqImp, Output = bool>
        + Apply<EqImp>
        + Apply<PartialOrdImp, Output = Option<Ordering>>
        + Apply<OrdImp, Output = Ordering>,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let (ptr, tag) = self.untag();
        let (optr, otag) = other.untag();
        tag.cmp(&otag)
            .then_with(|| unsafe { T::apply(ptr, tag, OrdImp(optr)) })
    }
}

pub struct HashImp<'a>(&'a mut dyn Hasher);
impl<T: Hash> Func<T> for HashImp<'_> {
    type Output = ();
    #[inline]
    fn call(self, value: &T) -> Self::Output {
        value.hash(&mut { self.0 })
    }
}
impl<T: TypeList + for<'a> Apply<HashImp<'a>>> Hash for Vari<T> {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        let (ptr, tag) = self.untag();
        unsafe {
            T::apply(ptr, tag, HashImp(hasher));
        }
    }
}

pub struct IteratorImp(Option<usize>);
impl<T: Iterator> Func<T> for IteratorImp {
    type Output = Option<T::Item>;
    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        match self.0 {
            None => value.next(),
            Some(n) => value.nth(n),
        }
    }
}
pub struct DoubleEndedIteratorImp(Option<usize>);
impl<T: DoubleEndedIterator> Func<T> for DoubleEndedIteratorImp {
    type Output = Option<T::Item>;
    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        match self.0 {
            None => value.next_back(),
            Some(n) => value.nth_back(n),
        }
    }
}
pub struct IteratorSizeImp(());
impl<T: Iterator> Func<T> for IteratorSizeImp {
    type Output = (usize, Option<usize>);
    #[inline]
    fn call(self, value: &T) -> Self::Output {
        value.size_hint()
    }
}
pub struct FusedIteratorImp(());
impl<T: FusedIterator> Func<T> for FusedIteratorImp {
    type Output = ();
}
pub struct ExactSizeIteratorImp(());
impl<T: ExactSizeIterator> Func<T> for ExactSizeIteratorImp {
    type Output = ();
}
impl<T, Item> Iterator for Vari<T>
where
    T: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>,
{
    type Item = Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (ptr, tag) = self.untag();
        unsafe { T::apply_mut(ptr, tag, IteratorImp(None)) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (ptr, tag) = self.untag();
        unsafe { T::apply(ptr, tag, IteratorSizeImp(())) }
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let (ptr, tag) = self.untag();
        unsafe { T::apply_mut(ptr, tag, IteratorImp(Some(n))) }
    }
}

impl<T, Item> DoubleEndedIterator for Vari<T>
where
    T: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<DoubleEndedIteratorImp, Output = Option<Item>>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let (ptr, tag) = self.untag();
        unsafe { T::apply_mut(ptr, tag, DoubleEndedIteratorImp(None)) }
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let (ptr, tag) = self.untag();
        unsafe { T::apply_mut(ptr, tag, DoubleEndedIteratorImp(Some(n))) }
    }
}

impl<T, Item> FusedIterator for Vari<T> where
    T: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<FusedIteratorImp>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>
{
}

impl<T, Item> ExactSizeIterator for Vari<T> where
    T: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<ExactSizeIteratorImp>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>
{
}

pub struct FutureImp<'a, 'b>(&'a mut Context<'b>);
impl<T: Future + Unpin> Func<T> for FutureImp<'_, '_> {
    type Output = Poll<T::Output>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        Pin::new(value).poll(self.0)
    }
}

impl<T, Output> Future for Vari<T>
where
    T: TypeList + for<'a, 'b> Apply<FutureImp<'a, 'b>, Output = Poll<Output>>,
{
    type Output = Output;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (ptr, tag) = self.untag();
        unsafe { T::apply_mut(ptr, tag, FutureImp(cx)) }
    }
}

pub struct PinFutureImp<'a, 'b>(&'a mut Context<'b>);
impl<T: Future> Func<T> for PinFutureImp<'_, '_> {
    type Output = Poll<T::Output>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        unsafe { Pin::new_unchecked(value).poll(self.0) }
    }
}

impl<T, Output> Future for PinVari<T>
where
    T: TypeList + for<'a, 'b> Apply<PinFutureImp<'a, 'b>, Output = Poll<Output>>,
{
    type Output = Output;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (ptr, tag) = Pin::into_inner(self).0.untag();
        unsafe { T::apply_mut(ptr, tag, PinFutureImp(cx)) }
    }
}

#[cfg(feature = "nightly")]
pub struct UnsizeImp<U: ?Sized>(pub PhantomData<U>);
#[cfg(feature = "nightly")]
impl<T: Unsize<U>, U: ?Sized> Func<T> for UnsizeImp<U> {
    type Output = *mut U;

    #[inline]
    fn call_raw(self, value: *mut T) -> Self::Output {
        value as *mut U
    }
}

#[cfg(feature = "nightly")]
pub trait UnsizeAny<U: ?Sized>: Apply<UnsizeImp<U>> {}
#[cfg(feature = "nightly")]
impl<T: Apply<UnsizeImp<U>>, U: ?Sized> UnsizeAny<U> for T {}

pub trait UnpinTuple {}
impl<T: Apply<UnpinTupleImp>> UnpinTuple for T {}

pub struct UnpinTupleImp(());
impl<T: Unpin> Func<T> for UnpinTupleImp {
    type Output = ();
}
