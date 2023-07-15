#![feature(step_trait)]

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
};
use bevy::render::camera::WindowOrigin;
use bevy::sprite::Anchor;
use crate::border::{Border, collect_borders};

use crate::fill::flood_fill;
use crate::grid::*;
use crate::input::GameInputPlugin;
use crate::noise::Noise;
use crate::wiggle::{TileWiggle, TileWigglePlugin};
use crate::zone::*;

mod border;
mod cardinal;
mod fill;
mod grid;
mod input;
mod noise;
mod procgen;
mod wiggle;
mod zone;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GameInputPlugin)
        .add_plugin(ZonePlugin(50, 50))
        // .add_plugin(TileWigglePlugin)
        .add_startup_system(setup_camera)
        .add_system(reset_tiles_on_keypress)
        .add_system(sync_zone_tile_sprites)
        .run();
}

fn setup_camera(
    mut commands: Commands,
    dimensions: Res<GridDimensions>,
) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            window_origin: WindowOrigin::Center,
            scaling_mode: ScalingMode::Auto {
                min_width: dimensions.world_width(),
                min_height: dimensions.world_height(),
            },
            ..default()
        },
        transform: Transform::from_translation((dimensions.world_center(), 0.).into()),
        ..default()
    });
}

fn sync_zone_tile_sprites(
    dimensions: Res<GridDimensions>,
    zone: Res<Grid<TileState>>,
    mut sprites: Query<(&mut Sprite, &TileAddress)>,
    border_entities: Query<Entity, With<Border>>,
    mut commands: Commands,
) {
    if zone.is_added() {
        for tile_address in zone.addresses() {
            let pos = dimensions.world_pos_of(&tile_address);
            let tile_state = zone[tile_address];

            commands
                .spawn(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::ONE),
                        anchor: Anchor::BottomLeft,
                        color: tile_state.as_color(),
                        ..default()
                    },
                    transform: Transform::from_translation((pos, 0.).into()),
                    ..default()
                })
                .insert(tile_address)
                .insert(zone[tile_address])
                .insert(TileWiggle::new())
            ;
        }
    } else if zone.is_changed() {
        for (mut sprite, tile_address) in &mut sprites {
            sprite.color = zone[tile_address].as_color();
        }
    }

    if zone.is_added() || zone.is_changed() {
        for entity in &border_entities {
            commands.entity(entity).despawn();
        }
        collect_borders(
            &zone,
            &|tile: &TileState| *tile == TileState::Floor,
            &mut |border: Border| {
                // println!("Got border @ {:?}", border);
                let tile_corner = dimensions.world_pos_of(border.pos());
                let border_radius = dimensions.tile_size * 0.1;
                let border_drop = Vec2::splat(-border_radius);
                let border_radius_x2 = dimensions.tile_size * 0.2;
                let (border_corner, size) = if border.is_vertical() {
                    let border_corner = tile_corner + border_drop;
                    let size = Vec2::new(border_radius_x2, dimensions.tile_size + border_radius_x2);
                    (border_corner, size)
                } else {
                    let border_corner = tile_corner + border_drop/* + Vec2::new(0., dimensions.tile_size)*/;
                    let size = Vec2::new(dimensions.tile_size + border_radius_x2, border_radius_x2);
                    (border_corner, size)
                };
                commands
                    .spawn(SpriteBundle {
                        sprite: Sprite {
                            anchor: Anchor::BottomLeft,
                            color: Color::CYAN,
                            custom_size: Some(size),
                            ..default()
                        },
                        transform: Transform::from_translation((border_corner, 0.).into()),
                        ..default()
                    })
                    .insert(border)
                ;
            }
        )
    }
}

fn reset_tiles_on_keypress(
    keyboard: Res<Input<KeyCode>>,
    mut zone_commands: EventWriter<ZoneCommand>,
) {
    if keyboard.just_pressed(KeyCode::Return) {
        zone_commands.send(ZoneCommand::Regenerate);
    }
}
