use bevy::{
    prelude::*,
    render::camera::ScalingMode,
};
use bevy::render::camera::{DepthCalculation, WindowOrigin};
use bevy::sprite::Anchor;
use rand::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(reset_tiles_on_keypress)
        .add_system(sync_tile_colors)
        .run();
}

const GRID_SIZE: [i32; 2] = [25, 25];

fn setup(
    mut commands: Commands,
) {
    let grid = GridDimensions::new(GRID_SIZE);

    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            window_origin: WindowOrigin::Center,
            depth_calculation: DepthCalculation::ZDifference,
            scaling_mode: ScalingMode::Auto {
                min_width: GRID_SIZE[0] as f32,
                min_height: GRID_SIZE[1] as f32,
            },
            ..default()
        },
        transform: Transform::from_translation((grid.center(), 0.).into()),
        ..default()
    });

    let mut rng = thread_rng();

    for i in 0..grid.size_in_tiles[0] {
        for j in 0..grid.size_in_tiles[1] {
            let tile = Tile::new(i, j);
            let pos = grid.tile_pos(&tile);
            let tile_on = rng.gen::<bool>();
            let color =
                if tile_on { Color::WHITE }
                else { Color::BLACK };

            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::ONE),
                        anchor: Anchor::BottomLeft,
                        ..default()
                    },
                    transform: Transform::from_translation((pos, 0.).into()),
                    ..default()
                })
                .insert(tile)
                .insert(TileColor(color));
        }
    }
}

fn sync_tile_colors(mut query: Query<(&mut Sprite, &TileColor), With<Tile>>) {
    for (mut sprite, TileColor(color)) in &mut query {
        sprite.color = *color;
    }
}

fn reset_tiles_on_keypress(keyboard: Res<Input<KeyCode>>, mut tiles: Query<&mut TileColor>) {
    if keyboard.just_pressed(KeyCode::Return) {
        let mut rng = thread_rng();
        for mut tile_color in &mut tiles {
            tile_color.0 =
                if rng.gen() { Color::WHITE }
                else { Color::BLACK };
        }
    }
}

#[derive(Component)]
pub struct Tile {
    x: i32,
    y: i32,
}
impl Tile {
    fn new(x: i32, y: i32) -> Tile {
        Tile { x, y }
    }
}

#[derive(Component)]
pub struct TileColor(pub Color);

pub struct GridDimensions {
    pub size_in_tiles: [i32; 2],
    pub tile_size: f32,
    pub bottom_left: Vec2,
}
impl GridDimensions {
    fn new(size_in_tiles: [i32; 2]) -> GridDimensions {
        GridDimensions {
            size_in_tiles,
            tile_size: 1.,
            bottom_left: Vec2::ZERO,
        }
    }
    fn center(&self) -> Vec2 {
        self.bottom_left + Vec2::new(self.width() * 0.5, self.height() * 0.5)
    }
    fn width(&self) -> f32 {
        self.tile_size * self.size_in_tiles[0] as f32
    }
    fn height(&self) -> f32 {
        self.tile_size * self.size_in_tiles[1] as f32
    }
    fn tile_pos(&self, tile: &Tile) -> Vec2 {
        self.bottom_left + Vec2::new(
            tile.x as f32 * self.tile_size,
            tile.y as f32 * self.tile_size,
        )
    }
}