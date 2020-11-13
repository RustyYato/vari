use core::alloc::Layout;
use core::ops::*;
use core::pin::Pin;
use core::ptr::NonNull;

pub trait TypeList: SizeClass + Repr {}
impl<T: SizeClass + Repr> TypeList for T {}

#[inline(always)]
unsafe fn unreachable_unchecked() -> ! {
    #[cfg(debug_assertions)]
    unreachable!();
    #[cfg(not(debug_assertions))]
    core::hint::unreachable_unchecked()
}

pub enum CNil {}
pub enum CoProd<A, B> {
    Item(A),
    Rest(B),
}

#[repr(C)]
pub struct Nil;
#[repr(C)]
pub struct Cons<A, B>(A, B);
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ReprItem {
    pub layout: Layout,
    pub drop_in_place: unsafe fn(*mut ()),
}

pub struct Z;
pub struct S<N>(N);

pub struct DeallocOnDrop(pub *mut (), pub Layout);
impl Drop for DeallocOnDrop {
    #[inline]
    fn drop(&mut self) {
        if self.1.size() != 0 {
            unsafe { std::alloc::dealloc(self.0.cast(), self.1) }
        }
    }
}

#[inline(always)]
pub const unsafe fn layout<T>(align: usize) -> Layout {
    #[inline(always)]
    const fn max(x: usize, y: usize) -> usize {
        if x > y {
            x
        } else {
            y
        }
    }
    Layout::from_size_align_unchecked(
        core::mem::size_of::<T>(),
        max(core::mem::align_of::<T>(), align),
    )
}

#[inline(always)]
pub unsafe fn repr<T: TypeList>(index: usize) -> ReprItem {
    let repr = T::REPR;
    let repr = &repr as *const _ as *const ReprItem;
    let mut repr = repr.add(index).read();
    repr.layout = T::layout(index, T::ALIGN);
    repr
}

pub fn new_with<T, F: FnOnce() -> T>(value: F, align: usize, index: usize) -> NonNull<()> {
    assert!(align.is_power_of_two());
    let layout = unsafe { layout::<T>(align) };

    let ptr = if layout.size() == 0 {
        let ptr = NonNull::<T>::dangling().as_ptr() as usize;
        (ptr & !(align - 1) | align) as *mut u8
    } else {
        let ptr = unsafe { std::alloc::alloc(layout) };

        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }

        ptr
    };

    unsafe {
        // dealloc if value fn panics
        let dealloc = DeallocOnDrop(ptr.cast(), layout);
        ptr.cast::<T>().write(value());
        core::mem::forget(dealloc);
    }

    let ptr = ptr as usize;
    let ptr = ptr | index;
    let ptr = ptr as *mut ();

    unsafe { NonNull::new_unchecked(ptr) }
}

pub unsafe fn destroy<T: TypeList>(ptr: *mut (), index: usize) {
    let layout = T::layout(index, T::ALIGN);
    let _dealloc = DeallocOnDrop(ptr, layout);
    T::drop_in_place(ptr, index);
}

pub unsafe trait Tuple {
    const COUNT: u64;
}

unsafe impl Tuple for CNil {
    const COUNT: u64 = 0;
}

unsafe impl<A, R: Tuple> Tuple for CoProd<A, R> {
    const COUNT: u64 = R::COUNT + 1;
}

pub unsafe trait SizeClass {
    const SIZE_CLASS: u32;
    const ALIGN: usize = 1 << Self::SIZE_CLASS;
}

unsafe impl SizeClass for CNil {
    const SIZE_CLASS: u32 = 0;
}

unsafe impl<T, R> SizeClass for CoProd<T, R>
where
    Self: Tuple,
{
    const SIZE_CLASS: u32 = 63 - (Self::COUNT * 2 - 1).leading_zeros();
}

pub trait Seal {}
pub trait Peano: Seal {
    const VALUE: usize;
}
impl Seal for Z {}
impl<N: Seal> Seal for S<N> {}
impl Peano for Z {
    const VALUE: usize = 0;
}
impl<N: Peano> Peano for S<N> {
    const VALUE: usize = N::VALUE + 1;
}

