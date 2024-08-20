

use crate::complexplanet;
use crate::block::{BlockProtoSet, BlockID};
use crate::geometry::{Vertex, Facing};
use ndarray::prelude::*;
use ndarray::{Ix3, Axis};
use noise::NoiseFn;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE;
pub const CHUNK_SIZE_F: f32 = CHUNK_SIZE as f32;

type ChunkArray<T> = [T; CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE];

pub struct Chunk {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    ids_array: ChunkArray<BlockID>,
    visibility_array: ChunkArray<u8>,
}

impl<'a> Chunk {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        //stacker::maybe_grow(std::mem::size_of::<Self>(), std::mem::size_of::<Self>(), || {
        Self {
            x, y, z,
            ids_array: [0; CHUNK_VOLUME],
            visibility_array: [1; CHUNK_VOLUME],
        }
        //})
    }

    pub fn check_inside_me(&self, x: f32, y: f32, z: f32) -> bool {
        self.x + CHUNK_SIZE_F > x && x >= self.x && 
        self.y + CHUNK_SIZE_F > y && y >= self.y && 
        self.z + CHUNK_SIZE_F > z && z >= self.z
    }

    pub fn set_block_id_at(&mut self, x: f32, y: f32, z: f32, id: BlockID) {
        let (chunk_x, chunk_y, chunk_z) = (x.floor()-self.x, y.floor()-self.y, z.floor()-self.z);
        let (chunk_i, chunk_j, chunk_k) = (chunk_x as usize, chunk_y as usize, chunk_z as usize);
        Self::get_view_mut(&mut self.ids_array)[(chunk_i, chunk_j, chunk_k)] = id;
    }

    pub fn get_block_id_at(&self, x: f32, y: f32, z: f32) -> BlockID {
        let (chunk_x, chunk_y, chunk_z) = (x.floor()-self.x, y.floor()-self.y, z.floor()-self.z);
        let (chunk_i, chunk_j, chunk_k) = (chunk_x as usize, chunk_y as usize, chunk_z as usize);
        Self::get_view(&self.ids_array)[(chunk_i, chunk_j, chunk_k)]
    }

    fn get_view<T>(arr: &'a ChunkArray<T>) -> ArrayView::<'a, T, Ix3> {
        ArrayView::from_shape(Ix3(CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), arr).unwrap()
    }
    fn get_view_mut<T>(arr: &'a mut ChunkArray<T>) -> ArrayViewMut::<'a, T, Ix3> {
        ArrayViewMut::from_shape(Ix3(CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE), arr).unwrap()
    }

    pub fn generate_flat(&mut self) {
        if self.z == 0.0 {
            let mut ids = Self::get_view_mut(&mut self.ids_array);
            ids.slice_mut(s![.., .., 0]).fill(4);
            ids.slice_mut(s![.., .., 1..3]).fill(1);
            ids.slice_mut(s![.., .., 3]).fill(2);
        }
    }

    pub fn get_mesh(&self, indices_offset: u32, block_proto_set: &BlockProtoSet) -> (Vec<Vertex>, Vec<u32>) {
        use glam::Vec3A;
        let ids = Self::get_view(&self.ids_array);
        let mut vertices: Vec<Vertex> = vec![];

        // just thread this lol, this is 6*size threads easy

        for (z, slice) in ids.axis_iter(Axis(2)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y, self.z + z as f32), Facing::U, block_proto_set);
            vertices.extend(verts);
        }
        for (z, slice) in ids.axis_iter(Axis(2)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y, self.z + z as f32), Facing::D, block_proto_set);
            vertices.extend(verts);
        }
        for (y, slice) in ids.axis_iter(Axis(1)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y + y as f32, self.z), Facing::N, block_proto_set);
            vertices.extend(verts);
        }
        for (y, slice) in ids.axis_iter(Axis(1)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y + y as f32, self.z), Facing::S, block_proto_set);
            vertices.extend(verts);
        }
        for (x, slice) in ids.axis_iter(Axis(0)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x + x as f32, self.y, self.z), Facing::E, block_proto_set);
            vertices.extend(verts);
        }
        for (x, slice) in ids.axis_iter(Axis(0)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x + x as f32, self.y, self.z), Facing::W, block_proto_set);
            vertices.extend(verts);
        }

        //println!("{:?}", vertices.len());
        let mut indices: Vec<u32> = vec![];
        for i in 0..vertices.len()/4 {
            indices.extend(
                [indices_offset, indices_offset+1, indices_offset+2, indices_offset+2, indices_offset+3, indices_offset]
                .into_iter().map(|x| (x + (i as u32) * 4) )
            )
        };

        (vertices, indices)
    }

    pub fn generate_random(&mut self, seed: u32) {
        let perlin = noise::Perlin::new(0);
        let scale = 0.1;
        let height = 10.0;

        let mut ids = Self::get_view_mut(&mut self.ids_array);
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let val = perlin.get([self.x as f64 + x as f64 * scale, self.y as f64 + y as f64 * scale, self.z as f64 + z as f64 * scale]);
                    if val > 0.0 {
                        ids[(x, y, z)] = 1;
                    }
                }
            }
        }
    }

    pub fn generate_planet(&mut self) {
        let noise_map = complexplanet::generate(self.x as f64, (self.x+CHUNK_SIZE_F) as f64, self.y as f64, (self.y+CHUNK_SIZE_F) as f64);

        let mut ids = Self::get_view_mut(&mut self.ids_array);
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let z = noise_map.get_value((self.x + x as f32) as usize, (self.y + y as f32) as usize);
                ids[(x, y, z.round() as usize)] = 1;
            }
        }
    }
}

