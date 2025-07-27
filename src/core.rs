use glam::{IVec2, IVec3};

/// Stores the three dimensional integer position of a block.
pub type BlockPosition = IVec3;

/// Stores the two dimensional integer position of a chunk.
pub type ChunkPosition = IVec2;

pub const CHUNKS_DIR: &str = "chunks";

pub const CHUNK_ADJ_OFFSETS: [ChunkPosition; 4] = [
    ChunkPosition::new(-1, 0),
    ChunkPosition::new(1, 0),
    ChunkPosition::new(0, -1),
    ChunkPosition::new(0, 1),
];

pub const BLOCK_OFFSETS: [BlockPosition; 6] = [
    BlockPosition::new(1, 0, 0),
    BlockPosition::new(0, 1, 0),
    BlockPosition::new(0, 0, 1),
    BlockPosition::new(-1, 0, 0),
    BlockPosition::new(0, -1, 0),
    BlockPosition::new(0, 0, -1),
];

pub trait FieldType: Sized {
    fn from_u64(v: u64) -> Self;
    fn to_u64(self) -> u64;
}

impl FieldType for u8 {
    #[inline(always)]
    fn from_u64(v: u64) -> Self {
        v as Self
    }
    #[inline(always)]
    fn to_u64(self) -> u64 {
        self as u64
    }
}

impl FieldType for bool {
    #[inline(always)]
    fn from_u64(v: u64) -> Self {
        v != 0
    }
    #[inline(always)]
    fn to_u64(self) -> u64 {
        self as u64
    }
}
