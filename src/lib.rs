mod chunk;
mod subchunk;

use std::{
    hash::BuildHasherDefault,
    collections::{ HashMap, hash_map::Entry },
    io::{ Write, self },
    fs,
};
use bincode::{
    serde as bincode_serde,
    config,
    error::{ EncodeError, DecodeError },
    serde::encode_to_vec,
};
use serde::{ Serialize, Deserialize };
use ahash::AHasher;
use itertools::iproduct;
use thiserror::Error;
use std::path::PathBuf;
use crate::{ subchunk::Subchunk, chunk::Chunk };
use chroma::BoundsError;

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

const CHUNKS_DIR: &str = "chunks";

macro_rules! impl_getter {
    ($name:ident, $sub_method:ident, $return_type:ty) => {
        pub fn $name(&self, pos: BlockPosition) -> Result<$return_type, AccessError> {
            let chunk_pos: ChunkPosition = Self::block_to_chunk_pos(pos);
            let local_pos: BlockPosition = Self::global_to_local_pos(pos);
            Ok(self.chunk(chunk_pos)?.$sub_method(local_pos)?)
        }
    };
}

macro_rules! impl_setter {
    ($name:ident, $value_type:ty, $sub_method:ident) => {
        pub fn $name(&mut self, pos: BlockPosition, value: $value_type) -> Result<(), AccessError> {
            let chunk_pos: ChunkPosition = Self::block_to_chunk_pos(pos);
            let local_pos: BlockPosition = Self::global_to_local_pos(pos);
            self.chunk_mut(chunk_pos)?.$sub_method(local_pos, value)?;
            Ok(())
        }
    };
}

/// Stores the two dimensional integer position of a chunk.
pub type ChunkPosition = glam::IVec2;

/// Stores the three dimensional integer position of a block.
pub type BlockPosition = glam::IVec3;

#[derive(Debug, Error)]
pub enum AccessError {
    #[error(transparent)] ChunkAccess(#[from] ChunkAccessError),
    #[error(transparent)] Bounds(#[from] BoundsError),
}

#[derive(Debug, Error)]
pub enum ChunkAccessError {
    #[error("Chunk {0:?} is currently unloaded.")] ChunkUnloaded(ChunkPosition),
}

#[derive(Debug, Error)]
pub enum ChunkOverwriteError {
    #[error("Chunk {0:?} already exists.")] ChunkAlreadyLoaded(ChunkPosition),
}

