use serde::{ Serialize, Deserialize };
use crate::subchunk::Subchunk;
use crate::BlockPosition;

macro_rules! impl_getter {
    ($name:ident, $return_type:ty, $sub_method:ident, $default:expr) => {
        #[inline]
        pub unsafe fn $name(&self, pos: BlockPosition) -> $return_type {
            let index: usize = Self::subchunk_index(pos.z);

            self.subchunks[index].as_ref().map_or($default, |s| {
                let sub_pos: BlockPosition = Self::local_to_sub(pos);
                unsafe { s.$sub_method(sub_pos) }
            })
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $sub_method:ident) => {
        pub unsafe fn $name(&mut self, pos: BlockPosition, value: $value_type) {
            let index: usize = Self::subchunk_index(pos.z);
            let subchunk_opt: &mut Option<Subchunk<W, H, SD>> = &mut self.subchunks[index];

            if value as u32 == 0 && subchunk_opt.is_none() {
                return; // return if placement is redundant
            }

            let subchunk: &mut Subchunk<W, H, SD> = subchunk_opt.get_or_insert_with(|| Subchunk::default());
            let sub_pos: BlockPosition = Self::local_to_sub(pos);

            unsafe { subchunk.$sub_method(sub_pos, value); }

            if subchunk.is_empty() {
                *subchunk_opt = None; // set empty subchunks to none
            }
        }
    };
}

#[derive(Serialize, Deserialize, Default)]
pub struct Chunk<const W: usize, const H: usize, const SD: usize, const NS: usize>
    where for<'a> [Option<Subchunk<W, H, SD>>; NS]: Sized + Default + Serialize + Deserialize<'a> {
    subchunks: [Option<Subchunk<W, H, SD>>; NS],
}

impl<const W: usize, const H: usize, const SD: usize, const NS: usize> Chunk<W, H, SD, NS>
    where for<'a> [Option<Subchunk<W, H, SD>>; NS]: Sized + Default + Serialize + Deserialize<'a>
{
    impl_getter!(block, u8, block, 0);
    impl_getter!(sky_light, u8, sky_light, 0);
    impl_getter!(block_light, u8, block_light, 0);
    impl_getter!(block_exposed, bool, block_exposed, false);

    impl_setter!(set_block, u8, set_block);
    impl_setter!(set_sky_light, u8, set_sky_light);
    impl_setter!(set_block_light, u8, set_block_light);
    impl_setter!(set_block_exposed, bool, set_block_exposed);

    #[inline]
    const fn subchunk_index(pos_z: i32) -> usize {
        (pos_z as usize).div_euclid(SD)
    }

    #[inline]
    const fn local_to_sub(pos: BlockPosition) -> BlockPosition {
        BlockPosition::new(pos.x, pos.y, pos.z.rem_euclid(SD as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_block() {
        let mut chunk: Chunk<16, 16, 16, 4> = Chunk::default();
        let pos_1: BlockPosition = BlockPosition::new(15, 1, 21);
        let pos_2: BlockPosition = BlockPosition::new(3, 0, 2);

        unsafe {
            chunk.set_block(pos_1, 2);
            chunk.set_block(pos_1, 1);
            chunk.set_block(pos_2, 3);

            assert_eq!(chunk.block(pos_1), 1);
            assert_eq!(chunk.block(pos_2), 3);
        }
    }
}
