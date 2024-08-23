use std::marker::PhantomData;
use std::sync::PoisonError;
use std::alloc::{alloc, dealloc, Layout, handle_alloc_error, alloc_zeroed};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::mem::{size_of, align_of};
use crate::memblock::*;

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
    memory: MemoryBlock<RwLock<T>>,
    pub last_known_free: usize,
    pub count: usize,
}


impl<T> std::fmt::Debug for Arena<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(
            unsafe {
                (0..self.memory.length).map(|i| if *self.memory.allocated.add(i) {Some(&*self.memory.memory.add(i))} else { None })
            }
        ).finish()
    }
}

impl<T> Arena<T> {
    //type MyArenaHandle =  ArenaHandle<T>;
    // length in number of T it can hold
    pub fn new(length: usize) -> Self {
        Arena {
            memory: MemoryBlock::new(length),
            last_known_free: 0,
            count: 0,
        }
    }

    fn new_handle(&self, index: usize) -> Result<ArenaHandle<T>, ArenaError> {
        if index >= self.memory.length {
            return Err(ArenaError::BoundsExceeded)
        }
        if !unsafe{self.memory.is_allocated(index)} {
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
        self.memory.write(index, RwLock::new(obj));
        self.memory.allocated.add(index).write(true);
    }

    // create an object at the next available space - if no space is free, sad!
    pub fn create(&mut self, obj: T) -> Result<ArenaHandle<T>, ArenaError> {
        for i in self.last_known_free..self.memory.length {
            if unsafe {self.memory.allocated.add(i).read()} { continue; } // already allocated to that slot, keep going

            // we found one that was free!
            unsafe {
                self.memory.write(i, RwLock::new(obj));
                self.memory.allocated.add(i).write(true);
            }

            self.count += 1;
            self.last_known_free += 1;
            return self.new_handle(i);
        }
        Err(ArenaError::OutOfMemory)
    }

    // destroy the object at a certain index so that space can be used again (e.g. entity dies)
    pub fn destroy(&mut self, handle: ArenaHandle<T>) -> Result<(), ArenaError> {
        unsafe {
            self.memory.drop(handle.index);
        }
        self.last_known_free = handle.index;
        self.count -= 1;
        Ok(())
    }

    // get the object at a certain index, wrapped in a RwLock
    pub fn fetch_lock(&self, handle: ArenaHandle<T>) -> Result<&RwLock<T>, ArenaError> {
        unsafe {Ok(&*self.memory.memory.add(handle.index))}
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

}


pub struct ArenaIterator<'a, T> {
    i: usize,
    arena: &'a Arena<T>,
}
impl<'a, T> Iterator for ArenaIterator<'a, T> {
    type Item = ArenaHandle<T>;
    fn next(&mut self) -> std::option::Option<Self::Item> {
        
        if self.i < self.arena.memory.length {
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