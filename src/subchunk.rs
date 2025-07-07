use serde::{ Serialize, Deserialize };
use crate::{ coords::BlockPosition, block::Block };

pub const SUBCHUNK_WIDTH: usize = 16;
pub const SUBCHUNK_HEIGHT: usize = 16;
pub const SUBCHUNK_DEPTH: usize = 16;

type ChunkSection = palette_bitmap::Section<SUBCHUNK_WIDTH, SUBCHUNK_HEIGHT, SUBCHUNK_DEPTH>;

macro_rules! impl_getter {
    ($name:ident, Block, $section:ident) => {
        pub fn $name(&self, pos: BlockPosition) -> Block {
            (self.$section.as_ref().map_or(0, |section| section.item(pos))).into()
        }
    };

    ($name:ident, bool, $section:ident) => {
        pub fn $name(&self, pos: BlockPosition) -> bool {
            self.$section.as_ref().map_or(0, |section| section.item(pos)) == 0
        }
    };

    ($name:ident, $return_type:ty, $section:ident) => {
        pub fn $name(&self, pos: BlockPosition) -> $return_type {
            self.$section.as_ref().map_or(0, |section| section.item(pos)) as $return_type
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $section:ident, $bits_per_item:expr) => {
        pub fn $name(&mut self, pos: BlockPosition, value: $value_type) {
            let value_u64: u64 = value.into();
            if value_u64 == 0 && self.$section.is_none() {
                return; // return is placement is redundant
            }

            let section: &mut ChunkSection = self.$section.get_or_insert_with(
                || ChunkSection::new($bits_per_item) // create new section if needed
            );
            section.set_item(pos, value_u64);

            if section.is_empty() {
                self.$section = None; // convert empty section to none
            }
        }
    };
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Subchunk {
    blocks: Option<ChunkSection>,
    sky_light: Option<ChunkSection>,
    block_light: Option<ChunkSection>,
    exposed_blocks: Option<ChunkSection>,
}

impl Subchunk {
    impl_getter!(block, Block, blocks);
    impl_getter!(sky_light, u8, sky_light);
    impl_getter!(block_light, u8, block_light);
    impl_getter!(block_exposed, bool, exposed_blocks);

    impl_setter!(set_block, Block, blocks, 4);
    impl_setter!(set_sky_light, u8, sky_light, 5);
    impl_setter!(set_block_light, u8, block_light, 4);
    impl_setter!(set_block_exposed, bool, exposed_blocks, 5);

    /// Returns a bool for if all sections are empty.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    #[test]
    fn test_set_and_get_block() {
        let mut subchunk: Subchunk = Subchunk::default();
        let pos_1: IVec3 = IVec3::new(15, 1, 1);
        let pos_2: IVec3 = IVec3::new(3, 0, 2);

        subchunk.set_block(pos_1, Block::Dirt);
        subchunk.set_block(pos_1, Block::Grass);
        subchunk.set_block(pos_2, Block::Air);

        assert_eq!(subchunk.block(pos_1), Block::Grass);
        assert_eq!(subchunk.block(pos_2), Block::Air);
    }
}
