use crate::{
    internals::{Apply, Func, TypeList},
    PinVari, Vari,
};

use core::cmp::Ordering;
use core::fmt;
use core::future::Future;
use core::hash::{Hash, Hasher};
#[cfg(feature = "nightly")]
use core::marker::Unsize;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(feature = "std")]
use std::error::Error;

impl<T: TypeList> Clone for PinVari<T>
where
    Vari<T>: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0)
    }
}

impl<T: TypeList> Eq for PinVari<T> where Vari<T>: Eq {}
impl<T: TypeList> PartialEq for PinVari<T>
where
    Vari<T>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: TypeList> PartialEq<Vari<T>> for PinVari<T>
where
    Vari<T>: PartialEq,
{
    fn eq(&self, other: &Vari<T>) -> bool {
        &self.0 == other
    }
}

impl<T: TypeList> PartialEq<PinVari<T>> for Vari<T>
where
    Vari<T>: PartialEq,
{
    fn eq(&self, other: &PinVari<T>) -> bool {
        self == &other.0
    }
}

impl<T: TypeList> PartialOrd for PinVari<T>
where
    Vari<T>: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: TypeList> PartialOrd<Vari<T>> for PinVari<T>
where
    Vari<T>: PartialOrd,
{
    fn partial_cmp(&self, other: &Vari<T>) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl<T: TypeList> PartialOrd<PinVari<T>> for Vari<T>
where
    Vari<T>: PartialOrd,
{
    fn partial_cmp(&self, other: &PinVari<T>) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl<T: TypeList> Ord for PinVari<T>
where
    Vari<T>: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: TypeList> Hash for PinVari<T>
where
    Vari<T>: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
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

impl<T: TypeList> fmt::Debug for PinVari<T>
where
    Vari<T>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: TypeList> fmt::Display for PinVari<T>
where
    Vari<T>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "std")]
impl<T: TypeList> Error for PinVari<T>
where
    Vari<T>: Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}
