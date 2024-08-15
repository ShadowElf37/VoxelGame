use serde::Deserialize;
use crate::geometry::{Vertex, Facing};
use ndarray::prelude::*;
use ndarray::{Array, Ix3, Axis};


pub type BlockID = u16;

mod BlockProtoDefaults {
    pub fn True () -> bool {true}
    pub fn TexFaceMapZeros () -> [usize; 6] {[0, 0, 0, 0, 0, 0]}
}
#[derive(Deserialize, Debug)]
pub struct BlockProto {
    pub name: String,
    pub textures: Vec<String>,

    #[serde(default = "BlockProtoDefaults::TexFaceMapZeros")]
    pub tex_face_map: [usize; 6], // newsud
    #[serde(default = "BlockProtoDefaults::True")]
    pub solid: bool,
    #[serde(default)]
    pub transparent: bool,
}

#[derive(Deserialize, Debug)]
struct BlockProtoArrayTableWrapper {
    blocks: Vec<BlockProto>
}


pub struct BlockProtoSet {
    pub blocks: Vec<BlockProto>,
}
impl BlockProtoSet {
    pub fn from_toml(fp: &str) -> Self {
        use std::fs::read_to_string;
        let data = read_to_string(fp).expect(&format!("Couldn't open {}", fp));
        let mut wrapper = toml::from_str::<BlockProtoArrayTableWrapper>(&data).expect("Improperly formatted toml");
        let mut true_tex_offset = 0;

        for block in wrapper.blocks.iter_mut() {
            for val in &mut block.tex_face_map {
                *val += true_tex_offset;
            }
            true_tex_offset += block.textures.len();
            //println!("{:?}", block.tex_face_map);
        }

        Self {
            blocks: wrapper.blocks,
        }
    }

    pub fn collect_textures(&self) -> Vec<String> {
        let mut textures = Vec::<String>::new();
        for block in &self.blocks {
            for texture in &block.textures {
                let mut s = "assets/textures/".to_string();
                s.push_str(texture);
                textures.push(s);
            }
        }
        textures
    }

    pub fn get_tex_id(&self, block_id: BlockID, facing: Facing) -> usize {
        return self.blocks[block_id as usize - 1].tex_face_map[facing as usize]
    }
}