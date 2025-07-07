use serde::{ Serialize, Deserialize };
use crate::subchunk::Subchunk;
use crate::world::BlockPosition;

// number of stacked subchunks in one chunk
const NUM_SUBCHUNKS: usize = 4;

macro_rules! impl_getter {
    ($name:ident, $return_type:ty, $sub_method:ident, $default:expr) => {
        pub(crate) fn $name(&self, pos: BlockPosition) -> $return_type {
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
        pub(crate) fn $name(&mut self, pos: BlockPosition, value: $value_type) {
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
