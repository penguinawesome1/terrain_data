use std::collections::{ HashMap, HashSet };
use itertools::iproduct;
use crate::{
    chunk::Chunk,
    coords::{
        ChunkPosition,
        BlockPosition,
        block_to_chunk_pos,
        global_to_local_pos,
        chunk_to_block_pos,
        chunk_offsets,
    },
    block::Block,
};

macro_rules! impl_getter {
    ($name:ident, $sub_method:ident, $return_type:ty) => {
        pub fn $name(&self, pos: BlockPosition) -> Result<$return_type, BlockAccessError> {
            let chunk_pos: ChunkPosition = block_to_chunk_pos(pos);
            let local_pos: BlockPosition = global_to_local_pos(pos);
            Ok(self.chunk(chunk_pos)?.$sub_method(local_pos))
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $sub_method:ident) => {
        pub fn $name(&mut self, pos: BlockPosition, value: $value_type) -> Result<(), BlockAccessError> {
            let chunk_pos: ChunkPosition = block_to_chunk_pos(pos);
            let local_pos: BlockPosition = global_to_local_pos(pos);
            self.mut_chunk(chunk_pos)?.$sub_method(local_pos, value);
            self.mark_chunks_dirty_with_adj(chunk_pos);
            Ok(())
        }
    };
}

// -- Error Types --

#[derive(Debug)]
pub enum BlockAccessError {
    ChunkUnloaded,
}

// -- World --

/// Stores all chunks and marks dirty chunks.
/// Allows access and modification to them.
#[derive(Default)]
pub struct World {
    chunks: HashMap<ChunkPosition, Chunk>,
    dirty_chunks: HashSet<ChunkPosition>,
}

impl World {
    // -- ChunkManagement --

    /// Gets a result of an immutable chunk reference.
    pub fn chunk(&self, pos: ChunkPosition) -> Result<&Chunk, BlockAccessError> {
        self.chunks.get(&pos).ok_or(BlockAccessError::ChunkUnloaded)
    }

    /// Sets chunk at the passed position.
    /// Intended only for the initial creation.
    ///
    /// # Panics
    /// Panics if setting a chunk at an existing chunk position.
    pub fn set_chunk(&mut self, pos: ChunkPosition, chunk: Chunk) {
        let old_chunk: Option<Chunk> = self.chunks.insert(pos, chunk);
        debug_assert!(old_chunk.is_none(), "chunk should be absent where setting one");
    }

    /// Gets a result of a mutable chunk reference.
    pub fn mut_chunk(&mut self, pos: ChunkPosition) -> Result<&mut Chunk, BlockAccessError> {
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
    ) -> impl Iterator<Item = &Chunk> {
        positions_in_square(origin, radius).filter_map(|pos| self.chunk(pos).ok())
    }

    // -- Block Management --

    impl_getter!(block, block, Block);
    impl_getter!(sky_light, sky_light, u8);
    impl_getter!(block_light, sky_light, u8);
    impl_getter!(block_exposed, block_exposed, bool);

    impl_setter!(set_block, Block, set_block);
    impl_setter!(set_sky_light, u8, set_sky_light);
    impl_setter!(set_block_light, u8, set_block_light);
    impl_setter!(set_block_exposed, bool, set_block_exposed);

    // -- Dirty Chunk Management --

    /// Marks chunks touching the sides as dirty.
    /// Includes passed position.
    ///
    /// # Examples
    ///
    /// ```
    /// use floralcraft::terrain::chunk::ChunkPosition;
    /// use floralcraft::terrain::World;
    ///
    /// let mut world: World = World::default();
    /// let pos: ChunkPosition = ChunkPosition::new(0, 0);
    ///
    /// world.mark_chunks_dirty_with_adj(pos);
    /// ```
    pub fn mark_chunks_dirty_with_adj(&mut self, pos: ChunkPosition) {
        self.dirty_chunks.insert(pos);
        for adj_pos in chunk_offsets(pos) {
            self.dirty_chunks.insert(adj_pos);
        }
    }

    /// Gets and clears dirty chunks.
    /// Only returns valid chunk positions.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashSet;
    /// use floralcraft::terrain::chunk::ChunkPosition;
    /// use floralcraft::terrain::World;
    ///
    /// let mut world: World = World::default();
    /// let pos: ChunkPosition = ChunkPosition::new(0, 0);
    ///
    /// world.mark_chunks_dirty_with_adj(pos);
    ///
    /// let dirty_chunks = world.consume_dirty_chunks();
    /// assert!(dirty_chunks.next().is_some());
    ///
    /// let dirty_chunks = world.consume_dirty_chunks();
    /// assert!(dirty_chunks.next().is_none());
    /// ```
    pub fn consume_dirty_chunks(&mut self) -> impl Iterator<Item = ChunkPosition> {
        std::mem
            ::take(&mut self.dirty_chunks)
            .into_iter()
            .filter(|&pos| self.is_chunk_at_pos(pos))
    }
}

// -- Helper Functions --

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

/// Returns an iter for every global position found in the passed chunk positions.
pub fn global_coords_in_chunks<I>(chunk_positions: I) -> impl Iterator<Item = BlockPosition>
    where I: Iterator<Item = ChunkPosition>
{
    chunk_positions.flat_map(move |chunk_pos| {
        let chunk_block_pos: BlockPosition = chunk_to_block_pos(chunk_pos);
        Chunk::chunk_coords().map(move |pos| chunk_block_pos + pos)
    })
}
