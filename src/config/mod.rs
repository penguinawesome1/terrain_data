use serde::Deserialize;
use thiserror::Error;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::num;
use crate::block::Block;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("I/O error: {0}")] IoError(#[from] io::Error),
    #[error("Failed to parse integer: {0}")] ParseError(#[from] num::ParseIntError),
    #[error("TOML deserialization error: {0}")] TomlDeError(#[from] toml::de::Error),
    #[error("Too many blocks. Found {count}, max count is {max_allowed}.")] TooManyBlocksError {
        count: usize,
        max_allowed: u8,
    },
}

/// Converts toml path into a result for vec of blocks.
/// Intended for use as a lookup table with stored integers as blocks.
#[must_use]
pub fn load_blocks(path: &str) -> Result<Vec<Block>, CliError> {
    let contents: String = fs::read_to_string(path)?;
    let block_toml_map: BlockTomlMap = toml::from_str(&contents)?;

    let mut named_toml_blocks: Vec<(String, BlockToml)> = block_toml_map.blocks
        .into_iter()
        .collect();
    named_toml_blocks.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

    named_toml_blocks
        .into_iter()
        .enumerate()
        .map(|(n, (_, block_toml))| {
            if n > (u8::MAX as usize) {
                return Err(CliError::TooManyBlocksError { count: n, max_allowed: u8::MAX });
            }

            Ok(Block::from(block_toml))
        })
        .collect::<Result<Vec<Block>, CliError>>()
}

#[derive(Deserialize)]
struct BlockTomlMap {
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
