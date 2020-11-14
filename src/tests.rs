use super::{match_any, tlist, vari};

use std::boxed::Box;

use mockalloc::Mockalloc;
use std::alloc::System;

#[global_allocator]
static ALLOC: Mockalloc<System> = Mockalloc(System);

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn create_new() {
    type _Vari = vari!(u8, Box<i32>);
    let _ = _Vari::new(10);
    let _ = _Vari::new(Box::new(0));
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn get() {
    type _Vari = vari!(u8, Box<i32>);
    let x = _Vari::new(10);

    assert_eq!(*x.get::<u8, _>(), 10);
    assert!(x.try_get::<Box<i32>, _>().is_none());

    // NOTE: doesn't compile
    // assert!(x.try_get::<u32, _>().is_none());
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn set() {
    type _Vari = vari!(u8, i8, Box<u32>);
    let mut x = _Vari::new(0xae_u8);

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
    type _Vari = vari!(u8, i8, u32);
    let w = _Vari::new(0xae_u8);
    let x = _Vari::new(0xad_u8);
    let y = _Vari::new(-0x72_i8);
    let z = _Vari::new(0xabcdef01_u32);

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
    type _Vari = vari!(u8, i8, u32);
    let w = _Vari::new(0xae_u8);
    let x = w.clone();
    let y = _Vari::new(0x72_u8);
    let z = _Vari::new(-0x72_i8);

    assert_eq!(w.tag(), _Vari::tag_for::<u8, _>());
    assert_eq!(x.tag(), _Vari::tag_for::<u8, _>());
    assert_eq!(y.tag(), _Vari::tag_for::<u8, _>());
    assert_eq!(z.tag(), _Vari::tag_for::<i8, _>());

    assert_eq!(w.tag(), x.tag());
    assert_eq!(w.tag(), y.tag());
    assert_ne!(w.tag(), z.tag());
    assert_ne!(y.tag(), z.tag());

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
    type _VariSub = vari!(i32);
    type _Vari = vari!(i32, u32);
    type _Vari2 = vari!(u32, i32);
    type _VariSup = vari!(u32, i32, u8);

    let x = _Vari::new(212_u32);
    let x: _VariSup = x.into_super_set();

    assert_eq!(*x.get::<u32, _>(), 212);

    let x: _Vari2 = x.try_into_subset().unwrap();

    assert_eq!(*x.get::<u32, _>(), 212);
    x.try_into_subset::<tlist!(i32), _>().unwrap_err();
}

#[cfg_attr(miri, test)]
#[cfg_attr(not(miri), mockalloc::test)]
fn match_any() {
    struct A(u8);
    struct B(u8);
    struct C(u8);

    let bx = <vari!(A, B, C)>::new(C(0));

    match_any!(match bx.into_inner() => {
        _ => panic!(),
        _ => panic!(),
        _ => ()
    });
}

#[test]
fn no_alloc() {
    let info = mockalloc::record_allocs(|| {
        <vari!((), u8)>::new(());
    });

    assert_eq!(info.num_allocs(), 0);
    assert_eq!(info.num_frees(), 0);
}
