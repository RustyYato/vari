use core::alloc::Layout;

use crate::internals::TypeList;

pub unsafe trait AllocStrategy<L: TypeList>: Clone {
    fn layout(&self, index: usize) -> Layout {
        assert!(L::COUNT > index);

        unsafe { self.layout_unchecked(index) }
    }

    unsafe fn layout_unchecked(&self, index: usize) -> Layout;

    unsafe fn matches_type_layout<T>(&self, current: usize) -> bool;
    unsafe fn matches_index_layout(&self, current: usize, other: usize) -> bool;
}

pub type DefaultStrategy = BiggestVariant;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BiggestVariant;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Minimal;

unsafe impl<L: TypeList> AllocStrategy<L> for BiggestVariant {
    unsafe fn layout_unchecked(&self, _: usize) -> Layout {
        L::layout_max_unchecked(Layout::from_size_align_unchecked(0, L::ALIGN))
    }

    unsafe fn matches_type_layout<T>(&self, _: usize) -> bool {
        true
    }

    unsafe fn matches_index_layout(&self, _: usize, _: usize) -> bool {
        true
    }
}

unsafe impl<L: TypeList> AllocStrategy<L> for Minimal {
    unsafe fn layout_unchecked(&self, index: usize) -> Layout {
        L::layout_min(index, L::ALIGN)
    }

    unsafe fn matches_type_layout<T>(&self, current_index: usize) -> bool {
        crate::internals::layout::<T>(L::ALIGN)
            == AllocStrategy::<L>::layout_unchecked(self, current_index)
    }

    unsafe fn matches_index_layout(&self, current_index: usize, other_index: usize) -> bool {
        AllocStrategy::<L>::layout_unchecked(self, current_index)
            == AllocStrategy::<L>::layout_unchecked(self, other_index)
    }
}
