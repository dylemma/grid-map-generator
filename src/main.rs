mod grid;

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(reset_tiles_on_keypress)
        .add_system(wiggle_tiles)
        .add_system(sync_tile_sprites)
        .run();
}

fn setup(
    mut commands: Commands,
) {
    let grid = Grid::new(GRID_SIZE[0], GRID_SIZE[1]);
    let grid_dims = GridDimensions::new(GRID_SIZE);
    let noise = Noise2::new();

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

    for i in 0..grid.width() {
        for j in 0..grid.height() {
            let tile_address = TileAddress(i, j);
            let pos = grid_dims.world_pos_of(&tile_address);

            commands
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
                .insert(compute_tile_state(&noise, &grid_dims, &tile_address))
                .insert(TileWiggle::new())
                ;
        }
    }

    commands.insert_resource(grid);
    commands.insert_resource(grid_dims);
    commands.insert_resource(noise);
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
            TileState::Elevation(e) => Color::rgb(*e, *e, *e),
        };
        sprite.color = color;
    }
}

fn reset_tiles_on_keypress(keyboard: Res<Input<KeyCode>>, mut noise: ResMut<Noise2>, grid: Res<GridDimensions>, mut tiles_states: Query<(&mut TileState, &TileAddress)>) {
    if keyboard.just_pressed(KeyCode::Return) {
        noise.reseed();
        for (mut state, address) in &mut tiles_states {
            *state = compute_tile_state(&noise, &grid, &address);
        }
    }
}

fn compute_tile_state(noise: &Noise2, grid: &GridDimensions, address: &TileAddress) -> TileState {
    let pos = grid.normalize_from_center(grid.world_pos_of(address));
    // let pos = grid.world_pos_of(address);
    let e = pick_elevation(&noise.x, pos);
    let d = 1.0 - Reshaping::square_bump(pos.x, pos.y); //grid.calc_square_bump(pos);
    let e2 = (e + d) * 0.5;

    if e2 > 0.5 { TileState::Elevation(e2) }
    else { TileState::Wall }
    // TileState::Elevation(e2)
}

const WIGGLE_MAGNITUDE: f32 = 0.5;
fn wiggle_tiles(grid: Res<GridDimensions>, time: Res<Time>, noise: Res<Noise2>, mut tiles: Query<(&mut TileWiggle, &mut Transform, &TileAddress)>) {
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
    fn new() -> Self {
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

// OpenSimplex seems to have a range of +/- 0.54397714
// and we want to scale that to +/- 0.5
const SIMPLEX_SCALAR: f64 = 0.5 / 0.5439777;

struct Noise(OpenSimplex);

impl Noise {
    fn new() -> Self {
        let seed = random();
        Noise(OpenSimplex::new().set_seed(seed))
    }
    fn reseed(&mut self) {
        let seed = random();
        self.0 = self.0.set_seed(seed);
    }
    fn get(&self, xy: [f32; 2]) -> f32 {
        let [x, y] = xy;
        (self.0.get([(x as f64) * 4.0, (y as f64) * 4.0]) * SIMPLEX_SCALAR) as f32
    }
    fn get_at(&self, point: Vec2) -> f32 {
        self.get(point.to_array())
    }
}

struct Noise2 {
    pub x: Noise,
    pub y: Noise,
}

impl Noise2 {
    fn new() -> Self {
        Noise2 {
            x: Noise::new(),
            y: Noise::new(),
        }
    }
    fn reseed(&mut self) {
        self.x.reseed();
        self.y.reseed();
    }
    #[allow(dead_code)]
    fn get(&self, xy: [f32; 2]) -> Vec2 {
        Vec2::new(
            self.x.get(xy),
            self.y.get(xy),
        )
    }
    fn get_at(&self, point: Vec2) -> Vec2 {
        Vec2::new(
            self.x.get_at(point),
            self.y.get_at(point),
        )
    }
}

fn pick_elevation(noise: &Noise, point: Vec2) -> f32 {
    let mut e = 0.0;
    // low-frequency noise as the baseline
    e += noise.get_at(point);
    // high-frequency noise for some variation
    e += 0.25 * noise.get_at(point * 2.0);
    // normalize magnitude
    e /= 1.25;
    // adjust range from [-0.5, 0.5] to [0, 1]
    e += 0.5;
    e
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

    // return a new Vec2 which represents the given `point`'s position relative to
    // the `world_center`, scaled relative to size of the grid, such that for a
    // `point` inside the grid, the magnitude of the x and y components of the returned
    // vector will be at most 1.
    fn normalize_from_center(&self, point: Vec2) -> Vec2 {
        let Vec2 { x, y } = point;
        Vec2 {
            x: 2.0 * (x - self.bottom_left.x) / self.world_width() - 1.0,
            y: 2.0 * (y - self.bottom_left.y) / self.world_height() - 1.0,
        }
    }
}

struct Reshaping;
impl Reshaping {
    fn square_bump(nx: f32, ny: f32) -> f32 {
        1. - (1. - nx.powi(2)) * (1. - ny.powi(2))
    }
}