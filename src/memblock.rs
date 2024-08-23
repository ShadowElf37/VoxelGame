use std::iter::Map;
use std::alloc::{alloc, dealloc, Layout, handle_alloc_error, alloc_zeroed};
use std::slice::Iter;
use std::mem::{size_of, align_of};


#[derive(Debug)]
pub enum MemoryError {
    BoundsExceeded,
    DoesNotExist,
}


pub struct MemoryBlock<T> {
    pub memory: *mut T,
    pub allocated: *mut bool,
    pub length: usize,
    layout_memory: Layout,
    layout_allocated: Layout,
}
impl<T> Drop for MemoryBlock<T> {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.length {
                std::ptr::drop_in_place(self.memory.add(i));
            }
            dealloc(self.memory as *mut u8, self.layout_memory);
            dealloc(self.allocated as *mut u8, self.layout_allocated);
        }
    }
}

impl<T> MemoryBlock<T> {
    // length in number of T it can hold
    pub fn new(length: usize) -> Self {
        let layout_memory = Layout::from_size_align(size_of::<T>()*length, align_of::<T>()).unwrap();
        let layout_allocated = Layout::from_size_align(size_of::<bool>()*length, align_of::<bool>()).unwrap();

        println!("Block allocation: {:?} bytes, {:?} objects", layout_memory.size(), layout_allocated.size());

        unsafe {
            let ptr_memory = alloc(layout_memory);
            let ptr_allocated = alloc_zeroed(layout_allocated);

            if ptr_memory.is_null() {
                handle_alloc_error(layout_memory);
            }
            if ptr_allocated.is_null() {
                handle_alloc_error(layout_allocated);
            }

            Self {
                memory: ptr_memory as *mut T,
                allocated: ptr_allocated as *mut bool,
                length,
                layout_memory,
                layout_allocated,
            }
        }
    }

    fn bounds_check(&self, index: usize) -> Result<(), MemoryError> {
        if index >= self.length {
            return Err(MemoryError::BoundsExceeded)
        }
        Ok(())
    }
    fn allocated_check(&self, index: usize) -> Result<(), MemoryError> {
        if !unsafe {self.allocated.add(index).read()} {
            return Err(MemoryError::DoesNotExist)
        }
        Ok(())
    }

    pub unsafe fn is_allocated(&self, index: usize) -> bool {
        unsafe { self.allocated.add(index).read() }
    }

    pub unsafe fn drop(&mut self, index: usize) {
        //self.bounds_check(index)?;
        //self.allocated_check(index)?;
        unsafe {
            self.memory.add(index).drop_in_place();
            self.allocated.add(index).write(false);
        }
        //Ok(())
    }
    // YOU MUST CALL DROP IF OVERWRITING SOMETHING
    pub unsafe fn write(&mut self, index: usize, object: T) {
        //self.bounds_check(index)?;
        unsafe {
            self.memory.add(index).write(object);
            self.allocated.add(index).write(true);
        }
        //Ok(())
    }
    pub unsafe fn read(&self, index: usize) -> &T {// -> Result<T, MemoryError> {
        //self.bounds_check(index)?;
        //self.allocated_check(index)?;
        unsafe {&*self.memory.add(index)}
    }
    pub fn get_ptr(&self, index: usize) -> Result<*mut T, MemoryError> {
        self.bounds_check(index)?;
        self.allocated_check(index)?;
        Ok(unsafe {self.memory.add(index)})
    }
    pub unsafe fn get_ptr_unchecked(&self, index: usize) -> *mut T {
        unsafe {self.memory.add(index)}
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        unsafe {std::slice::from_raw_parts(self.memory, self.length)}
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        unsafe{self.as_slice().iter().enumerate().filter(|(i, _)| self.is_allocated(*i)).map(|(_, x)| x)}
    }
}