pub unsafe trait Repr {
    const REPR: Self::Representation;
    type Representation;

    unsafe fn layout(index: usize, align: usize) -> Layout;
    unsafe fn drop_in_place(ptr: *mut (), index: usize);
}

unsafe impl Repr for CNil {
    type Representation = Nil;
    const REPR: Self::Representation = Nil;

    unsafe fn layout(_: usize, _: usize) -> Layout {
        unreachable_unchecked()
    }

    unsafe fn drop_in_place(_: *mut (), _: usize) {
        unreachable_unchecked()
    }
}

unsafe impl<T, B: Repr> Repr for CoProd<T, B> {
    type Representation = Cons<ReprItem, B::Representation>;
    const REPR: Self::Representation = {
        unsafe fn drop_in_place<T>(ptr: *mut ()) {
            ptr.cast::<T>().drop_in_place()
        }

        Cons(
            ReprItem {
                drop_in_place: drop_in_place::<T>,
                layout: Layout::new::<T>(),
            },
            B::REPR,
        )
    };

    unsafe fn layout(index: usize, align: usize) -> Layout {
        if index == 0 {
            Layout::from_size_align_unchecked(
                core::mem::size_of::<T>(),
                core::mem::align_of::<T>().max(align),
            )
        } else {
            B::layout(index.wrapping_sub(1), align)
        }
    }

    unsafe fn drop_in_place(ptr: *mut (), index: usize) {
        if index == 0 {
            ptr.cast::<T>().drop_in_place();
        } else {
            B::drop_in_place(ptr, index.wrapping_sub(1))
        }
    }
}

pub unsafe trait Contains<T, N>: Tuple {}
unsafe impl<T, R> Contains<T, Z> for CoProd<T, R> where Self: Tuple {}
unsafe impl<T, R: Contains<T, N>, U, N> Contains<T, S<N>> for CoProd<U, R> where Self: Tuple {}

pub trait IntoInner: Tuple {
    #[doc(hidden)]
    unsafe fn _into_inner(ptr: *mut (), index: usize) -> Self;
}

pub trait GetAny<'a>: Tuple {
    type Ref: 'a;
    type RefMut: 'a;

    type PinRef: 'a;
    type PinRefMut: 'a;

    #[doc(hidden)]
    unsafe fn _get_any(ptr: *const (), index: usize) -> Self::Ref;
    #[doc(hidden)]
    unsafe fn _get_any_mut(ptr: *mut (), index: usize) -> Self::RefMut;

    #[doc(hidden)]
    unsafe fn _pin_get_any(ptr: *const (), index: usize) -> Self::PinRef;
    #[doc(hidden)]
    unsafe fn _pin_get_any_mut(ptr: *mut (), index: usize) -> Self::PinRefMut;
}

impl IntoInner for CNil {
    #[inline(always)]
    unsafe fn _into_inner(_: *mut (), _: usize) -> Self {
        unreachable_unchecked()
    }
}

impl GetAny<'_> for CNil {
    type Ref = CNil;
    type RefMut = CNil;
    type PinRef = CNil;
    type PinRefMut = CNil;

    #[inline(always)]
    unsafe fn _get_any(_: *const (), _: usize) -> Self::Ref {
        unreachable_unchecked()
    }

    #[inline(always)]
    unsafe fn _get_any_mut(_: *mut (), _: usize) -> Self::RefMut {
        unreachable_unchecked()
    }

    #[inline(always)]
    unsafe fn _pin_get_any(_: *const (), _: usize) -> Self::PinRef {
        unreachable_unchecked()
    }

    #[inline(always)]
    unsafe fn _pin_get_any_mut(_: *mut (), _: usize) -> Self::PinRefMut {
        unreachable_unchecked()
    }
}

impl<A, B: IntoInner> IntoInner for CoProd<A, B> {
    #[inline(always)]
    unsafe fn _into_inner(ptr: *mut (), index: usize) -> Self {
        if index == 0 {
            Self::Item(ptr.cast::<A>().read())
        } else {
            Self::Rest(B::_into_inner(ptr, index.wrapping_sub(1)))
        }
    }
}

