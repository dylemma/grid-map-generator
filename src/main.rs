#![feature(step_trait)]

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
};
use bevy::render::camera::WindowOrigin;
use bevy::sprite::Anchor;
use parry2d::math::{Point, Vector};

use crate::border::{Border, collect_borders};
use crate::fill::flood_fill;
use crate::grid::*;
use crate::input::{GameInputPlugin, mouse_to_world, MouseLoc};
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

        .insert_resource(LaserPointer::default())
        .add_system(laser_pointer_system)

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

// ---- Laser Pointer ----

#[derive(Resource, Default)]
struct LaserPointer {
    pressed_at: Option<Vec2>,
    held_at: Option<Vec2>,
    hit_at: Option<Vec2>,
}

#[derive(Component)]
struct LaserOrigin;

#[derive(Component)]
struct LaserEnd;

#[derive(Component)]
struct LaserBeam;

fn laser_pointer_system(
    mut commands: Commands,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<MouseLoc>,
    mut pointer: ResMut<LaserPointer>,
    button: Res<Input<MouseButton>>,
    laser_origin: Query<Entity, With<LaserOrigin>>,
    mut laser_end: Query<(Entity, &mut Transform), With<LaserEnd>>,
    mut laser_beam: Query<(Entity, &LaserBeam, &mut Transform), Without<LaserEnd>>,
    obstacles: Res<Obstacles>,
) {
    let prev_held_loc = pointer.held_at;

    // update `pressed_at` when the mouse becomes pressed
    if button.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = q_camera.single();
        let clicked_loc = mouse_to_world(camera, camera_transform, mouse.0);
        pointer.pressed_at = clicked_loc;
    }

    // update `held_at` when the mouse remains pressed
    if button.pressed(MouseButton::Left) {
        // update the 'held' point
        let (camera, camera_transform) = q_camera.single();
        let held_loc = mouse_to_world(camera, camera_transform, mouse.0);
        pointer.held_at = held_loc;
    } else {
        // turn off the laser if the mouse is released
        pointer.pressed_at = None;
        pointer.held_at = None;
    }

    // update entities based on laser state
    let laser_needs_update = match (prev_held_loc, pointer.held_at) {
        (None, Some(new_held)) => {
            println!("Laser ON");
            // laser just turned on; spawn sprites!
            commands.spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::Center,
                    color: Color::RED,
                    custom_size: Some(Vec2::splat(0.5)),
                    ..default()
                },
                transform: Transform::from_translation((new_held, 0.).into()),
                ..default()
            }).insert(LaserOrigin);

            commands.spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::Center,
                    color: Color::ORANGE_RED,
                    custom_size: Some(Vec2::splat(0.5)),
                    ..default()
                },
                transform: Transform::from_translation((new_held, 0.).into()),
                ..default()
            }).insert(LaserEnd);

            commands.spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::CenterLeft,
                    color: Color::YELLOW,
                    custom_size: Some(Vec2::new(1.0, 0.25)),
                    ..default()
                },
                ..default()
            }).insert(LaserBeam);

            true
        },
        (Some(_), None) => {
            // laser just turned off; despawn sprites
            println!("Laser OFF");
            for entity in &laser_origin {
                commands.entity(entity).despawn();
            }
            for (entity, _) in &laser_end {
                commands.entity(entity).despawn();
            }
            for (entity, _, _) in &laser_beam {
                commands.entity(entity).despawn();
            }

            false
        },
        (None, None) => {
            // laser stayed off; do nothing
            false
        },
        (Some(prev_pos), Some(new_pos)) => {
            // laser remained on; see if it moved
            if !new_pos.abs_diff_eq(prev_pos, 0.001) {
                println!("laser moved!");
                // for (_, mut transform) in &mut laser_end {
                //     *transform = Transform::from_translation((new_pos, 0.).into())
                // }
                true
            } else {
                false
            }
        },
    };

    if laser_needs_update {
        if let Some(origin_pos) = pointer.pressed_at {
            if let Some(target_pos) = pointer.held_at {
                let direction = (target_pos - origin_pos).normalize();
                let translation = (origin_pos, 0.).into();
                let rotation = Quat::from_rotation_arc_2d(Vec2::new(1., 0.), direction);
                let parry_ray = parry2d::query::Ray::new(
                    Point::new(origin_pos.x, origin_pos.y),
                    Vector::new(direction.x, direction.y),
                );
                let toi = obstacles.get_toi(&parry_ray, 100.0);
                let laser_scale = toi.unwrap_or(100.0f32);
                for (_, _, mut t) in &mut laser_beam {
                    t.translation = translation;
                    t.rotation = rotation;
                    t.scale = (laser_scale, 1., 1.).into()
                }

                pointer.hit_at = toi.map(|t| origin_pos + (direction * t));

                println!("Laser Impact at {:?}", toi);
            }
        }
    }

    if let Some(origin_pos) = pointer.pressed_at {
        let hit_pos = pointer.hit_at.unwrap_or(origin_pos);
        for (_, mut transform) in &mut laser_end {
            transform.translation = (hit_pos, 0.).into();
        }

    }
}
