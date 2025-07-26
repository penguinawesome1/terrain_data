use serde::{ Serialize, Deserialize };
use chroma::{ Section, BoundsError };
use crate::BlockPosition;

macro_rules! impl_getter {
    ($name:ident, bool, $section:ident) => {
        #[inline]
        pub fn $name(&self, pos: BlockPosition) -> Result<bool, BoundsError> {
            self.$section.as_ref().map_or(Ok(false), |s| s.item(pos).map(|val| val != 0))
        }
    };
    ($name:ident, $return_type:ty, $section:ident) => {
        #[inline]
        pub fn $name(&self, pos: BlockPosition) -> Result<$return_type, BoundsError> {
            self.$section.as_ref().map_or(Ok(0), |s| s.item(pos).map(|val| val as $return_type))
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $section:ident, $bits_per_item:expr) => {
        #[must_use]
        pub fn $name(&mut self, pos: BlockPosition, value: $value_type) -> Result<(), BoundsError> {
            let value_u64: u64 = value.into();
            if value_u64 == 0 && self.$section.is_none() {
                return Ok(()); // return if placement is redundant
            }

            let section: &mut Section<W, H, D> = self.$section.get_or_insert_with(
                || Section::new($bits_per_item) // create new section if needed
            );
            section.set_item(pos, value_u64)?;

            if section.is_empty() {
                self.$section = None; // convert empty section to none
            }

            Ok(())
        }
    };
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Subchunk<const W: usize, const H: usize, const D: usize> {
    blocks: Option<Section<W, H, D>>,
    sky_light: Option<Section<W, H, D>>,
    block_light: Option<Section<W, H, D>>,
    exposed_blocks: Option<Section<W, H, D>>,
}

impl<const W: usize, const H: usize, const D: usize> Subchunk<W, H, D> {
    impl_getter!(block, u8, blocks);
    impl_getter!(sky_light, u8, sky_light);
    impl_getter!(block_light, u8, block_light);
    impl_getter!(block_exposed, bool, exposed_blocks);

    impl_setter!(set_block, u8, blocks, 4);
    impl_setter!(set_sky_light, u8, sky_light, 5);
    impl_setter!(set_block_light, u8, block_light, 4);
    impl_setter!(set_block_exposed, bool, exposed_blocks, 6);

    /// Returns a bool for if all sections are empty.
    #[inline]
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
        let mut subchunk: Subchunk<16, 16, 16> = Subchunk::default();
        let pos_1: IVec3 = IVec3::new(15, 1, 1);
        let pos_2: IVec3 = IVec3::new(3, 0, 2);

        subchunk.set_block(pos_1, 0).unwrap();
        subchunk.set_block(pos_1, 4).unwrap();
        subchunk.set_block(pos_2, 5).unwrap();

        assert_eq!(subchunk.block(pos_1).unwrap(), 4);
        assert_eq!(subchunk.block(pos_2).unwrap(), 5);
    }
}
