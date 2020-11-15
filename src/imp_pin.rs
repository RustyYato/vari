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

impl<L: TypeList> Clone for PinVari<L>
where
    Vari<L>: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0)
    }
}

impl<L: TypeList> Eq for PinVari<L> where Vari<L>: Eq {}
impl<L: TypeList> PartialEq for PinVari<L>
where
    Vari<L>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<L: TypeList> PartialEq<Vari<L>> for PinVari<L>
where
    Vari<L>: PartialEq,
{
    fn eq(&self, other: &Vari<L>) -> bool {
        &self.0 == other
    }
}

impl<L: TypeList> PartialEq<PinVari<L>> for Vari<L>
where
    Vari<L>: PartialEq,
{
    fn eq(&self, other: &PinVari<L>) -> bool {
        self == &other.0
    }
}

impl<L: TypeList> PartialOrd for PinVari<L>
where
    Vari<L>: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<L: TypeList> PartialOrd<Vari<L>> for PinVari<L>
where
    Vari<L>: PartialOrd,
{
    fn partial_cmp(&self, other: &Vari<L>) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl<L: TypeList> PartialOrd<PinVari<L>> for Vari<L>
where
    Vari<L>: PartialOrd,
{
    fn partial_cmp(&self, other: &PinVari<L>) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl<L: TypeList> Ord for PinVari<L>
where
    Vari<L>: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<L: TypeList> Hash for PinVari<L>
where
    Vari<L>: Hash,
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

impl<L, Output> Future for PinVari<L>
where
    L: TypeList + for<'a, 'b> Apply<PinFutureImp<'a, 'b>, Output = Poll<Output>>,
{
    type Output = Output;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (ptr, tag) = Pin::into_inner(self).0.split();
        unsafe { L::apply_mut(ptr, tag, PinFutureImp(cx)) }
    }
}

impl<L: TypeList> fmt::Debug for PinVari<L>
where
    Vari<L>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<L: TypeList> fmt::Display for PinVari<L>
where
    Vari<L>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "std")]
impl<L: TypeList> Error for PinVari<L>
where
    Vari<L>: Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}
