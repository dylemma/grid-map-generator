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
use crate::raycast_world::{Obstacle, ObstacleRef, Obstacles};
use crate::wiggle::{TileWiggle, TileWigglePlugin};
use crate::zone::*;

mod border;
mod cardinal;
mod fill;
mod grid;
mod input;
mod noise;
mod procgen;
mod raycast_world;
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
    mut obstacles: ResMut<Obstacles>,
    mut sprites: Query<(&mut Sprite, &TileAddress)>,
    border_entities: Query<Entity, (With<Border>, With<ObstacleRef>)>,
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
        // TODO: once I add more kinds of obstacles that aren't border walls,
        //       this struct is going to need an efficient way to remove individual items.
        obstacles.remove_all();

        collect_borders(
            &zone,
            &|tile: &TileState| *tile == TileState::Floor,
            &mut |border: Border| {
                let obs = Obstacle::border_wall(border, &dimensions);
                let aabb = obs.aabb();
                let obs_ref = obstacles.add(obs);
                let corner = aabb.mins;
                let size: [f32; 2] = aabb.extents().into();

                commands
                    .spawn(SpriteBundle {
                        sprite: Sprite {
                            anchor: Anchor::BottomLeft,
                            color: Color::CYAN,
                            custom_size: Some(size.into()),
                            ..default()
                        },
                        transform: Transform::from_translation((corner.x, corner.y, 0.).into()),
                        ..default()
                    })
                    .insert(border)
                    .insert(obs_ref)
                ;
            }
        );
        obstacles.refit();
        obstacles.rebalance();
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
