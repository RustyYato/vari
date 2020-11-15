use crate::{
    internals::{Apply, CloneImp, Func, TypeList},
    Vari,
    _alloc::AllocStrategy,
};

use core::cmp::Ordering;
use core::fmt;
use core::future::Future;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::marker::PhantomData;
#[cfg(feature = "nightly")]
use core::marker::Unsize;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(feature = "std")]
use std::error::Error;
#[cfg(feature = "std")]
use std::io;

impl<L: TypeList, S: AllocStrategy<L>> Unpin for Vari<L, S> {}

impl<L: CloneImp + TypeList, S: AllocStrategy<L>> Clone for Vari<L, S> {
    #[inline]
    fn clone(&self) -> Self {
        let (ptr, index) = self.split();
        unsafe {
            Self {
                tagged_ptr: L::clone::<L, S>(&self.strategy, ptr, L::ALIGN, index),
                strategy: self.strategy.clone(),
                mark: PhantomData,
            }
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        let (ptr, index) = self.split();
        let (src_ptr, src_index) = source.split();

        unsafe {
            L::clone_from::<L, S>(
                &self.strategy,
                ptr,
                index,
                src_ptr,
                src_index,
                src_index,
                &mut self.tagged_ptr,
            );
            self.strategy.clone_from(&source.strategy);
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

impl<L, S> fmt::Debug for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + for<'a, 'b> Apply<DebugImp<'a, 'b>, Output = fmt::Result>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ptr, index) = self.split();
        unsafe { L::apply(ptr, index, DebugImp(f)) }
    }
}

pub struct DisplayImp<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl<T: fmt::Display> Func<T> for DisplayImp<'_, '_> {
    type Output = fmt::Result;
    #[inline]
    fn call(self, value: &T) -> fmt::Result {
        value.fmt(self.0)
    }
}

impl<L, S> fmt::Display for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + for<'a, 'b> Apply<DisplayImp<'a, 'b>, Output = fmt::Result>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ptr, index) = self.split();
        unsafe { L::apply(ptr, index, DisplayImp(f)) }
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

impl<L, S> Eq for Vari<L, S>
where
    L: TypeList + Apply<PartialEqImp, Output = bool> + Apply<EqImp>,
    S: AllocStrategy<L>,
{
}
impl<L, S1, S2> PartialEq<Vari<L, S2>> for Vari<L, S1>
where
    S1: AllocStrategy<L>,
    S2: AllocStrategy<L>,
    L: TypeList + Apply<PartialEqImp, Output = bool>,
{
    #[inline]
    fn eq(&self, other: &Vari<L, S2>) -> bool {
        let (ptr, index) = self.split();
        let (optr, oindex) = other.split();
        index == oindex && unsafe { L::apply(ptr, index, PartialEqImp(optr)) }
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
impl<L, S1, S2> PartialOrd<Vari<L, S2>> for Vari<L, S1>
where
    S1: AllocStrategy<L>,
    S2: AllocStrategy<L>,
    L: TypeList
        + Apply<PartialEqImp, Output = bool>
        + Apply<EqImp>
        + Apply<PartialOrdImp, Output = Option<Ordering>>,
{
    #[inline]
    fn partial_cmp(&self, other: &Vari<L, S2>) -> Option<Ordering> {
        let (ptr, index) = self.split();
        let (optr, oindex) = other.split();
        match index.cmp(&oindex) {
            Ordering::Equal => unsafe { L::apply(ptr, index, PartialOrdImp(optr)) },
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
impl<L, S> Ord for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList
        + Apply<PartialEqImp, Output = bool>
        + Apply<EqImp>
        + Apply<PartialOrdImp, Output = Option<Ordering>>
        + Apply<OrdImp, Output = Ordering>,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let (ptr, index) = self.split();
        let (optr, oindex) = other.split();
        index
            .cmp(&oindex)
            .then_with(|| unsafe { L::apply(ptr, index, OrdImp(optr)) })
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
impl<L, S> Hash for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + for<'a> Apply<HashImp<'a>>,
{
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply(ptr, index, HashImp(hasher));
        }
    }
}

pub enum HasherImp<'a> {
    Bytes(&'a [u8]),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
}
impl<T: Hasher> Func<T> for HasherImp<'_> {
    type Output = ();
    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        match self {
            HasherImp::Bytes(bytes) => value.write(bytes),
            HasherImp::U8(i) => value.write_u8(i),
            HasherImp::U16(i) => value.write_u16(i),
            HasherImp::U32(i) => value.write_u32(i),
            HasherImp::U64(i) => value.write_u64(i),
            HasherImp::U128(i) => value.write_u128(i),
            HasherImp::Usize(i) => value.write_usize(i),
            HasherImp::I8(i) => value.write_i8(i),
            HasherImp::I16(i) => value.write_i16(i),
            HasherImp::I32(i) => value.write_i32(i),
            HasherImp::I64(i) => value.write_i64(i),
            HasherImp::I128(i) => value.write_i128(i),
            HasherImp::Isize(i) => value.write_isize(i),
        }
    }
}
pub struct HasherFinishImp(());
impl<T: Hasher> Func<T> for HasherFinishImp {
    type Output = u64;
    #[inline]
    fn call(self, value: &T) -> Self::Output {
        value.finish()
    }
}
impl<L, S> Hasher for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + for<'a> Apply<HasherImp<'a>> + Apply<HasherFinishImp, Output = u64>,
{
    fn finish(&self) -> u64 {
        let (ptr, index) = self.split();
        unsafe { L::apply(ptr, index, HasherFinishImp(())) }
    }
    fn write(&mut self, bytes: &[u8]) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::Bytes(bytes));
        }
    }
    fn write_u8(&mut self, i: u8) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::U8(i));
        }
    }
    fn write_u16(&mut self, i: u16) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::U16(i));
        }
    }
    fn write_u32(&mut self, i: u32) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::U32(i));
        }
    }
    fn write_u64(&mut self, i: u64) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::U64(i));
        }
    }
    fn write_u128(&mut self, i: u128) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::U128(i));
        }
    }
    fn write_usize(&mut self, i: usize) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::Usize(i));
        }
    }
    fn write_i8(&mut self, i: i8) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::I8(i));
        }
    }
    fn write_i16(&mut self, i: i16) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::I16(i));
        }
    }
    fn write_i32(&mut self, i: i32) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::I32(i));
        }
    }
    fn write_i64(&mut self, i: i64) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::I64(i));
        }
    }
    fn write_i128(&mut self, i: i128) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::I128(i));
        }
    }
    fn write_isize(&mut self, i: isize) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, HasherImp::Isize(i));
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
pub struct ExtendImp<'a, I>(&'a mut dyn Iterator<Item = I>);
impl<T: Extend<I>, I> Func<T> for ExtendImp<'_, I> {
    type Output = ();
    fn call_mut(self, value: &mut T) -> Self::Output {
        value.extend(self.0)
    }
}

