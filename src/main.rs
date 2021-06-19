#![feature(ptr_internals)]
#![feature(allocator_api)]
mod raw;

use raw::{RawValIter, RawVec};
use std::{
    mem,
    ops::{Deref, DerefMut},
    ptr,
};

pub struct Vec<T> {
    buf: RawVec<T>,
    len: usize,
}

pub struct IntoIter<T> {
    _buf: RawVec<T>,
    iter: RawValIter<T>,
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

    pub fn into_iter(self) -> IntoIter<T> {
        unsafe {
            let iter = RawValIter::new(&self);

            let buf = ptr::read(&self.buf);
            mem::forget(self);

            IntoIter { iter, _buf: buf }
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

// Iterators
impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

// Deallocation (Drop trait -> https://doc.rust-lang.org/1.9.0/book/drop.html)
impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        if self.cap() != 0 {
            while let Some(_) = self.pop() {}
            // Deallocation is handled by RawVec
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
