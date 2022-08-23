use bevy::prelude::*;

use crate::{GridDimensions, TileAddress};
use crate::noise::Noise;

pub struct TileWigglePlugin;

impl Plugin for TileWigglePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(WiggleNoise::new())
            .add_system(wiggle_tiles)
        ;
    }
}

const WIGGLE_MAGNITUDE: f32 = 0.5;

fn wiggle_tiles(grid: Res<GridDimensions>, time: Res<Time>, noise: Res<WiggleNoise>, mut tiles: Query<(&mut TileWiggle, &mut Transform, &TileAddress)>) {
    for (mut tile_wiggle, mut transform, tile) in &mut tiles {
        tile_wiggle.step(&time);
        let base_pos = grid.world_pos_of(tile);
        let noise_offset = noise.get_at(grid.world_pos_of(tile) + tile_wiggle.as_offset()) * WIGGLE_MAGNITUDE;
        *transform = Transform::from_translation((base_pos + noise_offset, 0.).into());
    }
}

#[derive(Component)]
pub struct TileWiggle {
    dt: f32,
    frequency: f32,
}

impl TileWiggle {
    const WIGGLE_FREQUENCY: f32 = 0.25;
    pub fn new() -> Self {
        TileWiggle {
            dt: 0.,
            frequency: Self::WIGGLE_FREQUENCY,
        }
    }
    fn step(&mut self, time: &Time) {
        self.dt += time.delta_seconds() * self.frequency * std::f32::consts::TAU;
        self.dt = self.dt % std::f32::consts::TAU;
    }
    fn as_offset(&self) -> Vec2 {
        Vec2::new(
            self.dt.cos(),
            self.dt.sin(),
        )
    }
}

struct WiggleNoise(Noise, Noise);

impl WiggleNoise {
    pub fn new() -> Self {
        WiggleNoise(Noise::new(), Noise::new())
    }
    fn get_at(&self, point: Vec2) -> Vec2 {
        Vec2::new(
            self.0.get_at(point),
            self.1.get_at(point),
        )
    }
}