impl<L, S, Item> Iterator for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>,
{
    type Item = Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, IteratorImp(None)) }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (ptr, index) = self.split();
        unsafe { L::apply(ptr, index, IteratorSizeImp(())) }
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, IteratorImp(Some(n))) }
    }
}

impl<L, S, Item> DoubleEndedIterator for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<DoubleEndedIteratorImp, Output = Option<Item>>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, DoubleEndedIteratorImp(None)) }
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, DoubleEndedIteratorImp(Some(n))) }
    }
}

impl<L, S, Item> FusedIterator for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<FusedIteratorImp>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>,
{
}

impl<L, S, Item> ExactSizeIterator for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList
        + Apply<IteratorImp, Output = Option<Item>>
        + Apply<ExactSizeIteratorImp>
        + Apply<IteratorSizeImp, Output = (usize, Option<usize>)>,
{
}

impl<L, S, A> Extend<A> for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + for<'a> Apply<ExtendImp<'a, A>>,
{
    fn extend<I: IntoIterator<Item = A>>(&mut self, iter: I) {
        let (ptr, index) = self.split();
        unsafe {
            L::apply_mut(ptr, index, ExtendImp(&mut iter.into_iter()));
        }
    }
}

pub struct FutureImp<'a, 'b>(&'a mut Context<'b>);
impl<T: Future + Unpin> Func<T> for FutureImp<'_, '_> {
    type Output = Poll<T::Output>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        Pin::new(value).poll(self.0)
    }
}

impl<L, S, Output> Future for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + for<'a, 'b> Apply<FutureImp<'a, 'b>, Output = Poll<Output>>,
{
    type Output = Output;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, FutureImp(cx)) }
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

#[cfg(feature = "std")]
pub enum ReadImp<'a> {
    ToEnd(&'a mut Vec<u8>),
    ToString(&'a mut String),
    Exact(&'a mut [u8]),
    Normal(&'a mut [u8]),
}

#[cfg(feature = "std")]
impl<T: io::Read> Func<T> for ReadImp<'_> {
    type Output = io::Result<usize>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        use ReadImp::*;

        fn zero<T>(_: T) -> usize {
            0
        }

        match self {
            ToEnd(buf) => value.read_to_end(buf),
            ToString(buf) => value.read_to_string(buf),
            Normal(buf) => value.read(buf),
            Exact(buf) => value.read_exact(buf).map(zero),
        }
    }
}

#[cfg(feature = "std")]
impl<L, S> io::Read for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + for<'a> Apply<ReadImp<'a>, Output = io::Result<usize>>,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, ReadImp::Normal(buf)) }
    }

    #[inline]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, ReadImp::ToEnd(buf)) }
    }

    #[inline]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, ReadImp::ToString(buf)) }
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, ReadImp::Exact(buf)).map(drop) }
    }
}

#[cfg(feature = "std")]
pub struct WriteBaseImp<'a>(&'a [u8]);

#[cfg(feature = "std")]
pub enum WriteExtImp<'a> {
    Flush,
    All(&'a [u8]),
    Fmt(fmt::Arguments<'a>),
}

#[cfg(feature = "std")]
impl<T: io::Write> Func<T> for WriteBaseImp<'_> {
    type Output = io::Result<usize>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        value.write(self.0)
    }
}

