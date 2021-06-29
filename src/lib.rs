#![feature(ptr_internals)]
#![feature(allocator_api)]
mod drain;
mod raw;

use drain::Drain;
use raw::{RawValIter, RawVec};
use std::{
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    ptr,
};

/// Simplified macro for vec creation.
/// # Example
/// ```
/// use vec::custom_vec;
/// let vec = custom_vec![1, 2, 3];
/// assert_eq!(vec.len(), 3);
/// ```
#[macro_export]
macro_rules! custom_vec {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec = Vec::new();
            $(
                temp_vec.push($x);
            )*
            temp_vec
        }
    };
}

/// Contiguous, dynamically-sized set of elements of any type.
#[derive(Debug)]
pub struct Vec<T> {
    /// Items in the vector
    pub len: usize,
    /// Pointer to Vector's RawPointer
    buf: RawVec<T>,
}

/// Coerces a `Vec` into an iterator.
pub struct IntoIter<T> {
    _buf: RawVec<T>,
    iter: RawValIter<T>,
}

impl<T> Vec<T> {
    fn ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }

    fn cap(&self) -> usize {
        self.buf.cap
    }

    /// Creates a new Vector with size 0 (unallocated).
    /// # Example
    /// ```
    /// let vec: Vec<i32> = Vec::new();
    /// assert_eq!(vec.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            buf: RawVec::new(),
            len: 0,
        }
    }

    /// Pushes an element to the end of the vector.
    /// # Example
    /// ```
    /// use vec::custom_vec;
    /// let mut vec = custom_vec![5, 4, 3, 2];
    /// vec.push(1);
    /// assert_eq!(custom_vec![5, 4, 3, 2, 1], vec);
    /// ```
    pub fn push(&mut self, elem: T) {
        if self.len == self.cap() {
            self.buf.grow()
        };

        unsafe {
            ptr::write(self.ptr().offset(self.len as isize), elem);
        }

        self.len += 1;
    }

    /// Removes the last element of the vector and returns it, or `None` if the vector is empty.
    /// # Example
    /// ```
    /// use vec::custom_vec;
    /// let mut vec = custom_vec![1, 2];
    /// let pop = vec.pop().unwrap();
    ///
    /// assert_eq!(pop, 2);
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr().offset(self.len as isize))) }
        }
    }

    /// Inserts an element at a given index, shifting all the elements to the right.
    /// # Panics
    /// This function will panic if the index is out of bounds (>= length).
    /// # Example
    /// ```
    /// use vec::{Vec, custom_vec};
    /// let mut vec = custom_vec![1, 2];
    /// vec.insert(1, 3);
    /// assert_eq!(custom_vec![1, 3, 2], vec);
    /// ```
    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "Index out of bounds");

        if self.cap() == self.len {
            self.buf.grow();
        }

        unsafe {
            if index < self.len {
                // ptr::copy(source, dest, count) > Copy from 'source' to 'dest' 'count' elements
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

    /// Removes an element from a given index, shifting all the elements to the left.
    /// # Panics
    /// This function will panic if the index is out of bounds.
    /// # Example
    /// ```
    /// use vec::{Vec, custom_vec};
    /// # fn main() {
    /// let mut vec = custom_vec![1];
    /// vec.remove(0);
    /// assert_eq!(vec.len(), 0);
    /// # }
    /// ```
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

    /// Consumes Self into an iterator.
    /// # Example
    /// ```
    /// use vec::custom_vec;
    /// let v = custom_vec![1, 2, 3];
    /// let mut iter = v.into_iter();
    /// assert_eq!(Some(1), iter.next());
    /// assert_eq!(Some(2), iter.next());
    /// assert_eq!(Some(3), iter.next());
    /// assert_eq!(None, iter.next());
    /// ```
    pub fn into_iter(self) -> IntoIter<T> {
        unsafe {
            let iter = RawValIter::new(&self);

            let buf = ptr::read(&self.buf);
            mem::forget(self);

            IntoIter { iter, _buf: buf }
        }
    }

    /// Creates a draining iterator that removes the specified range in the vector and yields the removed items.
    /// # Example
    /// ```
    /// use vec::custom_vec;
    /// let mut vec = custom_vec![1, 2, 3];
    /// let mut iter = vec.drain(..);
    /// assert_eq!(Some(1), iter.next());
    /// assert_eq!(Some(2), iter.next());
    /// assert_eq!(Some(3), iter.next());
    /// assert_eq!(None, iter.next());
    /// ```
    pub fn drain(&mut self) -> Drain<T> {
        unsafe {
            let iter = RawValIter::new(&self);

            self.len = 0;

            Drain {
                iter,
                vec: PhantomData,
            }
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

impl<T: PartialEq> PartialEq for Vec<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (i, el) in self.iter().enumerate() {
            if *other.get(i).unwrap() == *el {
                continue;
            } else {
                return false;
            }
        }
        return true;
    }
}
impl<T: PartialEq> Eq for Vec<T> {}
