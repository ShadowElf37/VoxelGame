use serde::Deserialize;
use crate::geometry::Facing;
use std::fmt; // Import the fmt module

pub type BlockID = u16;

mod BlockProtoDefaults {
    pub fn r#true () -> bool {true}
    pub fn tex_face_map_zeros () -> [usize; 6] {[0, 0, 0, 0, 0, 0]}
}

#[derive(Deserialize, Debug, Clone)]
pub struct BlockProto {
    pub name: String,
    pub textures: Vec<String>,

    #[serde(default = "BlockProtoDefaults::tex_face_map_zeros")]
    pub tex_face_map: [usize; 6], // newsud
    #[serde(default = "BlockProtoDefaults::r#true")]
    pub solid: bool,
    #[serde(default)]
    pub transparent: bool,
}

// Implement the Display trait for BlockProto
impl fmt::Display for BlockProto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockProto {{ name: {}, textures: {:?}, tex_face_map: {:?}, solid: {}, transparent: {} }}",
               self.name, self.textures, self.tex_face_map, self.solid, self.transparent)
    }
}

#[derive(Deserialize, Debug)]
struct BlockProtoArrayTableWrapper {
    blocks: Vec<BlockProto>
}

#[derive(Debug, Clone)]
pub struct BlockProtoSet {
    blocks: Vec<BlockProto>,
}

impl BlockProtoSet {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
        }
    }
    
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

        let mut actual_blocks = Vec::<BlockProto>::with_capacity(wrapper.blocks.len()+1);
        // ADD AIR FOR CONVENIENCE
        actual_blocks.push(BlockProto{
            name: "Air".to_string(),
            textures: vec![],
            tex_face_map: [0,0,0,0,0,0],
            solid: false,
            transparent: true,
        });
        actual_blocks.extend(wrapper.blocks);

        Self {
            blocks: actual_blocks,
        }
    }

    pub fn by_id(&self, block_id: BlockID) -> &BlockProto {
        &self.blocks[block_id as usize]
    }

    pub fn collect_textures(&self) -> Vec<String> {
        let mut textures = Vec::<String>::new();
        println!("Initial textures vector: {:?}", textures);
    
        if !self.blocks.is_empty() {
            for block in &self.blocks {
                println!("Processing block: {:?}", block);
        
                for texture in &block.textures {
                    let mut s = "assets/textures/".to_string();
                    s.push_str(texture);
                    println!("Processed texture: {}, Resulting string: {}", texture, s);
                    textures.push(s);
                }
            }
        } else {
            println!("No blocks to process.");
        }
    
        println!("Final textures vector: {:?}", textures);
        textures
    }

    pub fn get_tex_id(&self, block_id: BlockID, facing: Facing) -> usize {
        self.blocks[block_id as usize].tex_face_map[facing as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_new() {
        let block_proto_set = BlockProtoSet::new();
        assert!(block_proto_set.blocks.is_empty());
    }

    #[test]
    fn test_from_toml() {
        let toml_content = r#"
            [[blocks]]
            name = "Stone"
            textures = ["stone.png"]
            tex_face_map = [0, 0, 0, 0, 0, 0]
            solid = true
            transparent = false

            [[blocks]]
            name = "Glass"
            textures = ["glass.png"]
            tex_face_map = [1, 1, 1, 1, 1, 1]
            solid = false
            transparent = true
        "#;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("blocks.toml");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let block_proto_set = BlockProtoSet::from_toml(file_path.to_str().unwrap());
        assert_eq!(block_proto_set.blocks.len(), 3); // Including the "Air" block
        assert_eq!(block_proto_set.blocks[1].name, "Stone");
        assert_eq!(block_proto_set.blocks[2].name, "Glass");
    }

    #[test]
    fn test_by_id() {
        let block_proto_set = BlockProtoSet {
            blocks: vec![
                BlockProto {
                    name: "Air".to_string(),
                    textures: vec![],
                    tex_face_map: [0, 0, 0, 0, 0, 0],
                    solid: false,
                    transparent: true,
                },
                BlockProto {
                    name: "Stone".to_string(),
                    textures: vec!["stone.png".to_string()],
                    tex_face_map: [0, 0, 0, 0, 0, 0],
                    solid: true,
                    transparent: false,
                },
            ],
        };

        let block = block_proto_set.by_id(1);
        assert_eq!(block.name, "Stone");
    }

    #[test]
    fn test_collect_textures() {
        let block_proto_set = BlockProtoSet {
            blocks: vec![
                BlockProto {
                    name: "Air".to_string(),
                    textures: vec![],
                    tex_face_map: [0, 0, 0, 0, 0, 0],
                    solid: false,
                    transparent: true,
                },
                BlockProto {
                    name: "Stone".to_string(),
                    textures: vec!["stone.png".to_string()],
                    tex_face_map: [0, 0, 0, 0, 0, 0],
                    solid: true,
                    transparent: false,
                },
                BlockProto {
                    name: "Glass".to_string(),
                    textures: vec!["glass.png".to_string()],
                    tex_face_map: [1, 1, 1, 1, 1, 1],
                    solid: false,
                    transparent: true,
                },
            ],
        };

        let textures = block_proto_set.collect_textures();
        assert_eq!(textures, vec!["assets/textures/stone.png", "assets/textures/glass.png"]);
    }
}