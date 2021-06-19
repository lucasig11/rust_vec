use std::{
    alloc::{handle_alloc_error, Allocator, Global, Layout},
    mem,
    ptr::{self, NonNull, Unique},
};

// Type for abstracting the repeated allocation, growth and free logics
pub struct RawVec<T> {
    // pointer to the allocation
    pub ptr: Unique<T>,
    // size of allocation
    pub cap: usize,
}

// Type for abstracting iterators logic
pub struct RawValIter<T> {
    start: *const T,
    end: *const T,
}

// Allocate, grow and free shared methods
impl<T> RawVec<T> {
    pub fn new() -> Self {
        // !0 == usize::MAX
        let cap = if mem::size_of::<T>() == 0 { !0 } else { 0 };

        Self {
            ptr: Unique::dangling(),
            cap,
        }
    }

    pub fn grow(&mut self) {
        unsafe {
            let elem_size = mem::size_of::<T>();

            assert!(elem_size != 0, "capacity overflow");

            let (new_cap, ptr) = if self.cap == 0 {
                let ptr = Global.allocate(Layout::array::<T>(1).unwrap());
                (1, ptr)
            } else {
                let new_cap = 2 * self.cap;
                let old_num_bytes = self.cap * elem_size;

                assert!(
                    old_num_bytes <= (isize::MAX as usize) / 2,
                    "capacity overflow"
                );

                let c: NonNull<T> = self.ptr.into();
                let ptr = Global.grow(
                    c.cast(),
                    Layout::array::<T>(self.cap).unwrap(),
                    Layout::array::<T>(new_cap).unwrap(),
                );

                (new_cap, ptr)
            };

            // Out of memory
            if ptr.is_err() {
                handle_alloc_error(Layout::from_size_align_unchecked(
                    new_cap * elem_size,
                    mem::align_of::<T>(),
                ))
            }

            let ptr = ptr.unwrap();

            self.ptr = Unique::new_unchecked(ptr.as_ptr() as *mut _);
            self.cap = new_cap;
        }
    }
}

// RawVec Deallocation (Drop trait -> https://doc.rust-lang.org/1.9.0/book/drop.html)
impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        let elem_size = mem::size_of::<T>();

        // Don't free zero-sizes allocations
        if self.cap != 0 && elem_size != 0 {
            unsafe {
                let c: NonNull<T> = self.ptr.into();
                Global.deallocate(c.cast(), Layout::array::<T>(self.cap).unwrap())
            }
        }
    }
}

impl<T> RawValIter<T> {
    pub unsafe fn new(slice: &[T]) -> Self {
        Self {
            start: slice.as_ptr(),
            end: if mem::size_of::<T>() == 0 {
                ((slice.as_ptr() as usize) + slice.len()) as *const _
            } else if slice.len() == 0 {
                slice.as_ptr()
            } else {
                slice.as_ptr().offset(slice.len() as isize)
            },
        }
    }
}

impl<T> Iterator for RawValIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let result = ptr::read(self.start);
                self.start = if mem::size_of::<T>() == 0 {
                    (self.start as usize + 1) as *const _
                } else {
                    self.start.offset(1)
                };

                Some(result)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem_size = mem::size_of::<T>();

        let len =
            (self.end as usize - self.start as usize) / if elem_size == 0 { 1 } else { elem_size };

        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for RawValIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                self.start = if mem::size_of::<T>() == 0 {
                    (self.end as usize - 1) as *const _
                } else {
                    self.end.offset(-1)
                };

                Some(ptr::read(self.end))
            }
        }
    }
}
