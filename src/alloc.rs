use core::alloc::Layout;

use crate::internals::TypeList;

pub unsafe trait AllocStrategy<L: TypeList>: Clone {
    #[inline]
    fn layout(&self, index: usize) -> Layout {
        assert!(L::COUNT > index);

        unsafe { self.layout_unchecked(index) }
    }

    #[inline]
    unsafe fn matches_type_layout<T>(&self, current: usize) -> bool {
        self.matches_layout(current, crate::internals::layout::<T>(L::ALIGN))
    }

    #[inline]
    unsafe fn matches_index_layout(&self, current: usize, other: usize) -> bool {
        self.matches_layout(current, self.layout_unchecked(other))
    }

    unsafe fn layout_unchecked(&self, index: usize) -> Layout;
    unsafe fn matches_layout(&self, current: usize, layout: Layout) -> bool;
}

pub type DefaultStrategy = BiggestVariant;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BiggestVariant;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Minimal;

unsafe impl<L: TypeList> AllocStrategy<L> for BiggestVariant {
    #[inline]
    unsafe fn layout_unchecked(&self, _: usize) -> Layout {
        L::layout_max_unchecked(Layout::from_size_align_unchecked(0, L::ALIGN))
    }

    #[inline]
    unsafe fn matches_layout(&self, _: usize, _: Layout) -> bool {
        true
    }
}

unsafe impl<L: TypeList> AllocStrategy<L> for Minimal {
    #[inline]
    unsafe fn layout_unchecked(&self, index: usize) -> Layout {
        L::layout_min(index, L::ALIGN)
    }

    #[inline]
    unsafe fn matches_layout(&self, current_index: usize, other: Layout) -> bool {
        AllocStrategy::<L>::layout_unchecked(self, current_index) == other
    }
}