mod tessellate {
    use super::*;
    use ndarray::{ArrayView, Ix1, Ix2, Axis};
    use crate::geometry::{Vertex, Facing};
    use crate::block::BlockProtoSet;

    pub fn tessellate_slice(slice: ArrayView::<BlockID, Ix2>) -> Vec<(usize, usize, usize, usize, BlockID)> {
        let mut squares: Vec<(usize, usize, usize, usize, BlockID)> = vec![];
        let (mut x1, mut y1, mut x2, mut y2) = (0, 0, 0, 0);
        let mut found_new_square_anchor;

        fn in_square(x: usize, y: usize, sq: &(usize, usize, usize, usize, BlockID)) -> bool {
            x >= sq.0 && y >= sq.1 && x < sq.2 && y < sq.3
        }

        let mut i = 0;
        loop {
            i += 1;
            assert!(i < CHUNK_SIZE*CHUNK_SIZE, "You created an infinite loop in the tessellator. This is a bug, please report.");

            found_new_square_anchor = false;
            // find the next unchurched block
            for (y, row) in slice.axis_iter(Axis(1)).enumerate() {
                if y < y1 { continue; } // we're below the last known square so it can't be unchurched - skip

                for (x, v) in row.iter().enumerate() {
                    if *v != 0 {
                        found_new_square_anchor = true;
                        // if we are in a square, get out of here and start over with the next block
                        for square in &squares {
                            if in_square(x, y, square) {
                                found_new_square_anchor = false;
                                break;
                            }
                        }

                        if found_new_square_anchor {
                            (x1, y1, x2, y2) = (x, y, x, y);
                            break;
                        }
                    }
                }
                if found_new_square_anchor {
                    break;
                }
            }

            if !found_new_square_anchor {
                // all squares are churched! exit the loop at once!
                break;
            }

            let current_block_id: BlockID = slice[(x1, y1)];

            let mut hit_wall_y = false;
            let mut hit_wall_x = false;
            
            // grow the block
            while !(hit_wall_x && hit_wall_y) {
                if !hit_wall_x {
                    x2 += 1;

                    if x2 == slice.shape()[0]  // if you hit the edge of the chunk
                        || (y1..=y2).any(|y| squares.iter().any(|sq| in_square(x2, y, sq))) // if you hit a square we already built
                    {
                        hit_wall_x = true;
                    }
                    else
                    {  // if you hit a different block id (hole = 0)
                        let new_x_sliver: ArrayView::<BlockID, Ix1> = slice.slice(s![x2, y1..=y2]);
                        hit_wall_x = new_x_sliver.iter().any(|v| *v != current_block_id);
                    }

                    if hit_wall_x {x2 -= 1;}
                }
                if !hit_wall_y {
                    y2 += 1;

                    if y2 == slice.shape()[1]
                        || (x1..=x2).any(|x| squares.iter().any(|sq| in_square(x, y2, sq)))
                    {
                        hit_wall_y = true;
                    }
                    else
                    {
                        let new_y_sliver: ArrayView::<BlockID, Ix1> = slice.slice(s![x1..=x2, y2]);
                        hit_wall_y = new_y_sliver.iter().any(|v| *v != current_block_id);
                    }

                    if hit_wall_y {y2 -= 1;}
                }
            }

            squares.push((x1, y1, x2+1, y2+1, current_block_id));
        }
        squares
    }

