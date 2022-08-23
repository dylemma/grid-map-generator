#![feature(is_some_with)]
#![feature(step_trait)]

use std::ops::DerefMut;

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
};
use bevy::render::camera::{DepthCalculation, WindowOrigin};
use bevy::sprite::Anchor;
use noise::{NoiseFn, OpenSimplex, Seedable};
use rand::prelude::*;

use crate::fill::flood_fill;
use crate::grid::*;
use crate::procgen::{generate_island_into, Reachability};

mod grid;
mod fill;
mod procgen;

const GRID_SIZE: [u32; 2] = [50, 50];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(reset_tiles_on_keypress)
        .add_system(wiggle_tiles)
        .add_system(sync_tile_sprites)

        .insert_resource(MouseLoc(Vec2::ZERO))
        .add_system(mouse_pointing)
        .add_system(mouse_picking)

        .run();
}

fn setup(
    mut commands: Commands,
) {
    let grid_dims = GridDimensions::new(GRID_SIZE);
    let noise = Noise2::new();
    let tiles = {
        let mut tiles = Grid::<TileState>::new_from_dims(&grid_dims);
        generate_island_into(&grid_dims, &noise.x, &mut tiles, TileState::from);
        tiles
    };

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

    for tile_address in tiles.addresses() {
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
            .insert(tiles[tile_address])
            .insert(TileWiggle::new())
        ;
    }

    commands.insert_resource(tiles);
    commands.insert_resource(grid_dims);
    commands.insert_resource(noise);
}

#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub enum TileState {
    #[default]
    Floor,
    #[allow(dead_code)]
    Wall,
    Water,
    Elevation(f32),
    Colored(Color),
}

impl TileState {
    fn as_color(&self) -> Color {
        match self {
            TileState::Floor => Color::WHITE,
            TileState::Wall => Color::BLACK,
            TileState::Water => Color::rgb(0.0, 0.1, 0.4),
            TileState::Elevation(e) => Color::rgb(*e, *e, *e),
            TileState::Colored(c) => *c,
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

struct MouseLoc(Vec2);

fn mouse_pointing(
    mut mouse: ResMut<MouseLoc>,
    mut move_events: EventReader<CursorMoved>,
) {
    for event in move_events.iter() {
        mouse.0 = event.position;
    }
}

fn mouse_picking(
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<MouseLoc>,
    button: Res<Input<MouseButton>>,
    grid: Res<GridDimensions>,
) {
    if button.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = q_camera.single();
        if let Some(mouse_world_pos) = mouse_to_world(camera, camera_transform, mouse.0) {
            if let Some(TileAddress(x, y)) = world_to_tile(&grid, mouse_world_pos) {
                println!("clicked at {}, {}", x, y);
            }
        }
    }
}

// https://bevy-cheatbook.github.io/cookbook/cursor2world.html
fn mouse_to_world(camera: &Camera, camera_transform: &GlobalTransform, mouse_pixel_pos: Vec2) -> Option<Vec2> {
    let window_size = camera.logical_viewport_size()?;
    let ndc = (mouse_pixel_pos / window_size) * 2.0 - Vec2::ONE;
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
    Some(world_pos.truncate())
}

fn world_to_tile(dims: &GridDimensions, world_pos: Vec2) -> Option<TileAddress> {
    let rel_pos = ((world_pos - dims.bottom_left) / dims.tile_size).floor();
    let tile_x = u32::try_from(rel_pos.x as i32).ok()?;
    let tile_y = u32::try_from(rel_pos.y as i32).ok()?;
    if tile_x < dims.size_in_tiles[0] && tile_y < dims.size_in_tiles[1] {
        Some(TileAddress(tile_x, tile_y))
    } else {
        None
    }
}

// Update the color of any sprite whose TileState has changed
fn sync_tile_sprites(
    mut tile_sprites: Query<(&mut Sprite, &TileState), Changed<TileState>>,
) {
    for (mut sprite, tile_state) in &mut tile_sprites {
        sprite.color = tile_state.as_color();
    }
}

fn reset_tiles_on_keypress(
    keyboard: Res<Input<KeyCode>>,
    mut noise: ResMut<Noise2>,
    grid: Res<GridDimensions>,
    mut tiles: ResMut<Grid<TileState>>,
    mut tile_sprites: Query<(&mut TileState, &TileAddress)>,
) {
    if keyboard.just_pressed(KeyCode::Return) {
        noise.reseed();

        // regenerate the random "island" to update the `tiles` resource
        generate_island_into(&grid, &noise.x, tiles.deref_mut(), TileState::from);

        // update the TileState of any entity that represents a spot on the grid
        for (mut state, address) in &mut tile_sprites {
            *state = tiles[address];
        }
    }
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


pub struct Noise(OpenSimplex);

impl Noise {
    // OpenSimplex seems to have a range of +/- 0.54397714
    // and we want to scale that to +/- 0.5
    const SIMPLEX_SCALAR: f64 = 0.5 / 0.5439777;

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
        (self.0.get([(x as f64) * 4.0, (y as f64) * 4.0]) * Noise::SIMPLEX_SCALAR) as f32
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