impl<'a, A: 'a, B: GetAny<'a>> GetAny<'a> for CoProd<A, B>
where
    Self: Tuple,
{
    type Ref = CoProd<&'a A, B::Ref>;
    type RefMut = CoProd<&'a mut A, B::RefMut>;
    type PinRef = CoProd<Pin<&'a A>, B::PinRef>;
    type PinRefMut = CoProd<Pin<&'a mut A>, B::PinRefMut>;

    #[inline]
    unsafe fn _get_any(ptr: *const (), index: usize) -> Self::Ref {
        if index == 0 {
            CoProd::Item(&*(ptr as *const A))
        } else {
            CoProd::Rest(B::_get_any(ptr, index.wrapping_sub(1)))
        }
    }

    #[inline]
    unsafe fn _get_any_mut(ptr: *mut (), index: usize) -> Self::RefMut {
        if index == 0 {
            CoProd::Item(&mut *(ptr as *mut A))
        } else {
            CoProd::Rest(B::_get_any_mut(ptr, index.wrapping_sub(1)))
        }
    }

    #[inline]
    unsafe fn _pin_get_any(ptr: *const (), index: usize) -> Self::PinRef {
        if index == 0 {
            CoProd::Item(Pin::new_unchecked(&*(ptr as *const A)))
        } else {
            CoProd::Rest(B::_pin_get_any(ptr, index.wrapping_sub(1)))
        }
    }

    #[inline]
    unsafe fn _pin_get_any_mut(ptr: *mut (), index: usize) -> Self::PinRefMut {
        if index == 0 {
            CoProd::Item(Pin::new_unchecked(&mut *(ptr as *mut A)))
        } else {
            CoProd::Rest(B::_pin_get_any_mut(ptr, index.wrapping_sub(1)))
        }
    }
}

pub unsafe trait CloneImp: Sized {
    unsafe fn clone(ptr: *const (), align: usize, index: usize) -> NonNull<()>;
    unsafe fn clone_from<T: TypeList>(
        ptr: *mut (),
        index: usize,
        src_ptr: *const (),
        src_index: usize,
        orig_src_index: usize,
        tagged_ptr: &mut NonNull<()>,
    );
}

unsafe impl CloneImp for CNil {
    #[inline(always)]
    unsafe fn clone(_: *const (), _: usize, _: usize) -> NonNull<()> {
        unreachable_unchecked()
    }

    #[inline(always)]
    unsafe fn clone_from<T: TypeList>(
        _: *mut (),
        _: usize,
        _: *const (),
        _: usize,
        _: usize,
        _: &mut NonNull<()>,
    ) {
        unreachable_unchecked()
    }
}

unsafe impl<T: Clone, R: CloneImp> CloneImp for CoProd<T, R> {
    #[inline]
    unsafe fn clone(ptr: *const (), align: usize, index: usize) -> NonNull<()> {
        if index == 0 {
            let this = &*(ptr as *const T);
            new_with(|| this.clone(), align, index)
        } else {
            R::clone(ptr, align, index.wrapping_sub(1))
        }
    }

    unsafe fn clone_from<L: TypeList>(
        ptr: *mut (),
        index: usize,
        src_ptr: *const (),
        src_index: usize,
        orig_src_index: usize,
        tagged_ptr: &mut NonNull<()>,
    ) {
        if src_index == 0 {
            let src_index = orig_src_index;
            let source = &*(src_ptr as *const T);
            if src_index == index {
                let this = &mut *(ptr as *mut T);
                this.clone_from(source);
            } else {
                struct WriteOnDrop<T>(*mut (), Option<T>);

                impl<T> Drop for WriteOnDrop<T> {
                    fn drop(&mut self) {
                        unsafe { self.0.cast::<T>().write(self.1.take().unwrap()) }
                    }
                }

                struct OnDrop(unsafe fn(*mut (), usize), *mut (), usize);

                impl Drop for OnDrop {
                    fn drop(&mut self) {
                        unsafe { (self.0)(self.1, self.2) }
                    }
                }

                let layout = L::layout(index, L::ALIGN);
                let src_layout = L::layout(src_index, L::ALIGN);

                if layout == src_layout {
                    let _write = WriteOnDrop(ptr, Some(source.clone()));
                    *tagged_ptr = NonNull::new_unchecked((ptr as usize | src_index) as *mut ());
                    L::drop_in_place(ptr, index);
                } else {
                    let _on_drop = OnDrop(destroy::<L>, ptr, index);
                    *tagged_ptr = new_with(|| source.clone(), src_layout.align(), src_index);
                }
            }
        } else {
            R::clone_from::<L>(
                ptr,
                index,
                src_ptr,
                src_index.wrapping_sub(1),
                orig_src_index,
                tagged_ptr,
            )
        }
    }
}

