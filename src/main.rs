mod grid;

use std::time::Duration;
use bevy::{
    prelude::*,
    render::camera::ScalingMode,
};
use bevy::render::camera::{DepthCalculation, WindowOrigin};
use bevy::sprite::Anchor;
use noise::{NoiseFn, OpenSimplex, Seedable};
use rand::prelude::*;

use crate::grid::*;

const GRID_SIZE: [u32; 2] = [50, 50];
const AUTOMATA_STEP_PERIOD: f32 = 0.1;
const AUTOMATA_OPEN_THRESHOLD: u32 = 4;
const AUTOMATA_CLOSE_THRESHOLD: u32 = 4;

const WIGGLE_MAGNITUDE: f32 = 0.5;
const WIGGLE_FREQUENCY: f32 = 0.25;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GridPlugin::new(GRID_SIZE))
        .add_plugin(NoisePlugin)
        .add_startup_system(setup)
        .add_system(reset_tiles_on_keypress)
        .add_system(cellular_automata)
        .add_system(wiggle_tiles)
        .add_system(sync_tile_sprites)
        .run();
}

fn setup(
    mut commands: Commands,
) {
    let mut grid = Grid::new(GRID_SIZE[0], GRID_SIZE[1]);
    let grid_dims = GridDimensions::new(GRID_SIZE);

    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            window_origin: WindowOrigin::Center,
            depth_calculation: DepthCalculation::ZDifference,
            scaling_mode: ScalingMode::Auto {
                min_width: grid_dims.world_width(),
                min_height: grid_dims.world_height(),
            },
            ..default()
        },
        transform: Transform::from_translation((grid_dims.world_center(), 0.).into()),
        ..default()
    });

    let mut rng = thread_rng();

    for i in 0..grid.width() {
        for j in 0..grid.height() {
            let tile_address = TileAddress(i, j);
            let tile_state = rng.gen::<TileState>();
            let pos = grid_dims.world_pos_of(&tile_address);

            let tile_entity = commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::ONE),
                        anchor: Anchor::BottomLeft,
                        ..default()
                    },
                    transform: Transform::from_translation((pos, 0.).into()),
                    ..default()
                })
                .insert(tile_address)
                .insert(tile_state)
                .insert(TileWiggle::new())
                ;
        }
    }

    commands.insert_resource(grid);
    commands.insert_resource(AutomataTimer(Timer::new(Duration::from_secs_f32(AUTOMATA_STEP_PERIOD), true)));
}

// Sync changed TileState values back to the Grid and update associated sprite colors
fn sync_tile_sprites(
    mut grid: ResMut<Grid>,
    mut tile_sprites: Query<(&mut Sprite, &TileAddress, &TileState), Changed<TileState>>,
) {
    for (mut sprite, tile_address, tile_state) in &mut tile_sprites {
        grid[*tile_address].state = *tile_state;
        let color = match tile_state {
            TileState::Floor => Color::WHITE,
            TileState::Wall => Color::BLACK,
        };
        sprite.color = color;
    }
}

fn reset_tiles_on_keypress(keyboard: Res<Input<KeyCode>>, mut tiles_states: Query<&mut TileState>) {
    if keyboard.just_pressed(KeyCode::Return) {
        let mut rng = thread_rng();
        for mut state in &mut tiles_states {
            *state = rng.gen();
        }
    }
}

fn wiggle_tiles(grid: Res<GridDimensions>, time: Res<Time>, noise: Res<NoiseFunctions>, mut tiles: Query<(&mut TileWiggle, &mut Transform, &TileAddress)>) {
    for (mut tile_wiggle, mut transform, tile) in &mut tiles {
        tile_wiggle.step(&time);
        let base_pos = grid.world_pos_of(tile);
        let noise_offset = noise.get_offset_2d(grid.world_pos_of(tile) + tile_wiggle.as_offset()) * WIGGLE_MAGNITUDE;
        *transform = Transform::from_translation((base_pos + noise_offset, 0.).into());
    }
}

#[derive(Component)]
pub struct TileWiggle {
    dt: f32,
    frequency: f32,
    noise_root: [f32; 2],
}

impl TileWiggle {
    fn new() -> Self {
        let mut rng = thread_rng();
        TileWiggle {
            dt: 0.,
            frequency: WIGGLE_FREQUENCY,
            noise_root: rng.gen(),
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

struct AutomataTimer(Timer);

fn cellular_automata(
    time: Res<Time>,
    grid: Res<Grid>,
    mut timer: ResMut<AutomataTimer>,
    mut tiles: Query<(&TileAddress, &mut TileState)>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for (address, mut state) in &mut tiles {
            let floor_neighbors = grid.count_neighbors(address, |s| *s == TileState::Floor);
            let wall_neighbors = grid.count_neighbors(address, |s| *s == TileState::Wall);
            match *state {
                TileState::Floor => {
                    if wall_neighbors > AUTOMATA_CLOSE_THRESHOLD {
                        *state = TileState::Wall;
                    }
                }
                TileState::Wall => {
                    if floor_neighbors > AUTOMATA_OPEN_THRESHOLD {
                        *state = TileState::Floor;
                    }
                }
            }
        }
    }
}

struct NoisePlugin;

impl Plugin for NoisePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NoiseFunctions::new());
    }
}

struct NoiseFunctions {
    x: OpenSimplex,
    y: OpenSimplex,
}

impl NoiseFunctions {
    fn new() -> Self {
        let (x_seed, y_seed) = random();
        NoiseFunctions {
            x: OpenSimplex::new().set_seed(x_seed),
            y: OpenSimplex::new().set_seed(y_seed),
        }
    }
    fn get_offset_2d(&self, point: Vec2) -> Vec2 {
        let point_arr = point.as_dvec2().to_array();
        Vec2::new(
            self.x.get(point_arr) as f32,
            self.y.get(point_arr) as f32,
        )
    }
}

struct GridPlugin(GridDimensions);

impl GridPlugin {
    fn new(grid_size: [u32; 2]) -> Self {
        GridPlugin(GridDimensions::new(grid_size))
    }
}

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0);
    }
}

#[derive(Copy, Clone)]
pub struct GridDimensions {
    pub size_in_tiles: [u32; 2],
    pub tile_size: f32,
    pub bottom_left: Vec2,
}

impl GridDimensions {
    fn new(size_in_tiles: [u32; 2]) -> GridDimensions {
        GridDimensions {
            size_in_tiles,
            tile_size: 1.,
            bottom_left: Vec2::ZERO,
        }
    }
    fn world_center(&self) -> Vec2 {
        self.bottom_left + Vec2::new(self.world_width() * 0.5, self.world_height() * 0.5)
    }
    fn world_width(&self) -> f32 {
        self.tile_size * self.size_in_tiles[0] as f32
    }
    fn world_height(&self) -> f32 {
        self.tile_size * self.size_in_tiles[1] as f32
    }
    fn world_pos_of(&self, tile: &TileAddress) -> Vec2 {
        self.bottom_left + Vec2::new(
            tile.0 as f32 * self.tile_size,
            tile.1 as f32 * self.tile_size,
        )
    }
}