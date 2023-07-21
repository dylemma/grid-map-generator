use std::ops::DerefMut;

use bevy::prelude::*;

use crate::grid::*;
use crate::noise::Noise;
use crate::procgen::*;

pub struct ZonePlugin(pub u32, pub u32);

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ZoneNoise(Noise::new()))
            .insert_resource(Grid::<TileState>::new(self.0, self.1))
            .insert_resource(GridDimensions::new([self.0, self.1]))
            .add_event::<ZoneCommand>()
            .add_systems(Startup, startup_init_zone)
            .add_systems(Update, handle_zone_commands)
        ;
    }
}

#[derive(Resource)]
struct ZoneNoise(Noise);

#[derive(Event)]
pub enum ZoneCommand {
    Regenerate,
}

fn startup_init_zone(
    dimensions: Res<GridDimensions>,
    mut tiles: ResMut<Grid<TileState>>,
    zone_noise: Res<ZoneNoise>,
) {
    generate_island_into(&dimensions, &zone_noise.0, tiles.deref_mut(), TileState::from);
}

fn handle_zone_commands(
    mut zone_commands: EventReader<ZoneCommand>,
    dimensions: Res<GridDimensions>,
    mut tiles: ResMut<Grid<TileState>>,
    mut zone_noise: ResMut<ZoneNoise>,
) {
    for cmd in zone_commands.iter() {
        match cmd {
            ZoneCommand::Regenerate => {
                zone_noise.0.reseed();
                generate_island_into(&dimensions, &zone_noise.0, &mut tiles, TileState::from);
            },
        }
    }
}

#[derive(Component, Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum TileState {
    #[default]
    Floor,
    Water,
}

impl TileState {
    pub fn as_color(&self) -> Color {
        match self {
            TileState::Floor => Color::WHITE,
            TileState::Water => Color::rgb(0.0, 0.1, 0.4),
        }
    }
}

impl From<Reachability> for TileState {
    fn from(r: Reachability) -> Self {
        match r {
            Reachability::Closed => TileState::Water,
            Reachability::Open => TileState::Floor,
        }
    }
}


#[derive(Copy, Clone, Resource)]
pub struct GridDimensions {
    pub size_in_tiles: [u32; 2],
    pub tile_size: f32,
    pub bottom_left: Vec2,
}

impl GridDimensions {
    pub fn new(size_in_tiles: [u32; 2]) -> GridDimensions {
        GridDimensions {
            size_in_tiles,
            tile_size: 1.,
            bottom_left: Vec2::ZERO,
        }
    }
    pub fn world_center(&self) -> Vec2 {
        self.bottom_left + Vec2::new(self.world_width() * 0.5, self.world_height() * 0.5)
    }
    pub fn world_width(&self) -> f32 {
        self.tile_size * self.size_in_tiles[0] as f32
    }
    pub fn world_height(&self) -> f32 {
        self.tile_size * self.size_in_tiles[1] as f32
    }
    pub fn world_pos_of(&self, tile: &TileAddress) -> Vec2 {
        self.bottom_left + Vec2::new(
            tile.0 as f32 * self.tile_size,
            tile.1 as f32 * self.tile_size,
        )
    }

    pub fn position_to_address(&self, position: Vec2) -> Option<TileAddress> {
        let rel_pos = ((position - self.bottom_left) / self.tile_size).floor();
        let tile_x = u32::try_from(rel_pos.x as i32).ok()?;
        let tile_y = u32::try_from(rel_pos.y as i32).ok()?;
        if tile_x < self.size_in_tiles[0] && tile_y < self.size_in_tiles[1] {
            Some(TileAddress(tile_x, tile_y))
        } else {
            None
        }
    }

    // return a new Vec2 which represents the given `point`'s position relative to
    // the `world_center`, scaled relative to size of the grid, such that for a
    // `point` inside the grid, the magnitude of the x and y components of the returned
    // vector will be at most 1.
    pub fn normalize_from_center(&self, point: Vec2) -> Vec2 {
        let Vec2 { x, y } = point;
        Vec2 {
            x: 2.0 * (x - self.bottom_left.x) / self.world_width() - 1.0,
            y: 2.0 * (y - self.bottom_left.y) / self.world_height() - 1.0,
        }
    }
}