#[derive(Debug, Error)]
pub enum ChunkStoreError {
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Access(#[from] AccessError),
    #[error(transparent)] ChunkOverwrite(#[from] ChunkOverwriteError),
    #[error(transparent)] Encode(#[from] EncodeError),
    #[error(transparent)] Decode(#[from] DecodeError),
}

/// Stores all chunks and marks dirty chunks.
/// Allows access and modification to them.
#[derive(Default)]
pub struct World<const CW: usize, const CH: usize, const SD: usize, const NS: usize>
    where
        for<'a> [Option<Subchunk<CW, CH, SD>>; NS]: Sized + Default + Serialize + Deserialize<'a> {
    chunks: HashMap<ChunkPosition, Chunk<CW, CH, SD, NS>, BuildHasherDefault<AHasher>>,
}

impl<const CW: usize, const CH: usize, const SD: usize, const NS: usize> World<CW, CH, SD, NS>
    where for<'a> [Option<Subchunk<CW, CH, SD>>; NS]: Sized + Default + Serialize + Deserialize<'a>
{
    const CD: usize = SD * NS;

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
    pub fn add_default_chunk(&mut self, pos: ChunkPosition) -> Result<(), ChunkOverwriteError> {
        match self.chunks.entry(pos) {
            Entry::Occupied(_) => Err(ChunkOverwriteError::ChunkAlreadyLoaded(pos)),
            Entry::Vacant(entry) => {
                let chunk: Chunk<CW, CH, SD, NS> = Chunk::default();
                entry.insert(chunk);
                Ok(())
            }
        }
    }

    /// Closure allowing modifications of a chunk using its inward functions.
    /// Iterates through its global positions.
    #[must_use]
    pub fn decorate_chunk<F>(
        &mut self,
        chunk_pos: ChunkPosition,
        mut f: F
    ) -> Result<(), ChunkAccessError>
        where F: FnMut(&mut Chunk<CW, CH, SD, NS>, BlockPosition)
    {
        let chunk: &mut Chunk<CW, CH, SD, NS> = self.chunk_mut(chunk_pos)?;

        for pos in Self::chunk_coords(ChunkPosition::ZERO) {
            f(chunk, pos);
        }

        Ok(())
    }

    /// Returns an iter for every block type and global position pair found in the chunk.
    /// Filters out blocks that are not exposed.
    pub fn chunk_render_data(
        &self,
        chunk_pos: ChunkPosition
    ) -> Result<impl Iterator<Item = (u8, BlockPosition)>, ChunkAccessError> {
        let chunk: &Chunk<CW, CH, SD, NS> = self.chunk(chunk_pos)?;
        let origin_block_pos: BlockPosition = Self::chunk_to_block_pos(chunk_pos);

        Ok(
            Self::chunk_coords(ChunkPosition::ZERO)
                .filter(|&pos| chunk.block_exposed(pos).unwrap())
                .map(move |pos| (chunk.block(pos).unwrap(), origin_block_pos + pos))
        )
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
    #[inline]
    pub fn block_offsets(pos: BlockPosition) -> impl Iterator<Item = BlockPosition> {
        BLOCK_OFFSETS.iter().map(move |offset| { pos + offset })
    }

    /// Returns an iter for every global position found in the passed chunk positions.
    pub fn coords_in_chunks<I>(chunk_positions: I) -> impl Iterator<Item = BlockPosition>
        where I: Iterator<Item = ChunkPosition>
    {
        chunk_positions.flat_map(move |chunk_pos| Self::chunk_coords(chunk_pos))
    }

    /// Returns an iter for all block positions in the chunk offset by the chunk position.
    /// Passing in zero offset returns local positions.
    pub fn chunk_coords(offset: ChunkPosition) -> impl Iterator<Item = BlockPosition> {
        let base_block_pos: BlockPosition = Self::chunk_to_block_pos(offset);

        iproduct!(0..CW as i32, 0..CH as i32, 0..Self::CD as i32).map(
            move |(x, y, z)| base_block_pos + BlockPosition::new(x, y, z)
        )
    }

    /// Converts a given chunk position to its zero corner block position.
    #[inline]
    pub const fn chunk_to_block_pos(pos: ChunkPosition) -> BlockPosition {
        BlockPosition::new(pos.x * (CW as i32), pos.y * (CH as i32), 0)
    }

    /// Gets the chunk position a block position falls into.
    #[inline]
    pub const fn block_to_chunk_pos(pos: BlockPosition) -> ChunkPosition {
        ChunkPosition::new(pos.x.div_euclid(CW as i32), pos.y.div_euclid(CH as i32))
    }

    /// Finds the remainder of a global position using chunk size.
    #[inline]
    pub const fn global_to_local_pos(pos: BlockPosition) -> BlockPosition {
        BlockPosition::new(pos.x.rem_euclid(CW as i32), pos.y.rem_euclid(CH as i32), pos.z)
    }

    pub fn unload_chunk(&mut self, pos: ChunkPosition) -> Result<(), ChunkStoreError> {
        let chunk: Chunk<CW, CH, SD, NS> = self.chunks
            .remove(&pos)
            .ok_or(AccessError::ChunkAccess(ChunkAccessError::ChunkUnloaded(pos)))?;

        fs::create_dir_all(CHUNKS_DIR)?;
        let path: PathBuf = PathBuf::from(CHUNKS_DIR).join(format!("{}_{}.bin", pos.x, pos.y));
        let mut file: fs::File = fs::File::create(&path)?;

        let encoded_data = encode_to_vec(&chunk, config::standard())?;

        file.write_all(&encoded_data)?;

        Ok(())
    }

    #[must_use]
    pub fn load_chunk(&mut self, pos: ChunkPosition) -> Result<(), ChunkStoreError> {
        if self.is_chunk_at_pos(pos) {
            return Err(
                ChunkStoreError::ChunkOverwrite(ChunkOverwriteError::ChunkAlreadyLoaded(pos))
            );
        }

        let path: PathBuf = PathBuf::from(CHUNKS_DIR).join(format!("{}_{}.bin", pos.x, pos.y));
        let encoded_data: Vec<u8> = fs::read(&path)?;

        let (chunk, _): (Chunk<CW, CH, SD, NS>, usize) = bincode_serde::decode_from_slice(
            &encoded_data,
            config::standard()
        )?;

        self.chunks.insert(pos, chunk);

        Ok(())
    }

    #[inline]
    fn chunk(&self, pos: ChunkPosition) -> Result<&Chunk<CW, CH, SD, NS>, ChunkAccessError> {
        self.chunks.get(&pos).ok_or(ChunkAccessError::ChunkUnloaded(pos))
    }

    #[inline]
    fn chunk_mut(
        &mut self,
        pos: ChunkPosition
    ) -> Result<&mut Chunk<CW, CH, SD, NS>, ChunkAccessError> {
        self.chunks.get_mut(&pos).ok_or(ChunkAccessError::ChunkUnloaded(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_load_chunk() -> Result<(), ChunkStoreError> {
        let mut world: World<16, 16, 16, 4> = World::default();
        let chunk_pos: ChunkPosition = ChunkPosition::new(0, 0);
        let pos: BlockPosition = BlockPosition::new(1, 2, 3);

        world.add_default_chunk(chunk_pos)?;
        world.set_block(pos, 3)?;

        world.unload_chunk(chunk_pos)?;

        assert!(world.block(pos).is_err());

        world.load_chunk(chunk_pos)?;

        let block: u8 = world.block(pos)?;
        assert!(block == 3);

        Ok(())
    }
}
