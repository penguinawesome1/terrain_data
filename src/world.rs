use std::collections::HashMap;
use itertools::iproduct;
use crate::{ chunk::{ Chunk, BlockPosition, ChunkPosition }, block::Block };

macro_rules! impl_getter {
    ($name:ident, $sub_method:ident, $return_type:ty) => {
        pub fn $name(&self, pos: BlockPosition) -> Result<$return_type, BlockAccessError> {
            let chunk_pos: ChunkPosition = Chunk::<CW, CH, CD, SD>::block_to_chunk_pos(pos);
            let local_pos: BlockPosition = Chunk::<CW, CH, CD, SD>::global_to_local_pos(pos);
            Ok(self.chunk(chunk_pos)?.$sub_method(local_pos))
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $sub_method:ident) => {
        pub fn $name(&mut self, pos: BlockPosition, value: $value_type) -> Result<(), BlockAccessError> {
            let chunk_pos: ChunkPosition = Chunk::<CW, CH, CD, SD>::block_to_chunk_pos(pos);
            let local_pos: BlockPosition = Chunk::<CW, CH, CD, SD>::global_to_local_pos(pos);
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

#[derive(Debug)]
pub enum BlockAccessError {
    ChunkUnloaded,
}

/// Stores all chunks and marks dirty chunks.
/// Allows access and modification to them.
#[derive(Default)]
pub struct World<const CW: usize, const CH: usize, const CD: usize, const SD: usize> {
    chunks: HashMap<ChunkPosition, Chunk<CW, CH, CD, SD>>,
}

impl<const CW: usize, const CH: usize, const CD: usize, const SD: usize> World<CW, CH, CD, SD> {
    impl_getter!(block, block, Block);
    impl_getter!(sky_light, sky_light, u8);
    impl_getter!(block_light, sky_light, u8);
    impl_getter!(block_exposed, block_exposed, bool);

    impl_setter!(set_block, Block, set_block);
    impl_setter!(set_sky_light, u8, set_sky_light);
    impl_setter!(set_block_light, u8, set_block_light);
    impl_setter!(set_block_exposed, bool, set_block_exposed);

    /// Gets a result of an immutable chunk reference.
    pub fn chunk(&self, pos: ChunkPosition) -> Result<&Chunk<CW, CH, CD, SD>, BlockAccessError> {
        self.chunks.get(&pos).ok_or(BlockAccessError::ChunkUnloaded)
    }

    /// Sets chunk at the passed position.
    /// Intended only for the initial creation.
    ///
    /// # Panics
    /// Panics if setting a chunk at an existing chunk position.
    pub fn set_chunk(&mut self, pos: ChunkPosition, chunk: Chunk<CW, CH, CD, SD>) {
        let old_chunk: Option<Chunk<CW, CH, CD, SD>> = self.chunks.insert(pos, chunk);
        debug_assert!(old_chunk.is_none(), "chunk should be absent where setting one");
    }

    /// Gets a result of a mutable chunk reference.
    pub fn mut_chunk(
        &mut self,
        pos: ChunkPosition
    ) -> Result<&mut Chunk<CW, CH, CD, SD>, BlockAccessError> {
        self.chunks.get_mut(&pos).ok_or(BlockAccessError::ChunkUnloaded)
    }

    /// Returns bool for if a chunk is found at the passed position.
    pub fn is_chunk_at_pos(&self, pos: ChunkPosition) -> bool {
        self.chunks.contains_key(&pos)
    }

    /// Gets an iter of all chunks in a square around the passed origin position.
    /// Radius of 0 results in 1 chunk.
    pub fn chunks_in_square(
        &self,
        origin: ChunkPosition,
        radius: u32
    ) -> impl Iterator<Item = &Chunk<CW, CH, CD, SD>> {
        Self::positions_in_square(origin, radius).filter_map(|pos| self.chunk(pos).ok())
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
    pub fn chunk_offsets(pos: ChunkPosition) -> impl Iterator<Item = ChunkPosition> {
        CHUNK_ADJ_OFFSETS.iter().map(move |offset| { pos + offset })
    }

    /// Returns all adjacent block offsets.
    /// Filters out illegal vertical offsets.
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
            let chunk_block_pos: BlockPosition = Chunk::<CW, CH, CD, SD>::chunk_to_block_pos(
                chunk_pos
            );
            Chunk::<CW, CH, CD, SD>::chunk_coords().map(move |pos| chunk_block_pos + pos)
        })
    }
}
