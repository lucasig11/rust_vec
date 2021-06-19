#![feature(ptr_internals)]
#![feature(allocator_api)]

use std::{
    alloc::{handle_alloc_error, Allocator, Global, Layout},
    mem,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull, Unique},
};

// Type for abstracting the repeated allocation, growth and free logics
pub struct RawVec<T> {
    // pointer to the allocation
    ptr: Unique<T>,
    // size of allocation
    cap: usize,
}

pub struct Vec<T> {
    buf: RawVec<T>,
    len: usize,
}

pub struct IntoIter<T> {
    _buf: RawVec<T>,
    start: *const T,
    end: *const T,
}

// Allocate, grow and free shared methods
impl<T> RawVec<T> {
    fn new() -> Self {
        assert!(mem::size_of::<T>() != 0, "zero-sized type not allowed..yet");
        Self {
            ptr: Unique::dangling(),
            cap: 0,
        }
    }

    fn grow(&mut self) {
        unsafe {
            let elem_size = mem::size_of::<T>();

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
        if self.cap != 0 {
            unsafe {
                let c: NonNull<T> = self.ptr.into();
                Global.deallocate(c.cast(), Layout::array::<T>(self.cap).unwrap())
            }
        }
    }
}

// Initialization methods
impl<T> Vec<T> {
    fn ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }

    fn cap(&self) -> usize {
        self.buf.cap
    }

    pub fn new() -> Self {
        Self {
            buf: RawVec::new(),
            len: 0,
        }
    }
}

// Data manipulation methods
impl<T> Vec<T> {
    pub fn push(&mut self, elem: T) {
        if self.len == self.cap() {
            self.buf.grow()
        };

        unsafe {
            ptr::write(self.ptr().offset(self.len as isize), elem);
        }

        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr().offset(self.len as isize))) }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "Index out of bounds");

        if self.cap() == self.len {
            self.buf.grow();
        }

        unsafe {
            if index < self.len {
                // ptr::copy(source, dest, count) > Copy from source to dest count elements
                ptr::copy(
                    self.ptr().offset(index as isize),
                    self.ptr().offset(index as isize + 1),
                    self.len - index,
                );
            }

            ptr::write(self.ptr().offset(index as isize), elem);
            self.len += 1;
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");

        unsafe {
            self.len -= 1;
            ptr::copy(
                self.ptr().offset(index as isize + 1),
                self.ptr().offset(index as isize),
                self.len - index,
            );
            ptr::read(self.ptr().offset(index as isize))
        }
    }
}

// Deref coertion (so our vector can be 'sliced')
impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr(), self.len) }
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr(), self.len) }
    }
}

// Deallocation
impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        if self.cap() != 0 {
            while let Some(_) = self.pop() {}
            // Deallocation is handled by RawVec
        }
    }
}

// Iterators
impl<T> Vec<T> {
    pub fn into_iter(self) -> IntoIter<T> {
        unsafe {
            let buf = ptr::read(&self.buf);
            let len = self.len;

            mem::forget(self);

            IntoIter {
                start: buf.ptr.as_ptr(),
                end: buf.ptr.as_ptr().offset(len as isize),
                _buf: buf,
            }
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let result = ptr::read(self.start);
                self.start = self.start.offset(1);
                Some(result)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end as usize - self.start as usize) / mem::size_of::<T>();
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                self.start = self.end.offset(-1);
                Some(ptr::read(self.end))
            }
        }
    }
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        // Ensure all elements are read
        for _ in &mut *self {}
    }
}

fn main() {
    let mut vec: Vec<i32> = Vec::new();
    
    vec.push(10);
    vec.push(11);
    vec.push(12);
    vec.push(13);
    vec.push(14);

    
    for i in vec.iter() {
        println!("iter {}", i)
    }

    let el = vec.pop().unwrap();
    println!("pop {} | new length {}", el, vec.len);
}
