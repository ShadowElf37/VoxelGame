use std::marker::PhantomData;
use std::sync::{Arc, RwLock, Mutex};

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

pub struct Arena<T> {
    items: Vec<Option<Arc<RwLock<T>>>>,
    allocated: Arc<Mutex<Vec<bool>>>,
    pub length: usize,
    pub last_known_free: usize,
    pub count: usize,
}

impl<T> std::fmt::Debug for Arena<T> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(
            self.items.iter().map(|item| item.as_ref())
        ).finish()
    }
}

impl<T> Clone for Arena<T> 
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Arena {
            items: self.items.clone(),
            allocated: Arc::new(Mutex::new(self.allocated.lock().unwrap().clone())),
            length: self.length,
            last_known_free: self.last_known_free,
            count: self.count,
        }
    }
}

impl<T> Arena<T> {
    // length in number of T it can hold
    pub fn new(length: usize) -> Self {
        let allocated = vec![false; length];
        Arena {
            items: vec![None; length],
            allocated: Arc::new(Mutex::new(allocated)),
            length,
            last_known_free: 0,
            count: 0,
        }
    }

    fn new_handle(&self, index: usize) -> Result<ArenaHandle<T>, ArenaError> {
        if index >= self.length {
            return Err(ArenaError::BoundsExceeded)
        }
        if !self.allocated.lock().unwrap()[index] {
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
            arena.overwrite(i, Arc::new(RwLock::new(item)));
            arena.count += 1;
        }
        arena
    }

    pub fn iter(&self) -> ArenaIterator<T> {
        return ArenaIterator { i: 0, arena: self }
    }

    // writes an object to an index regardless of allocation status. does no bounds checking. use with care.
    fn overwrite(&mut self, index: usize, obj: Arc<RwLock<T>>) {
        self.items[index] = Some(obj);
        self.allocated.lock().unwrap()[index] = true;
    }

    // create an object at the next available space - if no space is free, sad!
    pub fn create(&mut self, obj: T) -> Result<ArenaHandle<T>, ArenaError> {
        for i in self.last_known_free..self.length {
            if self.allocated.lock().unwrap()[i] { continue; } // already allocated to that slot, keep going

            // we found one that was free!
            self.items[i] = Some(Arc::new(RwLock::new(obj)));
            self.allocated.lock().unwrap()[i] = true;

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
        if !self.allocated.lock().unwrap()[handle.index] {
            return Err(ArenaError::DoesNotExist);
        }
        self.items[handle.index] = None;
        self.allocated.lock().unwrap()[handle.index] = false;
        self.last_known_free = handle.index;
        self.count -= 1;

        Ok(())
    }

    // get the object at a certain index, wrapped in a Arc<RwLock>
    pub fn fetch_lock(&self, handle: ArenaHandle<T>) -> Result<Arc<RwLock<T>>, ArenaError> {
        match &self.items[handle.index] {
            Some(arc_rwlock) => Ok(arc_rwlock.clone()),
            None => Err(ArenaError::DoesNotExist),
        }
    }

    pub fn read_lock(&self, handle: ArenaHandle<T>) -> Result<Arc<RwLock<T>>, ArenaError> {
        match self.fetch_lock(handle) {
            Ok(arc_rwlock) => Ok(arc_rwlock),
            Err(_) => Err(ArenaError::PoisonedLock)
        }
    }

    pub fn write_lock(&self, handle: ArenaHandle<T>) -> Result<Arc<RwLock<T>>, ArenaError> {
        let arc_rwlock = self.fetch_lock(handle)?;
        Ok(arc_rwlock)
    }
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