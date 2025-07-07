use serde::{ Serialize, Deserialize };
use itertools::iproduct;
use crate::{
    coords::BlockPosition,
    subchunk::{ Subchunk, SUBCHUNK_WIDTH, SUBCHUNK_HEIGHT, SUBCHUNK_DEPTH },
    block::Block,
};

const SUBCHUNKS_IN_CHUNK: usize = 4;

pub const CHUNK_WIDTH: usize = SUBCHUNK_WIDTH;
pub const CHUNK_HEIGHT: usize = SUBCHUNK_HEIGHT;
pub const CHUNK_DEPTH: usize = SUBCHUNK_DEPTH * SUBCHUNKS_IN_CHUNK;
pub const CHUNK_VOLUME: usize = CHUNK_WIDTH * CHUNK_HEIGHT * CHUNK_DEPTH;

macro_rules! impl_getter {
    ($name:ident, $return_type:ty, $sub_method:ident, $default:expr) => {
        pub fn $name(&self, pos: BlockPosition) -> $return_type {
            if let Some(subchunk) = self.subchunk(pos.z) {
                let sub_pos: BlockPosition = Self::local_to_sub(pos);
                subchunk.$sub_method(sub_pos)
            } else {
                $default
            }
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $sub_method:ident, $default:expr) => {
        pub fn $name(&mut self, pos: BlockPosition, value: $value_type) {
            let index: usize = Self::subchunk_index(pos.z);
            let subchunk_opt: &mut Option<Subchunk> = &mut self.subchunks[index];

            if value == $default && subchunk_opt.is_none() {
                return; // return if placement is redundant
            }

            let subchunk: &mut Subchunk = subchunk_opt.get_or_insert_with(|| Subchunk::default());
            let sub_pos: BlockPosition = Self::local_to_sub(pos);

            subchunk.$sub_method(sub_pos, value);

            if subchunk.is_empty() {
                *subchunk_opt = None; // set empty subchunks to none
            }
        }
    };
}

#[derive(Serialize, Deserialize, Default)]
pub struct Chunk {
    subchunks: [Option<Subchunk>; SUBCHUNKS_IN_CHUNK],
}

impl Chunk {
    impl_getter!(block, Block, block, Block::Air);
    impl_getter!(sky_light, u8, sky_light, 0);
    impl_getter!(block_light, u8, block_light, 0);
    impl_getter!(block_exposed, bool, block_exposed, false);

    impl_setter!(set_block, Block, set_block, Block::Air);
    impl_setter!(set_sky_light, u8, set_sky_light, 0);
    impl_setter!(set_block_light, u8, set_block_light, 0);
    impl_setter!(set_block_exposed, bool, set_block_exposed, false);

    /// Returns an iterator for all block positions.
    pub fn chunk_coords() -> impl Iterator<Item = BlockPosition> {
        iproduct!(0..CHUNK_WIDTH as i32, 0..CHUNK_HEIGHT as i32, 0..CHUNK_DEPTH as i32).map(
            |(x, y, z)| BlockPosition::new(x, y, z)
        )
    }

    const fn subchunk(&self, pos_z: i32) -> Option<&Subchunk> {
        let index: usize = Self::subchunk_index(pos_z);
        self.subchunks[index].as_ref()
    }

    const fn subchunk_index(pos_z: i32) -> usize {
        (pos_z as usize).div_euclid(SUBCHUNK_DEPTH)
    }

    const fn local_to_sub(pos: BlockPosition) -> BlockPosition {
        BlockPosition::new(pos.x, pos.y, pos.z.rem_euclid(SUBCHUNK_DEPTH as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    #[test]
    fn test_set_and_get_block() {
        let mut chunk: Chunk = Chunk::default();
        let pos_1: IVec3 = IVec3::new(15, 1, 21);
        let pos_2: IVec3 = IVec3::new(3, 0, 2);

        chunk.set_block(pos_1, Block::Dirt);
        chunk.set_block(pos_1, Block::Grass);
        chunk.set_block(pos_2, Block::Air);

        assert_eq!(chunk.block(pos_1), Block::Grass);
        assert_eq!(chunk.block(pos_2), Block::Air);
    }
}
