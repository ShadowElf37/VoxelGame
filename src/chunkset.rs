use std::slice::Iter;
use glam::Vec3;
use crate::chunk::{CHUNK_SIZE_F, Chunk};
use ndarray::prelude::*;
use crate::memblock::*;
use std::alloc::{alloc, dealloc, Layout, handle_alloc_error, alloc_zeroed};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::mem::{size_of, align_of};
use crate::block;


pub type ChunkCoord = (isize, isize, isize);

pub struct ChunkSet {
    chunks: MemoryBlock<RwLock<Chunk>>,
    pub center: ChunkCoord,
    pub render_distance: isize,
    pub arr_length: usize,
    arr_area: usize,
    arr_vol: usize,
}

impl ChunkSet {
    pub fn new(center: ChunkCoord, render_distance: usize) -> Self {
        let arr_length = 2*render_distance+1;
        let arr_area = arr_length*arr_length;
        let arr_vol = arr_area*arr_length;
        Self {
            chunks: MemoryBlock::new(arr_vol),
            center,
            render_distance: render_distance.try_into().unwrap(),
            arr_length,
            arr_area,
            arr_vol,
        }
    }
    pub fn recenter(&mut self, center: ChunkCoord) {
        self.center = center;
    }

    pub fn generate_chunk(&mut self, chunk_coord: ChunkCoord, tp: &rayon::ThreadPool, block_proto_set: &block::BlockProtoSet, device: &wgpu::Device) {
        let i = self.arr_index_to_real_index(self.chunk_coord_to_arr_index(chunk_coord));
        unsafe {
            if self.chunks.is_allocated(i) {self.chunks.drop(i);}
            self.chunks.write(i, RwLock::new(Chunk::new(
                chunk_coord.0 as f32 * CHUNK_SIZE_F,
                chunk_coord.1 as f32 * CHUNK_SIZE_F,
                chunk_coord.2 as f32 * CHUNK_SIZE_F
            )));
        }
        let lock = unsafe{self.chunks.read(i)};
        tp.install(||{
            let mut chunk = lock.write().unwrap();
            chunk.generate_planet();
            chunk.make_mesh(block_proto_set, tp);
            chunk.make_vertex_buffer(device);
            chunk.ready_to_display = true;
            drop(chunk);
        });
    }
    pub fn mark_unloaded(&mut self, chunk_coord: ChunkCoord) {
        //self.get_chunk_at_chunk_coords(chunk_coord).unwrap().write().unwrap().ready_to_display = false;
        unsafe {self.chunks.drop(self.arr_index_to_real_index(self.chunk_coord_to_arr_index(chunk_coord)));}
    }
    pub fn is_unloaded(&self, chunk_coord: ChunkCoord) -> bool {
        unsafe {!self.chunks.is_allocated(self.chunk_coord_to_real_index(chunk_coord))}
    }

    pub fn world_to_chunk_coords(&self, pos: Vec3) -> ChunkCoord {
        (
            (pos.x / CHUNK_SIZE_F).floor() as isize,
            (pos.y / CHUNK_SIZE_F).floor() as isize,
            (pos.z / CHUNK_SIZE_F).floor() as isize,
        )
    }
    fn chunk_coord_to_arr_index(&self, coord: ChunkCoord) -> (usize, usize, usize) {
        //println!("{:?} {}", coord, self.arr_length);
        (
            coord.0.rem_euclid(self.arr_length as isize).try_into().unwrap(),
            coord.1.rem_euclid(self.arr_length as isize).try_into().unwrap(),
            coord.2.rem_euclid(self.arr_length as isize).try_into().unwrap(),
        )
    }
    fn arr_index_to_real_index(&self, index: (usize, usize, usize)) -> usize {
        index.0 * self.arr_area + index.1 * self.arr_length + index.2
    }
    pub fn chunk_coord_to_real_index(&self, coord: ChunkCoord) -> usize {
        self.arr_index_to_real_index(self.chunk_coord_to_arr_index(coord))
    }

    pub fn check_in_bounds(&self, chunk_coord: ChunkCoord) -> bool {
        chunk_coord.0 <= self.center.0 + self.render_distance &&
        chunk_coord.0 >= self.center.0 - self.render_distance &&
        chunk_coord.1 <= self.center.1 + self.render_distance &&
        chunk_coord.1 >= self.center.1 - self.render_distance &&
        chunk_coord.2 <= self.center.2 + self.render_distance &&
        chunk_coord.2 >= self.center.2 - self.render_distance
    }
    pub fn get_chunk_at_chunk_coords(&self, chunk_coord: ChunkCoord) -> Option<&RwLock<Chunk>> {
        if self.check_in_bounds(chunk_coord) {
            let i = self.arr_index_to_real_index(self.chunk_coord_to_arr_index(chunk_coord));
            if unsafe {self.chunks.is_allocated(i)} {
                return unsafe {Some(self.chunks.read(i))};
            }
        }
        None
    }
    pub fn get_chunk_at_world_coords(&self, pos: Vec3) -> Option<&RwLock<Chunk>> {
        let c = self.world_to_chunk_coords(pos);
        self.get_chunk_at_chunk_coords(c)
    }

    pub fn iter(&self) -> impl Iterator<Item = &RwLock<Chunk>> {
        self.chunks.iter()
    }
}