#[cfg(feature = "std")]
impl<T: io::Write> Func<T> for WriteExtImp<'_> {
    type Output = io::Result<()>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        match self {
            WriteExtImp::Flush => value.flush(),
            WriteExtImp::All(buf) => value.write_all(buf),
            WriteExtImp::Fmt(fmt) => value.write_fmt(fmt),
        }
    }
}

#[cfg(feature = "std")]
impl<L, S> io::Write for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList
        + for<'a> Apply<WriteBaseImp<'a>, Output = io::Result<usize>>
        + for<'a> Apply<WriteExtImp<'a>, Output = io::Result<()>>,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, WriteBaseImp(buf)) }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, WriteExtImp::Flush) }
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, WriteExtImp::All(buf)) }
    }

    #[inline]
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, WriteExtImp::Fmt(fmt)) }
    }
}

#[cfg(feature = "std")]
pub struct SeekImp(io::SeekFrom);

#[cfg(feature = "std")]
impl<T: io::Seek> Func<T> for SeekImp {
    type Output = io::Result<u64>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        value.seek(self.0)
    }
}

#[cfg(feature = "std")]
impl<L, S> io::Seek for Vari<L, S>
where
    S: AllocStrategy<L>,
    L: TypeList + Apply<SeekImp, Output = io::Result<u64>>,
{
    #[inline]
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let (ptr, index) = self.split();
        unsafe { L::apply_mut(ptr, index, SeekImp(pos)) }
    }
}

#[cfg(feature = "std")]
pub struct BufReadFillImp<'a>(PhantomData<&'a ()>);
#[cfg(feature = "std")]
impl<'a, T: 'a + io::BufRead> Func<T> for BufReadFillImp<'a> {
    type Output = io::Result<&'a [u8]>;

    #[inline]
    fn call_raw(self, value: *mut T) -> Self::Output {
        unsafe { (&mut *value).fill_buf() }
    }
}

#[cfg(feature = "std")]
pub struct BufReadConsumeImp(usize);
#[cfg(feature = "std")]
impl<'a, T: 'a + io::BufRead> Func<T> for BufReadConsumeImp {
    type Output = ();

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        value.consume(self.0)
    }
}

#[cfg(feature = "std")]
pub enum BufReadExtImp<'a> {
    Until { byte: u8, buf: &'a mut Vec<u8> },
    Line { buf: &'a mut String },
}

#[cfg(feature = "std")]
impl<'a, T: 'a + io::BufRead> Func<T> for BufReadExtImp<'_> {
    type Output = io::Result<usize>;

    #[inline]
    fn call_mut(self, value: &mut T) -> Self::Output {
        match self {
            BufReadExtImp::Until { byte, buf } => value.read_until(byte, buf),
            BufReadExtImp::Line { buf } => value.read_line(buf),
        }
    }
}

#[cfg(feature = "std")]
impl<L, S> io::BufRead for Vari<L, S>
where
    Self: io::Read,
    S: AllocStrategy<L>,
    L: TypeList
        + for<'a> Apply<BufReadFillImp<'a>, Output = io::Result<&'a [u8]>>
        + Apply<BufReadConsumeImp>
        + for<'a> Apply<BufReadExtImp<'a>, Output = io::Result<usize>>,
{
    #[inline]
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        let (ptr, index) = Vari::split(self);
        unsafe { L::apply_raw(ptr, index, BufReadFillImp(PhantomData)) }
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        let (ptr, index) = Vari::split(self);
        unsafe {
            L::apply_mut(ptr, index, BufReadConsumeImp(amt));
        }
    }

    #[inline]
    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        let (ptr, index) = Vari::split(self);
        unsafe { L::apply_mut(ptr, index, BufReadExtImp::Until { byte, buf }) }
    }

    #[inline]
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        let (ptr, index) = Vari::split(self);
        unsafe { L::apply_mut(ptr, index, BufReadExtImp::Line { buf }) }
    }
}

#[cfg(feature = "std")]
pub struct ErrorImp<'a>(PhantomData<&'a ()>);

#[cfg(feature = "std")]
impl<'a, T: 'a + Error> Func<T> for ErrorImp<'a> {
    type Output = Option<&'a (dyn Error + 'static)>;

    #[inline]
    fn call_raw(self, value: *mut T) -> Self::Output {
        unsafe { (*value).source() }
    }
}

#[cfg(feature = "std")]
impl<L, S> Error for Vari<L, S>
where
    Self: fmt::Debug + fmt::Display,
    S: AllocStrategy<L>,
    L: TypeList + for<'a> Apply<ErrorImp<'a>, Output = Option<&'a (dyn Error + 'static)>>,
{
    fn cause(&self) -> Option<&dyn Error> {
        let (ptr, index) = self.split();
        unsafe { L::apply_raw(ptr, index, ErrorImp(PhantomData)) }
    }
}
pub trait UnpinTuple {}
impl<T: Apply<UnpinTupleImp>> UnpinTuple for T {}

pub struct UnpinTupleImp(());
impl<T: Unpin> Func<T> for UnpinTupleImp {
    type Output = ();
}