pub trait Func<T>: Sized {
    type Output;

    fn call(self, _: &T) -> Self::Output {
        unreachable!()
    }

    fn call_mut(self, _: &mut T) -> Self::Output {
        unreachable!()
    }

    fn call_raw(self, _: *mut T) -> Self::Output {
        unreachable!()
    }
}

pub unsafe trait Apply<F>: Sized {
    type Output;

    unsafe fn apply(ptr: *const (), index: usize, f: F) -> Self::Output;
    unsafe fn apply_mut(ptr: *mut (), index: usize, f: F) -> Self::Output;
    unsafe fn apply_raw(ptr: *mut (), index: usize, f: F) -> Self::Output;
}

pub unsafe trait ApplyImp<F, O>: Sized {
    unsafe fn apply(ptr: *const (), index: usize, f: F) -> O;
    unsafe fn apply_mut(ptr: *mut (), index: usize, f: F) -> O;
    unsafe fn apply_raw(ptr: *mut (), index: usize, f: F) -> O;
}

unsafe impl<F, O> ApplyImp<F, O> for CNil {
    #[inline(always)]
    unsafe fn apply(_: *const (), _: usize, _: F) -> O {
        unreachable_unchecked()
    }

    #[inline(always)]
    unsafe fn apply_mut(_: *mut (), _: usize, _: F) -> O {
        unreachable_unchecked()
    }

    #[inline(always)]
    unsafe fn apply_raw(_: *mut (), _: usize, _: F) -> O {
        unreachable_unchecked()
    }
}

unsafe impl<T, R: ApplyImp<F, F::Output>, F: Func<T>> ApplyImp<F, F::Output> for CoProd<T, R> {
    #[inline]
    unsafe fn apply(ptr: *const (), index: usize, f: F) -> F::Output {
        if index == 0 {
            f.call(&*(ptr as *const T))
        } else {
            R::apply(ptr, index.wrapping_sub(1), f)
        }
    }

    #[inline]
    unsafe fn apply_mut(ptr: *mut (), index: usize, f: F) -> F::Output {
        if index == 0 {
            f.call_mut(&mut *(ptr as *mut T))
        } else {
            R::apply_mut(ptr, index.wrapping_sub(1), f)
        }
    }

    #[inline]
    unsafe fn apply_raw(ptr: *mut (), index: usize, f: F) -> F::Output {
        if index == 0 {
            f.call_raw(ptr as *mut T)
        } else {
            R::apply_raw(ptr, index.wrapping_sub(1), f)
        }
    }
}

unsafe impl<T, R: ApplyImp<F, F::Output>, F: Func<T>> Apply<F> for CoProd<T, R> {
    type Output = F::Output;

    #[inline]
    unsafe fn apply(ptr: *const (), index: usize, f: F) -> F::Output {
        <Self as ApplyImp<F, F::Output>>::apply(ptr, index, f)
    }

    #[inline]
    unsafe fn apply_mut(ptr: *mut (), index: usize, f: F) -> F::Output {
        <Self as ApplyImp<F, F::Output>>::apply_mut(ptr, index, f)
    }

    #[inline]
    unsafe fn apply_raw(ptr: *mut (), index: usize, f: F) -> F::Output {
        <Self as ApplyImp<F, F::Output>>::apply_raw(ptr, index, f)
    }
}
