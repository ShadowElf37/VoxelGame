use toml::from_str;
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub struct BlockProto {
    pub textures: Vec<String>,
    //#[serde(default = "[0,0,0,0,0,0]")]
    pub tex_face_map: [usize; 6], // newsud
    pub solid: bool,
    #[serde(default)]
    pub transparent: bool,
}

#[derive(Deserialize, Debug)]
struct BlockProtoSet { blocks: Vec<BlockProto> }

pub fn load_from_toml(fp: &str) -> Vec<BlockProto> {
    use std::fs::read_to_string;
    let data = read_to_string(fp).expect(&format!("Couldn't open {}", fp));
    toml::from_str::<BlockProtoSet>(&data).expect("Invalid toml").blocks
}

#[test]
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
}


use ndarray::prelude::*;
use ndarray::{Array, ArrayView, Ix1, Ix2, Ix3, Axis};
use crate::geometry::{Vertex, Facing};

const CHUNK_SIZE: usize = 16;
const CHUNK_SIZE_F: f32 = 16.0;
pub struct Chunk {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub ids: Array::<u32, Ix3>,
}
impl Chunk {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x, y, z,
            ids: Array::<u32, Ix3>::zeros((CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE,))
        }
    }

    pub fn get_mesh(&self) -> (Vec<Vertex>, Vec<u32>) {
        use glam::Vec3A;
        let mut vertices: Vec<Vertex> = vec![];

        for (z, slice) in self.ids.axis_iter(Axis(2)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y, self.z + z as f32), Facing::U);
            vertices.extend(verts);
        }
        for (z, slice) in self.ids.axis_iter(Axis(2)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y, self.z + z as f32), Facing::D);
            vertices.extend(verts);
        }
        for (y, slice) in self.ids.axis_iter(Axis(1)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y + y as f32, self.z), Facing::N);
            vertices.extend(verts);
        }
        for (y, slice) in self.ids.axis_iter(Axis(1)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x, self.y + y as f32, self.z), Facing::S);
            vertices.extend(verts);
        }
        for (x, slice) in self.ids.axis_iter(Axis(0)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x + x as f32, self.y, self.z), Facing::E);
            vertices.extend(verts);
        }
        for (x, slice) in self.ids.axis_iter(Axis(0)).enumerate() {
            let squares = tessellate::tessellate_slice(slice);
            let verts = tessellate::squares_to_vertices(&squares, Vec3A::new(self.x + x as f32, self.y, self.z), Facing::W);
            vertices.extend(verts);
        }

        //println!("{:?}", vertices.len());
        let mut indices: Vec<u32> = vec![];
        for i in 0..vertices.len()/4 {
            indices.extend([0, 1, 2, 2, 3, 0].into_iter().map(|x| (x+i*4) as u32))
        };

        (vertices, indices)
    }
}

mod tessellate {
    use crate::block::{CHUNK_SIZE, CHUNK_SIZE_F};
    use glam::Vec3A;
    use ndarray::prelude::*;
    use ndarray::{Array, ArrayView, Ix1, Ix2, Ix3, Axis};
    use crate::geometry::{Vertex, Facing};

    pub fn tessellate_slice(slice: ArrayView::<u32, Ix2>) -> Vec<(usize, usize, usize, usize, u32)> {
        let mut squares: Vec<(usize, usize, usize, usize, u32)> = vec![];
        let (mut x1, mut y1, mut x2, mut y2) = (0, 0, 0, 0);
        let mut found_new_square_anchor;

        fn in_square(x: usize, y: usize, sq: &(usize, usize, usize, usize, u32)) -> bool {
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

            let current_block_id = slice[(x1, y1)];

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
                        let new_x_sliver: ArrayView::<u32, Ix1> = slice.slice(s![x2, y1..=y2]);
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
                        let new_y_sliver: ArrayView::<u32, Ix1> = slice.slice(s![x1..=x2, y2]);
                        hit_wall_y = new_y_sliver.iter().any(|v| *v != current_block_id);
                    }

                    if hit_wall_y {y2 -= 1;}
                }
            }

            squares.push((x1, y1, x2+1, y2+1, current_block_id));
        }
        squares
    }

    pub fn squares_to_vertices(squares: &Vec<(usize, usize, usize, usize, u32)>, offset: glam::Vec3A, facing: Facing) -> Vec<Vertex> {
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

        // to bottom left of chunk face
        let offset = match facing {
            Facing::N => offset + Vec3A::Y,
            Facing::E => offset + Vec3A::X,
            Facing::U => offset + Vec3A::Z,
            _ => offset,
        };

        for sq in squares {
            /*let sq = match facing {
                // flip NWD so they are 
                Facing::N | Facing::W | Facing::D => &(sq.2, sq.3, sq.0, sq.1, sq.4),
                _ => sq,
            };*/

            let w = (sq.2 as f32 - sq.0 as f32).abs();
            let h = (sq.3 as f32 - sq.1 as f32).abs();

            let face = [
                Vertex {
                    pos: (e1 * sq.0 as f32 + e2 * sq.3 as f32 + offset).to_array(),
                    uv: [0.0, 0.0],
                    tex_id: sq.4
                },
                Vertex {
                    pos: (e1 * sq.0 as f32 + e2 * sq.1 as f32 + offset).to_array(),
                    uv: [0.0, h],
                    tex_id: sq.4
                },
                Vertex {
                    pos: (e1 * sq.2 as f32 + e2 * sq.1 as f32 + offset).to_array(),
                    uv: [w, h],
                    tex_id: sq.4
                },
                Vertex {
                    pos: (e1 * sq.2 as f32 + e2 * sq.3 as f32 + offset).to_array(),
                    uv: [w, 0.0],
                    tex_id: sq.4
                }
            ];

            // W, N, D have flipped UVs
            //vertices.extend(face);
            match facing {
                Facing::N => vertices.extend(face.iter().rev()),
                Facing::E => vertices.extend(face),
                Facing::W => vertices.extend(face.iter().rev()),
                Facing::S => vertices.extend(face),
                Facing::U => vertices.extend(face),
                Facing::D => vertices.extend(face.iter().rev()),
            }
        }

        vertices
    }
}