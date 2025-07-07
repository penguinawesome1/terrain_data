use serde::Deserialize;
use std::collections::HashMap;
use toml;
use std::error::Error;
use crate::block::Block;

pub fn load_blocks(toml_str: &str) -> Result<Vec<Block>, Box<dyn Error>> {
    let config: BlockTomlHashMap = toml::from_str(toml_str)?;
    Ok(config.blocks.into_values().map(Block::from).collect())
}

#[derive(Deserialize)]
struct BlockTomlHashMap {
    #[serde(flatten)]
    blocks: HashMap<String, BlockToml>,
}

#[derive(Deserialize)]
struct BlockToml {
    is_hoverable: bool,
    is_visible: bool,
    is_breakable: bool,
    is_collidable: bool,
    is_replaceable: bool,
}

impl From<BlockToml> for Block {
    fn from(block_toml: BlockToml) -> Self {
        Block::new(
            block_toml.is_hoverable,
            block_toml.is_visible,
            block_toml.is_breakable,
            block_toml.is_collidable,
            block_toml.is_replaceable
        )
    }
}
