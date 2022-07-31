use bevy::{
    prelude::*,
    render::camera::ScalingMode,
};
use bevy::render::camera::{DepthCalculation, WindowOrigin};
use bevy::sprite::Anchor;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(bounce_colors)
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

    let red_corner = grid.bottom_left;
    let green_corner = grid.bottom_left + Vec2::new(grid.width() / 2., grid.height());
    let blue_corner = grid.bottom_left + Vec2::new(grid.width(), 0.);
    let max_dist = (grid.width().powi(2) + grid.height().powi(2)).sqrt();

    for i in 0..grid.size_in_tiles[0] {
        for j in 0..grid.size_in_tiles[1] {
            let tile = Tile::new(i, j);
            let pos = grid.tile_pos(&tile);
                //bottom_left + Vec2::new(i as f32, j as f32);
            let r = 1.0 - pos.distance(red_corner) / max_dist;
            let g = 1.0 - pos.distance(green_corner) / max_dist;
            let b = 1.0 - pos.distance(blue_corner) / max_dist;
            let color = Color::rgb(r, g, b);

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

fn bounce_colors(time: Res<Time>, mut query: Query<&mut TileColor, With<Tile>>) {
    let dt = time.delta_seconds() * 0.5;
    for mut tc in &mut query {
        let [r, g, b, a] = tc.0.as_rgba_f32();
        let r2 = (r + dt) % 1.;
        let g2 = (g + dt) % 1.;
        let b2 = (b + dt) % 1.;
        tc.0 = Color::rgba(r2, g2, b2, a);
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