use std::marker::PhantomData;
use std::time::Instant;
use std::alloc::{alloc, dealloc, Layout, handle_alloc_error, alloc_zeroed};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::mem::{size_of, align_of};

#[derive(Debug)]
pub enum ArenaError {
    BoundsExceeded,
    DoesNotExist,
    OutOfMemory,
    PoisonedLock,
}

#[derive(Debug)]
pub struct ArenaHandle<T> {
    index: usize,
    phantom_arena: PhantomData<T>
}
impl<T> ArenaHandle<T> {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            phantom_arena: PhantomData,
        }
    }
}
impl<T> Copy for ArenaHandle<T> {}
impl<T> Clone for ArenaHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
unsafe impl<T> Send for ArenaHandle<T> {}
unsafe impl<T> Sync for ArenaHandle<T> {}
// impl<T> std::fmt::Debug for ArenaHandle<T> where T: std::fmt::Debug {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct(format!("ArenaHandle<{}>", T::fmt(T, f))).finish()
//     }
// }


pub struct Arena<T> {
    memory: *mut RwLock<T>,
    allocated: *mut bool,
    pub length: usize,
    pub last_known_free: usize,
    pub count: usize,
 
    layout_memory: Layout,
    layout_allocated: Layout,
}


impl<T> std::fmt::Debug for Arena<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(
            unsafe {
                (0..self.length).map(|i| if *self.allocated.add(i) {Some(&*self.memory.add(i))} else { None })
            }
        ).finish()
    }
}

impl<T> Drop for Arena<T> {
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

impl<T> Arena<T> {
    //type MyArenaHandle =  ArenaHandle<T>;
    // length in number of T it can hold
    pub fn new(length: usize) -> Self {
        let layout_memory = Layout::from_size_align(size_of::<RwLock<T>>()*length, align_of::<RwLock<T>>()).unwrap();
        let layout_allocated = Layout::from_size_align(size_of::<bool>()*length, align_of::<bool>()).unwrap();

        println!("Arena allocation: {:?} bytes, {:?} entities", layout_memory.size(), layout_allocated.size());

        unsafe {
            let ptr_memory = alloc(layout_memory);
            let ptr_allocated = alloc_zeroed(layout_allocated);

            if ptr_memory.is_null() {
                handle_alloc_error(layout_memory);
            }
            if ptr_allocated.is_null() {
                handle_alloc_error(layout_allocated);
            }

            Arena {
                memory: ptr_memory as *mut RwLock<T>,
                allocated: ptr_allocated as *mut bool,
                length,
                last_known_free: 0,
                count: 0,

                layout_memory,
                layout_allocated,
            }
        }
    }

    fn new_handle(&self, index: usize) -> Result<ArenaHandle<T>, ArenaError> {
        if index >= self.length {
            return Err(ArenaError::BoundsExceeded)
        }
        if !unsafe{*self.allocated.add(index)} {
            return Err(ArenaError::DoesNotExist)
        }
        Ok(ArenaHandle::new(index))
    }

    // make new arena and load collection directly into arena contents
    // you still need to specify a maximum length for the arena
    pub fn from_iter<I>(length: usize, iterator: I) -> Self where I: IntoIterator<Item=T> {
        let mut arena = Arena::<T>::new(length);
        for (i, item) in iterator.into_iter().enumerate() {
            if i == length {
                panic!("Arena::from_iter is about to segfault because you didn't specify a high enough length")
            }
            unsafe {arena.overwrite(i, item);}
            arena.count += 1;
        }
        arena
    }

    pub fn iter(&self) -> ArenaIterator<T> {
        return ArenaIterator { i: 0, arena: self }
    }

    // writes an object to an index regardless of allocation status. does no bounds checking. use with care.
    unsafe fn overwrite(&mut self, index: usize, obj: T) {
        self.memory.add(index).write(RwLock::new(obj));
        self.allocated.add(index).write(true);
    }

    // create an object at the next available space - if no space is free, sad!
    pub fn create(&mut self, obj: T) -> Result<ArenaHandle<T>, ArenaError> {
        for i in self.last_known_free..self.length {
            if unsafe {self.allocated.add(i).read()} { continue; } // already allocated to that slot, keep going

            // we found one that was free!
            unsafe {
                self.memory.add(i).write(RwLock::new(obj));
                self.allocated.add(i).write(true);
            }

            self.count += 1;
            self.last_known_free += 1;
            return self.new_handle(i);
        }
        Err(ArenaError::OutOfMemory)
    }

    // destroy the object at a certain index so that space can be used again (e.g. entity dies)
    pub fn destroy(&mut self, handle: ArenaHandle<T>) -> Result<(), ArenaError> {
        if handle.index >= self.length {
            return Err(ArenaError::BoundsExceeded);
        }
        unsafe {
            if !self.allocated.add(handle.index).read() {
                return Err(ArenaError::DoesNotExist);
            }
            self.memory.add(handle.index).drop_in_place();
            self.allocated.add(handle.index).write(false);
        }
        self.last_known_free = handle.index;
        self.count -= 1;

        // Logging statement to indicate chunk destruction
        // println!("Destroying chunk at index {}", handle.index);

        Ok(())
    }

    // get the object at a certain index, wrapped in a RwLock
    pub fn fetch_lock(&self, handle: ArenaHandle<T>) -> Result<&RwLock<T>, ArenaError> {
        unsafe {Ok(&*self.memory.add(handle.index))}
    }

    pub fn read_lock(&self, handle: ArenaHandle<T>) -> Result<RwLockReadGuard<T>, ArenaError> {
        match self.fetch_lock(handle)?.read() {
            Ok(readable) => Ok(readable),
            Err(_) => Err(ArenaError::PoisonedLock)
        }
    }
    pub fn write_lock(&self, handle: ArenaHandle<T>) -> Result<RwLockWriteGuard<T>, ArenaError> {
        match self.fetch_lock(handle)?.write() {
            Ok(writable) => Ok(writable),
            Err(_) => Err(ArenaError::PoisonedLock)
        }
    }

    /*pub fn get_mut(&self, index: usize) -> Result<&mut T, &str> {
        self.check_allocated(index)?;
        unsafe {
            Ok(&mut *self.memory.add(index))
        }
    }*/
}


pub struct ArenaIterator<'a, T> {
    i: usize,
    arena: &'a Arena<T>,
}
impl<'a, T> Iterator for ArenaIterator<'a, T> {
    type Item = ArenaHandle<T>;
    fn next(&mut self) -> std::option::Option<Self::Item> {
        
        if self.i < self.arena.length {
            match self.arena.new_handle(self.i) {
                Ok(handle) => {self.i += 1; return Some(handle)}
                _ => {self.i += 1; return self.next()}
            }
        } else {
            return None;
        }
        
    }
}



/*
#[derive(Debug)]
struct Test<'a> {
    a: &'a str,
    b: i32,
}



#[test]
fn main() {
    let t = Test {a: "hello", b:1};
    //std::fmt::Debug::fmt(t);

    let mut a = Arena::<Test>::new(3);
    a.create(Test {
        a: "hello world 1",
        b: -5,
    }).unwrap();
    a.create(Test {
        a: "hello world 2",
        b: -3,
    }).unwrap();
    a.create(Test {
        a: "hello world 3",
        b: -1,
    }).unwrap();

    a.destroy(0);
    a.create(Test {
        a: "hello world 8",
        b: 100,
    }).unwrap();

    a.obtain(2).unwrap().write().unwrap().b = 7;
    a.destroy(1);

    for obj in a.iter() {
        println!("{:?}", obj);
    }

    println!("{:?}", a);
}
*/