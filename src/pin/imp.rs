use crate::{
    internals::{Apply, Func, TypeList},
    PinVari, Vari,
    _alloc::AllocStrategy,
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

impl<L, S> Clone for PinVari<L, S>
where
    Vari<L, S>: Clone,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0)
    }
}

impl<L, S> Eq for PinVari<L, S>
where
    Vari<L, S>: Eq,
    L: TypeList,
    S: AllocStrategy<L>,
{
}
impl<L, S> PartialEq for PinVari<L, S>
where
    Vari<L, S>: PartialEq,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<L, S> PartialEq<Vari<L, S>> for PinVari<L, S>
where
    Vari<L, S>: PartialEq,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn eq(&self, other: &Vari<L, S>) -> bool {
        &self.0 == other
    }
}

impl<L, S> PartialEq<PinVari<L, S>> for Vari<L, S>
where
    Vari<L, S>: PartialEq,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn eq(&self, other: &PinVari<L, S>) -> bool {
        self == &other.0
    }
}

impl<L, S> PartialOrd for PinVari<L, S>
where
    Vari<L, S>: PartialOrd,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<L, S> PartialOrd<Vari<L, S>> for PinVari<L, S>
where
    Vari<L, S>: PartialOrd,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn partial_cmp(&self, other: &Vari<L, S>) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl<L, S> PartialOrd<PinVari<L, S>> for Vari<L, S>
where
    Vari<L, S>: PartialOrd,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn partial_cmp(&self, other: &PinVari<L, S>) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl<L, S> Ord for PinVari<L, S>
where
    Vari<L, S>: Ord,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<L, S> Hash for PinVari<L, S>
where
    Vari<L, S>: Hash,
    L: TypeList,
    S: AllocStrategy<L>,
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

impl<L, S, Output> Future for PinVari<L, S>
where
    L: TypeList + for<'a, 'b> Apply<PinFutureImp<'a, 'b>, Output = Poll<Output>>,
    S: AllocStrategy<L>,
{
    type Output = Output;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (ptr, tag) = Pin::into_inner(self).0.split();
        unsafe { L::apply_mut(ptr, tag, PinFutureImp(cx)) }
    }
}

impl<L, S> fmt::Debug for PinVari<L, S>
where
    Vari<L, S>: fmt::Debug,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<L, S> fmt::Display for PinVari<L, S>
where
    Vari<L, S>: fmt::Display,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "std")]
impl<L, S> Error for PinVari<L, S>
where
    Vari<L, S>: Error,
    L: TypeList,
    S: AllocStrategy<L>,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}
