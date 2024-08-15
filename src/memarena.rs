use std::alloc::{alloc, dealloc, Layout, handle_alloc_error, alloc_zeroed};
use std::sync::RwLock;
use std::mem::{size_of, align_of};

#[derive(Debug)]
pub enum ArenaError {
    BoundsExceeded,
    DoesNotExist,
    OutOfMemory,
}


pub struct Arena<T> {
    memory: *mut RwLock<T>,
    allocated: *mut bool,
    length: usize,
    last_known_free: usize,
    count: usize,
 
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
    // length in number of T it can hold
    pub fn new(length: usize) -> Arena<T> {
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

    // make new arena and load collection directly into arena contents
    // you still need to specify a maximum length for the arena
    pub fn from_iter<I>(length: usize, iterator: I) -> Arena<T> where I: IntoIterator<Item=T> {
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
    pub fn create(&mut self, obj: T) -> Result<usize, ArenaError> {
        for i in self.last_known_free..self.length {
            if unsafe {self.allocated.add(i).read()} { continue; } // already allocated to that slot, keep going

            // we found one that was free!
            unsafe {
                self.memory.add(i).write(RwLock::new(obj));
                self.allocated.add(i).write(true);
            }

            self.count += 1;
            self.last_known_free += 1;
            return Ok(i);
        }
        Err(ArenaError::OutOfMemory)
    }

    // destroy the object at a certain index so that space can be used again (e.g. entity dies)
    pub fn destroy(&mut self, index: usize) -> Result<(), ArenaError> {
        //we have to do this error checking manually because the borrow checker gets very angry about &mut
        if index >= self.length {
            return Err(ArenaError::BoundsExceeded);
        }
        if !unsafe{*self.allocated.add(index)} {
            return Err(ArenaError::DoesNotExist)
        }

        unsafe {
            self.memory.add(index).drop_in_place();
            self.allocated.add(index).write(false);
        }

        self.last_known_free = index;
        self.count -= 1;
        Ok(())
    }

    // get the object at a certain index, wrapped in a RwLock
    pub fn obtain(&self, index: usize) -> Result<&RwLock<T>, ArenaError> {
        if index >= self.length {
            return Err(ArenaError::BoundsExceeded)
        }
        if !unsafe{*self.allocated.add(index)} {
            return Err(ArenaError::DoesNotExist)
        }

        unsafe {
            Ok(&*self.memory.add(index))
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
    type Item = &'a RwLock<T>;
    fn next(&mut self) -> std::option::Option<<Self as Iterator>::Item> {
        
        if self.i < self.arena.length {
            match self.arena.obtain(self.i) {
                Ok(lock) => {self.i += 1; return Some(lock)}
                _ => {self.i += 1; return self.next()}
            }
        } else {
            return None;
        }
        
    }
}




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