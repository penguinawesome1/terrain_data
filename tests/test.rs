use std::sync::Arc;
use terrain_data::prelude::*;

world! {
    chunk_width: 16,
    chunk_height: 16,
    subchunk_depth: 16,
    num_subchunks: 16,
    Block r#as block: u8 = 1,
    SkyLight r#as sky_light: u8 = 5,
    Exposed r#as is_exposed: bool = 1,
}

fn main() -> Result<(), AccessError> {
    let world: Arc<World> = Arc::new(World::default());
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
