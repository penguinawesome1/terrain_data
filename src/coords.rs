use crate::chunk::{ CHUNK_WIDTH, CHUNK_HEIGHT, CHUNK_DEPTH };

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

/// Converts a given chunk position to its zero corner block position.
pub const fn chunk_to_block_pos(pos: ChunkPosition) -> BlockPosition {
    BlockPosition::new(pos.x * (CHUNK_WIDTH as i32), pos.y * (CHUNK_HEIGHT as i32), 0)
}

/// Gets the chunk position a block position falls into.
pub const fn block_to_chunk_pos(pos: BlockPosition) -> ChunkPosition {
    ChunkPosition::new(pos.x.div_euclid(CHUNK_WIDTH as i32), pos.y.div_euclid(CHUNK_HEIGHT as i32))
}

/// Finds the remainder of a global position using chunk size.
pub const fn global_to_local_pos(pos: BlockPosition) -> BlockPosition {
    BlockPosition::new(
        pos.x.rem_euclid(CHUNK_WIDTH as i32),
        pos.y.rem_euclid(CHUNK_HEIGHT as i32),
        pos.z
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
        .filter(|adj_pos| { adj_pos.z >= 0 && adj_pos.z < (CHUNK_DEPTH as i32) })
}
