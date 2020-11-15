use vari::{match_any, tlist};

use std::boxed::Box;

use mockalloc::Mockalloc;
use std::alloc::System;

#[global_allocator]
static ALLOC: Mockalloc<System> = Mockalloc(System);

#[derive(Default, Clone)]
struct Strat(Box<i32>);

unsafe impl<L: vari::traits::TypeList> vari::traits::AllocStrategy<L> for Strat {
    unsafe fn layout_unchecked(&self, index: usize) -> core::alloc::Layout {
        vari::traits::AllocStrategy::<L>::layout_unchecked(&vari::alloc::BiggestVariant, index)
    }

    unsafe fn matches_layout(&self, current: usize, layout: core::alloc::Layout) -> bool {
        vari::traits::AllocStrategy::<L>::matches_layout(
            &vari::alloc::BiggestVariant,
            current,
            layout,
        )
    }
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn create_new() {
    type _Vari<S> = vari::Vari<tlist!(u8, Box<i32>), S>;
    let _ = _Vari::using_strategy(10, Strat::default());
    let _ = _Vari::using_strategy(Box::new(0), Strat::default());
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn get() {
    type _Vari<S> = vari::Vari<tlist!(u8, Box<i32>), S>;
    let x = _Vari::using_strategy(10, Strat::default());

    assert_eq!(*x.get::<u8, _>(), 10);
    assert!(x.try_get::<Box<i32>, _>().is_none());

    // NOTE: doesn't compile
    // assert!(x.try_get::<u32, _>().is_none());
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn set() {
    type _Vari<S> = vari::Vari<tlist!(u8, i8, Box<u32>), S>;
    let mut x = _Vari::using_strategy(0xae_u8, Strat::default());

    assert_eq!(*x.get::<u8, _>(), 0xae);

    // exact
    x.set(0x2f_u8);
    assert_eq!(*x.get::<u8, _>(), 0x2f);

    // layout identical
    x.set(-0xa_i8);
    assert_eq!(*x.get::<i8, _>(), -0xa);

    // different in every way
    x.set(Box::new(0xefda_u32));
    assert_eq!(**x.get::<Box<u32>, _>(), 0xefda);

    // overwrite
    x.set(-0xa_i8);
    assert_eq!(*x.get::<i8, _>(), -0xa);
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn clone() {
    type _Vari<S> = vari::Vari<tlist!(u8, i8, u32), S>;
    let w = _Vari::using_strategy(0xae_u8, Strat::default());
    let x = _Vari::using_strategy(0xad_u8, Strat::default());
    let y = _Vari::using_strategy(-0x72_i8, Strat::default());
    let z = _Vari::using_strategy(0xabcdef01_u32, Strat::default());

    let mut a = w.clone();
    assert_eq!(*a.get::<u8, _>(), 0xae);

    // exact
    a.clone_from(&x);
    assert_eq!(*a.get::<u8, _>(), 0xad);

    // layout identical
    a.clone_from(&y);
    assert_eq!(*a.get::<i8, _>(), -0x72);

    // different in every way
    a.clone_from(&z);
    assert_eq!(*a.get::<u32, _>(), 0xabcdef01);
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn eq() {
    type _Vari<S> = vari::Vari<tlist!(u8, i8, u32), S>;
    let w = _Vari::using_strategy(0xae_u8, Strat::default());
    let x = w.clone();
    let y = _Vari::using_strategy(0x72_u8, Strat::default());
    let z = _Vari::using_strategy(-0x72_i8, Strat::default());

    assert_eq!(w.index(), _Vari::<Strat>::index_of::<u8, _>());
    assert_eq!(x.index(), _Vari::<Strat>::index_of::<u8, _>());
    assert_eq!(y.index(), _Vari::<Strat>::index_of::<u8, _>());
    assert_eq!(z.index(), _Vari::<Strat>::index_of::<i8, _>());

    assert_eq!(w.index(), x.index());
    assert_eq!(w.index(), y.index());
    assert_ne!(w.index(), z.index());
    assert_ne!(y.index(), z.index());

    assert_eq!(w, w);
    assert_eq!(w, x);
    assert_ne!(w, y);
    assert_ne!(w, z);

    assert_eq!(x, w);
    assert_eq!(x, x);
    assert_ne!(x, y);
    assert_ne!(x, z);

    assert_ne!(y, w);
    assert_ne!(y, x);
    assert_eq!(y, y);
    assert_ne!(y, z);

    assert_ne!(z, w);
    assert_ne!(z, x);
    assert_ne!(z, y);
    assert_eq!(z, z);
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn into_superset() {
    type _VariSub<S> = vari::Vari<tlist!(i32), S>;
    type _Vari<S> = vari::Vari<tlist!(i32, u32), S>;
    type _Vari2<S> = vari::Vari<tlist!(u32, i32), S>;
    type _VariSup<S> = vari::Vari<tlist!(u32, i32, u8), S>;

    let x = _Vari::using_strategy(212_u32, Strat::default());
    let x: _VariSup<_> = x.into_superset();

    assert_eq!(*x.get::<u32, _>(), 212);

    let x: _Vari2<_> = x.try_into_subset().unwrap();

    assert_eq!(*x.get::<u32, _>(), 212);
    x.try_into_subset::<tlist!(i32), _>().unwrap_err();
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn match_any() {
    struct A(u8);
    struct B(u8);
    struct C(u8);

    let bx = vari::Vari::<tlist!(A, B, C), Strat>::using_strategy(C(0), Strat::default());

    match_any!(match bx.into_inner() => {
        _ => panic!(),
        _ => panic!(),
        _ => ()
    });
}
