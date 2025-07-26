#![allow(dead_code)]

/// Macro to create a new world.
///
/// # Examples
///
/// ```
/// use floralcraft_terrain::make_world;
///
/// make_world! {
///     chunk_width: 16,
///     chunk_height: 16,
///     subchunk_depth: 16,
///     num_subchunks: 16,
///     Block r#as block: u8 = 1,
///     SkyLight r#as sky_light: u8 = 1,
///     Exposed r#as is_exposed: bool = 1,
/// }
/// ```
#[macro_export]
macro_rules! make_world {
    (
        chunk_width: $chunk_width:expr,
        chunk_height: $chunk_height:expr,
        subchunk_depth: $subchunk_depth:expr,
        num_subchunks: $num_subchunks:expr,
        $(
            $field_name_enum:ident r#as $field_name_method:ident: $field_type:ty = $bits_per_item:expr
        ),*
        $(,)?
    ) => {
        use serde::{ Serialize, Deserialize };
        use chroma::{ Section, BoundsError };
        use paste::paste;
        use glam::{ IVec3, IVec2 };
        use ahash::AHasher;
        use itertools::iproduct;
        use thiserror::Error;
        use std::{
            hash::BuildHasherDefault,
            collections::{ HashMap, hash_map::Entry },
            path::PathBuf,
        };
        use std::{
            io::{ Write, self },
            fs,
        };
        use bincode::{
            serde as bincode_serde,
            config,
            error::{ EncodeError, DecodeError },
            serde::encode_to_vec,
        };

        /// Stores the three dimensional integer position of a block.
        pub type BlockPosition = IVec3;

        /// Stores the two dimensional integer position of a chunk.
        pub type ChunkPosition = IVec2;

        const SUBCHUNK_DEPTH: usize = $subchunk_depth as usize;
        const NUM_SUBCHUNKS: usize = $num_subchunks as usize;
        const CHUNK_WIDTH: usize = $chunk_width as usize;
        const CHUNK_HEIGHT: usize = $chunk_height as usize;
        const CHUNK_DEPTH: usize = SUBCHUNK_DEPTH * NUM_SUBCHUNKS;

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

        pub trait FieldType: Sized {
            fn from_u64(v: u64) -> Self;
            fn to_u64(self) -> u64;
        }

        impl FieldType for u8 {
            #[inline(always)]
            fn from_u64(v: u64) -> Self { v as Self }
            #[inline(always)]
            fn to_u64(self) -> u64 { self as u64 }
        }

        impl FieldType for bool {
            #[inline(always)]
            fn from_u64(v: u64) -> Self { v != 0 }
            #[inline(always)]
            fn to_u64(self) -> u64 { self as u64 }
        }

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

        // -- SectionField --

        #[derive(Clone, Copy, Serialize, Deserialize)]
        pub enum SectionField {
            $($field_name_enum),*,
            #[doc(hidden)]
            __COUNT
        }

        impl SectionField {
            pub const COUNT: usize = Self::__COUNT as usize;
            const BITS_PER_ITEM_TABLE: &'static [u8] = &[$($bits_per_item),*];

            pub const fn bits(&self) -> u8 {
                Self::BITS_PER_ITEM_TABLE[*self as usize]
            }
        }

        // -- Subchunk --

        #[derive(Default, Serialize, Deserialize)]
        pub struct Subchunk {
            sections: [Option<Section<CHUNK_WIDTH, CHUNK_HEIGHT, SUBCHUNK_DEPTH>>; SectionField::COUNT],
        }

        impl Subchunk {
            #[inline]
            pub fn is_empty(&self) -> bool {
                self.sections.iter().all(Option::is_none)
            }

            // getters

            $(
                #[inline]
                pub fn $field_name_method(&self, pos: BlockPosition) -> Result<$field_type, BoundsError> {
                    // Ok(self.item(SectionField::$field_name_enum, pos)? as $field_type)

                    Ok(
                        <$field_type as FieldType>::from_u64(
                            self.item(SectionField::$field_name_enum, pos)?
                        )
                    )
                }
            )*

            // setters

            paste! {
                $(
                    #[must_use]
                    #[inline]
                    pub fn [<set_ $field_name_method>](
                        &mut self,
                        pos: BlockPosition,
                        value: $field_type
                    ) -> Result<(), BoundsError> {
                        self.set_item(
                            SectionField::$field_name_enum, pos, <$field_type as FieldType>::to_u64(value)
                        )?;
                        Ok(())
                    }
                )*
            }

            #[inline]
            fn item(&self, section_field: SectionField, pos: BlockPosition) -> Result<u64, BoundsError> {
                self.sections[section_field as usize].as_ref().map_or(Ok(0), |s| s.item(pos))
            }

            #[must_use]
            #[inline]
            fn set_item(
                &mut self,
                section_field: SectionField,
                pos: BlockPosition,
                value: u64
            ) -> Result<(), BoundsError> {
                let section_index = section_field as usize;

                if value == 0 && self.sections[section_index].is_none() {
                    return Ok(());
                }

                let section = self.sections[section_index].get_or_insert_with(||
                    Section::new(section_field.bits())
                );

                section.set_item(pos, value)?;

                if section.is_empty() {
                    self.sections[section_index] = None;
                }

                Ok(())
            }
        }

        // -- Chunk --

        #[derive(Default, Serialize, Deserialize)]
        pub struct Chunk {
            subchunks: [Option<Subchunk>; NUM_SUBCHUNKS],
        }

        impl Chunk {
            // getters

            $(
                #[inline]
                pub fn $field_name_method(&self, pos: BlockPosition) -> Result<$field_type, BoundsError> {
                    let index: usize = Self::subchunk_index(pos.z);

                    let Some(subchunk_opt) = self.subchunks.get(index) else {
                        return Err(BoundsError::OutOfBounds(pos));
                    };

                    subchunk_opt.as_ref().map_or(Ok(<$field_type as FieldType>::from_u64(0)), |s| {
                        let sub_pos: BlockPosition = Self::local_to_sub(pos);
                        Ok(s.$field_name_method(sub_pos)?)
                    })
                }
            )*

            // setters

            paste! {
                $(
                    #[must_use]
                    #[inline]
                    pub fn [<set_ $field_name_method>](
                        &mut self,
                        pos: BlockPosition,
                        value: $field_type
                    ) -> Result<(), BoundsError> {
                        let index: usize = Self::subchunk_index(pos.z);

                        let Some(subchunk_opt) = self.subchunks.get_mut(index) else {
                            return Err(BoundsError::OutOfBounds(pos));
                        };

                        if <$field_type as FieldType>::to_u64(value) == 0 && subchunk_opt.is_none() {
                            return Ok(()); // return if placement is redundant
                        }

                        let subchunk: &mut Subchunk = subchunk_opt.get_or_insert_with(|| Subchunk::default());
                        let sub_pos: BlockPosition = Self::local_to_sub(pos);

                        subchunk.[<set_ $field_name_method>](sub_pos, value)?;

                        if subchunk.is_empty() {
                            *subchunk_opt = None; // set empty subchunks to none
                        }

                        Ok(())
                    }
                )*
            }

            #[inline]
            const fn subchunk_index(pos_z: i32) -> usize {
                (pos_z as usize).div_euclid(SUBCHUNK_DEPTH)
            }

            #[inline]
            const fn local_to_sub(pos: BlockPosition) -> BlockPosition {
                BlockPosition::new(pos.x, pos.y, pos.z.rem_euclid(SUBCHUNK_DEPTH as i32))
            }
        }

        // -- World --

        /// Stores all chunks and marks dirty chunks.
        /// Allows access and modification to them.
        #[derive(Default)]
        pub struct World {
            chunks: HashMap<ChunkPosition, Chunk, BuildHasherDefault<AHasher>>,
        }

        impl World {
            // getters

            $(
                #[inline]
                pub fn $field_name_method(&self, pos: BlockPosition) -> Result<$field_type, AccessError> {
                    let chunk_pos: ChunkPosition = Self::block_to_chunk_pos(pos);
                    let local_pos: BlockPosition = Self::global_to_local_pos(pos);
                    Ok(self.chunk(chunk_pos)?.$field_name_method(local_pos)?)
                }
            )*

            // setters

            paste! {
                $(
                    #[must_use]
                    #[inline]
                    pub fn [<set_ $field_name_method>](
                        &mut self,
                        pos: BlockPosition,
                        value: $field_type
                    ) -> Result<(), AccessError> {
                        let chunk_pos: ChunkPosition = Self::block_to_chunk_pos(pos);
                        let local_pos: BlockPosition = Self::global_to_local_pos(pos);
                        self.chunk_mut(chunk_pos)?.[<set_$field_name_method>](local_pos, value)?;
                        Ok(())
                    }
                )*
            }

            /// Returns bool for if a chunk is found at the passed position.
            pub fn is_chunk_at_pos(&self, pos: ChunkPosition) -> bool {
                self.chunks.contains_key(&pos)
            }

            /// Sets new blank chunk at the passed position.
            /// Returns an error if a chunk is already at the position.
            #[must_use]
            pub fn add_empty_chunk(&mut self, pos: ChunkPosition) -> Result<(), ChunkOverwriteError> {
                match self.chunks.entry(pos) {
                    Entry::Occupied(_) => Err(ChunkOverwriteError::ChunkAlreadyLoaded(pos)),
                    Entry::Vacant(entry) => {
                        let chunk: Chunk = Chunk::default();
                        entry.insert(chunk);
                        Ok(())
                    }
                }
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

                iproduct!(0..CHUNK_WIDTH as i32, 0..CHUNK_HEIGHT as i32, 0..CHUNK_DEPTH as i32).map(
                    move |(x, y, z)| base_block_pos + BlockPosition::new(x, y, z)
                )
            }

            /// Converts a given chunk position to its zero corner block position.
            #[inline]
            pub const fn chunk_to_block_pos(pos: ChunkPosition) -> BlockPosition {
                BlockPosition::new(pos.x * (CHUNK_WIDTH as i32), pos.y * (CHUNK_HEIGHT as i32), 0)
            }

            /// Gets the chunk position a block position falls into.
            #[inline]
            pub const fn block_to_chunk_pos(pos: BlockPosition) -> ChunkPosition {
                ChunkPosition::new(pos.x.div_euclid(CHUNK_WIDTH as i32), pos.y.div_euclid(CHUNK_HEIGHT as i32))
            }

            /// Finds the remainder of a global position using chunk size.
            #[inline]
            pub const fn global_to_local_pos(pos: BlockPosition) -> BlockPosition {
                BlockPosition::new(
                    pos.x.rem_euclid(CHUNK_WIDTH as i32),
                    pos.y.rem_euclid(CHUNK_HEIGHT as i32),
                    pos.z
                )
            }

            pub fn unload_chunk(&mut self, pos: ChunkPosition) -> Result<(), ChunkStoreError> {
                let chunk: Chunk = self.chunks
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
                    return Err(ChunkStoreError::ChunkOverwrite(ChunkOverwriteError::ChunkAlreadyLoaded(pos)));
                }

                let path: PathBuf = PathBuf::from(CHUNKS_DIR).join(format!("{}_{}.bin", pos.x, pos.y));
                let encoded_data: Vec<u8> = fs::read(&path)?;

                let (chunk, _): (Chunk, usize) = bincode_serde::decode_from_slice(
                    &encoded_data,
                    config::standard()
                )?;

                self.chunks.insert(pos, chunk);

                Ok(())
            }

            #[inline]
            fn chunk(&self, pos: ChunkPosition) -> Result<&Chunk, ChunkAccessError> {
                self.chunks.get(&pos).ok_or(ChunkAccessError::ChunkUnloaded(pos))
            }

            #[inline]
            fn chunk_mut(
                &mut self,
                pos: ChunkPosition
            ) -> Result<&mut Chunk, ChunkAccessError> {
                self.chunks.get_mut(&pos).ok_or(ChunkAccessError::ChunkUnloaded(pos))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    make_world! {
        chunk_width: 16,
        chunk_height: 16,
        subchunk_depth: 16,
        num_subchunks: 16,
        Block r#as block: u8 = 1,
        SkyLight r#as sky_light: u8 = 1,
        Exposed r#as is_exposed: bool = 1,
    }

    #[test]
    fn test_get_and_set_subchunk() -> Result<(), BoundsError> {
        let mut subchunk: Subchunk = Subchunk::default();
        let pos_1: BlockPosition = BlockPosition::new(15, 1, 1);
        let pos_2: BlockPosition = BlockPosition::new(3, 0, 2);

        subchunk.set_block(pos_1, 0)?;
        subchunk.set_block(pos_1, 4)?;
        subchunk.set_block(pos_2, 5)?;

        assert_eq!(subchunk.block(pos_1)?, 4);
        assert_eq!(subchunk.block(pos_2)?, 5);

        Ok(())
    }

    #[test]
    fn test_get_and_set_chunk() -> Result<(), BoundsError> {
        let mut chunk: Chunk = Chunk::default();
        let pos_1: BlockPosition = BlockPosition::new(15, 1, 200);
        let pos_2: BlockPosition = BlockPosition::new(3, 0, 2);

        chunk.set_block(pos_1, 0)?;
        chunk.set_block(pos_1, 4)?;
        chunk.set_block(pos_2, 5)?;

        assert_eq!(chunk.block(pos_1)?, 4);
        assert_eq!(chunk.block(pos_2)?, 5);

        Ok(())
    }

    #[test]
    fn test_get_and_set_world() -> Result<(), AccessError> {
        let mut world: World = World::default();
        let chunk_pos: ChunkPosition = ChunkPosition::new(0, 0);
        world.add_empty_chunk(chunk_pos).unwrap();

        let pos_1: BlockPosition = BlockPosition::new(15, 1, 200);
        let pos_2: BlockPosition = BlockPosition::new(3, 0, 2);

        world.set_is_exposed(pos_1, true)?;
        world.set_is_exposed(pos_1, false)?;
        world.set_is_exposed(pos_2, true)?;

        assert_eq!(world.is_exposed(pos_1)?, false);
        assert_eq!(world.is_exposed(pos_2)?, true);

        Ok(())
    }

    #[test]
    fn test_save_load_chunk() -> Result<(), ChunkStoreError> {
        let mut world: World = World::default();
        let chunk_pos: ChunkPosition = ChunkPosition::new(0, 0);
        let pos: BlockPosition = BlockPosition::new(1, 2, 3);

        world.add_empty_chunk(chunk_pos)?;
        world.set_block(pos, 3)?;

        world.unload_chunk(chunk_pos)?;

        assert!(world.block(pos).is_err());

        world.load_chunk(chunk_pos)?;

        let block: u8 = world.block(pos)?;
        assert!(block == 3);

        Ok(())
    }
}
