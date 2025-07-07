use serde::{ Serialize, Deserialize };
use itertools::iproduct;
use crate::subchunk::Subchunk;

macro_rules! impl_getter {
    ($name:ident, $return_type:ty, $sub_method:ident, $default:expr) => {
        pub fn $name(&self, pos: BlockPosition) -> $return_type {
            let index: usize = Self::subchunk_index(pos.z);

            if let Some(subchunk) = self.subchunks[index].as_ref() {
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
            let subchunk_opt: &mut Option<Subchunk<W, H, SD>> = &mut self.subchunks[index];

            if value == $default && subchunk_opt.is_none() {
                return; // return if placement is redundant
            }

            let subchunk: &mut Subchunk<W, H, SD> = subchunk_opt.get_or_insert_with(|| Subchunk::default());
            let sub_pos: BlockPosition = Self::local_to_sub(pos);

            subchunk.$sub_method(sub_pos, value);

            if subchunk.is_empty() {
                *subchunk_opt = None; // set empty subchunks to none
            }
        }
    };
}

const NUM_SUBCHUNKS: usize = 4;

/// Stores the two dimensional integer position of a chunk.
pub type ChunkPosition = glam::IVec2;

/// Stores the three dimensional integer position of a block.
pub type BlockPosition = glam::IVec3;

#[derive(Serialize, Deserialize, Default)]
pub struct Chunk<const W: usize, const H: usize, const D: usize, const SD: usize> {
    subchunks: [Option<Subchunk<W, H, SD>>; NUM_SUBCHUNKS],
}

impl<const W: usize, const H: usize, const D: usize, const SD: usize> Chunk<W, H, D, SD> {
    impl_getter!(block, u8, block, 0);
    impl_getter!(sky_light, u8, sky_light, 0);
    impl_getter!(block_light, u8, block_light, 0);
    impl_getter!(block_exposed, bool, block_exposed, false);

    impl_setter!(set_block, u8, set_block, 0);
    impl_setter!(set_sky_light, u8, set_sky_light, 0);
    impl_setter!(set_block_light, u8, set_block_light, 0);
    impl_setter!(set_block_exposed, bool, set_block_exposed, false);

    /// Returns an iterator for all block positions.
    pub fn chunk_coords() -> impl Iterator<Item = BlockPosition> {
        iproduct!(0..W as i32, 0..H as i32, 0..D as i32).map(|(x, y, z)|
            BlockPosition::new(x, y, z)
        )
    }

    /// Converts a given chunk position to its zero corner block position.
    pub const fn chunk_to_block_pos(pos: ChunkPosition) -> BlockPosition {
        BlockPosition::new(pos.x * (W as i32), pos.y * (H as i32), 0)
    }

    /// Gets the chunk position a block position falls into.
    pub const fn block_to_chunk_pos(pos: BlockPosition) -> ChunkPosition {
        ChunkPosition::new(pos.x.div_euclid(W as i32), pos.y.div_euclid(H as i32))
    }

    /// Finds the remainder of a global position using chunk size.
    pub const fn global_to_local_pos(pos: BlockPosition) -> BlockPosition {
        BlockPosition::new(pos.x.rem_euclid(W as i32), pos.y.rem_euclid(H as i32), pos.z)
    }

    const fn subchunk_index(pos_z: i32) -> usize {
        (pos_z as usize).div_euclid(SD)
    }

    const fn local_to_sub(pos: BlockPosition) -> BlockPosition {
        BlockPosition::new(pos.x, pos.y, pos.z.rem_euclid(SD as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_block() {
        let mut chunk: Chunk<16, 16, 32, 16> = Chunk::default();
        let pos_1: BlockPosition = BlockPosition::new(15, 1, 21);
        let pos_2: BlockPosition = BlockPosition::new(3, 0, 2);

        chunk.set_block(pos_1, 2);
        chunk.set_block(pos_1, 1);
        chunk.set_block(pos_2, 3);

        assert_eq!(chunk.block(pos_1), 1);
        assert_eq!(chunk.block(pos_2), 3);
    }
}
