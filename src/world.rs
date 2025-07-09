use std::collections::HashMap;
use itertools::iproduct;
use thiserror::Error;
use crate::chunk::Chunk;

macro_rules! impl_getter {
    ($name:ident, $sub_method:ident, $return_type:ty) => {
        pub fn $name(&self, pos: BlockPosition) -> Result<$return_type, ChunkError> {
            let chunk_pos: ChunkPosition = Self::block_to_chunk_pos(pos);
            let local_pos: BlockPosition = Self::global_to_local_pos(pos);
            Ok(self.chunk(chunk_pos)?.$sub_method(local_pos))
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $sub_method:ident) => {
        pub fn $name(&mut self, pos: BlockPosition, value: $value_type) -> Result<(), ChunkError> {
            let chunk_pos: ChunkPosition = Self::block_to_chunk_pos(pos);
            let local_pos: BlockPosition = Self::global_to_local_pos(pos);
            self.mut_chunk(chunk_pos)?.$sub_method(local_pos, value);
            Ok(())
        }
    };
}

const CHUNK_ADJ_OFFSETS: [ChunkPosition; 4] = [
    ChunkPosition::new(-1, 0),
    ChunkPosition::new(1, 0),
    ChunkPosition::new(0, -1),
    ChunkPosition::new(0, 1),
];

const BLOCK_OFFSETS: [BlockPosition; 6] = [
    BlockPosition::new(1, 0, 0),
    BlockPosition::new(0, 1, 0),
    BlockPosition::new(0, 0, 1),
    BlockPosition::new(-1, 0, 0),
    BlockPosition::new(0, -1, 0),
    BlockPosition::new(0, 0, -1),
];

/// Stores the two dimensional integer position of a chunk.
pub type ChunkPosition = glam::IVec2;

/// Stores the three dimensional integer position of a block.
pub type BlockPosition = glam::IVec3;

#[derive(Debug, Error)]
pub enum ChunkError {
    #[error("Chunk at position is currently unloaded")]
    ChunkUnloaded,
    #[error("A chunk already exists at the specified position")]
    ChunkAlreadyLoaded,
}

/// Stores all chunks and marks dirty chunks.
/// Allows access and modification to them.
#[derive(Default)]
pub struct World<const CW: usize, const CH: usize, const CD: usize, const SD: usize> {
    chunks: HashMap<ChunkPosition, Chunk<CW, CH, CD, SD>>,
}

impl<const CW: usize, const CH: usize, const CD: usize, const SD: usize> World<CW, CH, CD, SD> {
    impl_getter!(block, block, u8);
    impl_getter!(sky_light, sky_light, u8);
    impl_getter!(block_light, block_light, u8);
    impl_getter!(block_exposed, block_exposed, bool);

    impl_setter!(set_block, u8, set_block);
    impl_setter!(set_sky_light, u8, set_sky_light);
    impl_setter!(set_block_light, u8, set_block_light);
    impl_setter!(set_block_exposed, bool, set_block_exposed);

    /// Sets new blank chunk at the passed position.
    /// Returns an error if a chunk is already at the position.
    #[must_use]
    pub fn add_default_chunk(&mut self, pos: ChunkPosition) -> Result<(), ChunkError> {
        if self.is_chunk_at_pos(pos) {
            return Err(ChunkError::ChunkAlreadyLoaded);
        }

        let chunk: Chunk<CW, CH, CD, SD> = Chunk::default();
        self.chunks.insert(pos, chunk);
        Ok(())
    }

    /// Returns bool for if a chunk is found at the passed position.
    pub fn is_chunk_at_pos(&self, pos: ChunkPosition) -> bool {
        self.chunks.contains_key(&pos)
    }

    /// Gets an iter of all chunk positions in a square around the passed origin position.
    /// Radius of 0 results in 1 position.
    pub fn positions_in_square(
        origin: ChunkPosition,
        radius: u32
    ) -> impl Iterator<Item = ChunkPosition> {
        let radius: i32 = radius as i32;
        iproduct!(-radius..=radius, -radius..=radius).map(
            move |(x, y)| origin + ChunkPosition::new(x, y)
        )
    }

    /// Returns all adjacent chunk offsets.
    #[inline]
    pub fn chunk_offsets(pos: ChunkPosition) -> impl Iterator<Item = ChunkPosition> {
        CHUNK_ADJ_OFFSETS.iter().map(move |offset| { pos + offset })
    }

    /// Returns all adjacent block offsets.
    /// Filters out illegal vertical offsets.
    #[inline]
    pub fn block_offsets(pos: BlockPosition) -> impl Iterator<Item = BlockPosition> {
        BLOCK_OFFSETS.iter()
            .map(move |offset| { pos + offset })
            .filter(|adj_pos| { adj_pos.z >= 0 && adj_pos.z < (CD as i32) })
    }

    /// Returns an iter for every global position found in the passed chunk positions.
    pub fn global_coords_in_chunks<I>(chunk_positions: I) -> impl Iterator<Item = BlockPosition>
        where I: Iterator<Item = ChunkPosition>
    {
        chunk_positions.flat_map(move |chunk_pos| {
            let chunk_block_pos: BlockPosition = Self::chunk_to_block_pos(chunk_pos);
            Self::chunk_coords().map(move |pos| chunk_block_pos + pos)
        })
    }

    /// Returns an iterator for all block positions.
    pub fn chunk_coords() -> impl Iterator<Item = BlockPosition> {
        iproduct!(0..CW as i32, 0..CH as i32, 0..CD as i32).map(|(x, y, z)|
            BlockPosition::new(x, y, z)
        )
    }

    /// Converts a given chunk position to its zero corner block position.
    #[inline]
    pub const fn chunk_to_block_pos(pos: ChunkPosition) -> BlockPosition {
        BlockPosition::new(pos.x * (CW as i32), pos.y * (CH as i32), 0)
    }

    /// Gets the chunk position a block position falls into.
    pub const fn block_to_chunk_pos(pos: BlockPosition) -> ChunkPosition {
        ChunkPosition::new(pos.x.div_euclid(CW as i32), pos.y.div_euclid(CH as i32))
    }

    /// Finds the remainder of a global position using chunk size.
    #[inline]
    pub const fn global_to_local_pos(pos: BlockPosition) -> BlockPosition {
        BlockPosition::new(pos.x.rem_euclid(CW as i32), pos.y.rem_euclid(CH as i32), pos.z)
    }

    #[inline]
    fn chunk(&self, pos: ChunkPosition) -> Result<&Chunk<CW, CH, CD, SD>, ChunkError> {
        self.chunks.get(&pos).ok_or(ChunkError::ChunkUnloaded)
    }

    #[inline]
    fn mut_chunk(&mut self, pos: ChunkPosition) -> Result<&mut Chunk<CW, CH, CD, SD>, ChunkError> {
        self.chunks.get_mut(&pos).ok_or(ChunkError::ChunkUnloaded)
    }
}