    pub fn squares_to_vertices(squares: &Vec<(usize, usize, usize, usize, BlockID)>, offset: glam::Vec3A, facing: Facing, block_proto_set: &BlockProtoSet) -> Vec<Vertex> {
        use glam::Vec3A;
        let mut vertices: Vec<Vertex> = Vec::with_capacity(4*squares.len());

        // basis vectors for the subspace :)
        let e1 = match facing {
            // N W minus for UV purposes but we keep it positive to build the mesh symmetrically
            Facing::E | Facing::W => Vec3A::Y,
            _ => Vec3A::X, //-
        };
        let e2 = match facing {
            // U minus
            Facing::U | Facing::D => Vec3A::Y, //-
            _ => Vec3A::Z,
        };

        // to bottom left of chunk
        let offset = match facing {
            Facing::N => offset + Vec3A::Y,
            Facing::E => offset + Vec3A::X,
            Facing::U => offset + Vec3A::Z,
            _ => offset,
        };

        for sq in squares {
            let verts_raw = [
                (e1 * sq.0 as f32 + e2 * sq.3 as f32 + offset).to_array(),
                (e1 * sq.0 as f32 + e2 * sq.1 as f32 + offset).to_array(),
                (e1 * sq.2 as f32 + e2 * sq.1 as f32 + offset).to_array(),
                (e1 * sq.2 as f32 + e2 * sq.3 as f32 + offset).to_array(),
            ];

            let w = (sq.2 - sq.0) as f32;
            let h = (sq.3 - sq.1) as f32;
            let uvs_raw = [
                [0.0, 0.0],
                [0.0, h],
                [w, h],
                [w, 0.0],
            ];

            // sq.4 will never be 0 because the mesher ignores blocks with id 0
            let tex_id = block_proto_set.get_tex_id(sq.4 as BlockID, facing.clone());

            let face = (match facing {
                // fix normals and uvs by changing the order of the vertices, which are flipped for NWD
                Facing::N | Facing::W => [(3, 0), (2, 1), (1, 2), (0, 3)],
                Facing::D =>             [(3, 2), (2, 3), (1, 0), (0, 1)],
                _ =>                     [(0, 0), (1, 1), (2, 2), (3, 3)]
            }).map(|(vti, uvi)|
                Vertex{
                    pos: verts_raw[vti],
                    uv: uvs_raw[uvi],
                    tex_id: tex_id.try_into().unwrap()
                }
            );

            //println!("{} {}", offset, tex_id);
            //panic!();

            // W, N, D have flipped UVs
            vertices.extend(face);
        }

        vertices
    }
}




//#[test]
/*
fn main() {
    load_from_toml("config/blocks.toml");
    //let a: Array::<u32, Dim> = Array::<u32, _>::zeros((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE,).f()).dimension;
    let mut c = Chunk::new(0.0, 0.0, 0.0);
    c.ids.slice_mut(s![.., .., 0]).fill(1u32);
    c.ids[(7, 0, 0)] = 0u32;
    c.ids[(4, 6, 0)] = 0u32;
    c.ids[(14, 12, 0)] = 2u32;
    let slc = c.ids.slice(s![.., .., 0]);

    use std::time::Instant;
    let now = Instant::now();

    let t = tessellate::tessellate_slice(slc);
    //println!("{}/9", t.len());

    let elapsed = now.elapsed().mul_f32(16.0*6.0);
    println!("Elapsed: {:.2?}", elapsed);

    println!("{:?}", t);
}*/