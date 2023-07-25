#![feature(step_trait)]

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
};
use bevy::sprite::Anchor;
use bevy_rapier2d::prelude::*;

use crate::border::{Border, collect_borders};
use crate::fill::flood_fill;
use crate::grid::*;
use crate::input::{GameInputPlugin, PlayerCursor};
use crate::laser::{LaserBundle, LasersPlugin};
use crate::noise::Noise;
use crate::pathing::PathingPlugin;
use crate::wiggle::{TileWiggle, TileWigglePlugin};
use crate::zone::*;

mod border;
mod cardinal;
mod fill;
mod grid;
mod input;
mod laser;
mod noise;
mod pathing;
mod procgen;
mod wiggle;
mod zone;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GameInputPlugin)
        .add_plugins(ZonePlugin(50, 50))
        // .add_plugin(TileWigglePlugin)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, reset_tiles_on_keypress)
        .add_systems(Update, sync_zone_tile_sprites)

        .add_plugins(LasersPlugin)

        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_systems(Update, spawn_balls)
        .add_systems(PostUpdate, reap_balls)

        .add_plugins(PathingPlugin)
        .add_systems(Startup, init_player)
        .add_systems(Update, handle_player_collisions)
        .run();
}

#[derive(Component)]
struct PlayerControlled;

fn init_player(
    mut commands: Commands,
) {
    commands
        .spawn(PlayerControlled)
        .insert(LaserBundle::default())
        .insert(Collider::cuboid(1., 1.))
        .insert(SpriteBundle {
            sprite: Sprite {
                color: Color::BLACK,
                custom_size: Some(Vec2::splat(2.)),
                anchor: Anchor::Center,
                ..default()
            },
            ..default()
        })
        .insert(KinematicCharacterController {
            apply_impulse_to_dynamic_bodies: true,
            custom_mass: Some(10.0),
            ..default()
        })
    ;
}

#[derive(Component)]
struct MainCamera;

fn setup_camera(
    mut commands: Commands,
    dimensions: Res<GridDimensions>,
) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            viewport_origin: Vec2::splat(0.5), //WindowOrigin::Center,
            scaling_mode: ScalingMode::AutoMin {
                min_width: dimensions.world_width(),
                min_height: dimensions.world_height(),
            },
            ..default()
        },
        transform: Transform::from_translation((dimensions.world_center(), 999.9).into()).with_scale(Vec3::new(1., 1., 1.)),
        ..default()
    }).insert(MainCamera);
}

#[derive(Component)]
struct BorderWall;

fn sync_zone_tile_sprites(
    dimensions: Res<GridDimensions>,
    zone: Res<Grid<TileState>>,
    mut sprites: Query<(&mut Sprite, &TileAddress)>,
    border_entities: Query<Entity, (With<Border>, With<BorderWall>)>,
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
                let aabb = border.get_aabb(&dimensions, 0.1);
                let center = aabb.center(); //mins;
                let size: [f32; 2] = aabb.extents().into();

                commands
                    .spawn(SpriteBundle {
                        sprite: Sprite {
                            anchor: Anchor::Center,
                            color: Color::CYAN,
                            custom_size: Some(size.into()),
                            ..default()
                        },
                        transform: Transform::from_translation((center.x, center.y, 0.).into()),
                        ..default()
                    })
                    .insert(border)
                    .insert(BorderWall)
                    .insert(RigidBody::Fixed)
                    .insert(Collider::cuboid(size[0] * 0.5, size[1] * 0.5))
                ;
            }
        );
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

#[derive(Component)]
struct Ball;

fn spawn_balls(
    mut commands: Commands,
    buttons: Res<Input<MouseButton>>,
    cursor: Res<PlayerCursor>,
) {
    if buttons.pressed(MouseButton::Right) {
        commands
            .spawn(Ball)
            .insert(RigidBody::Dynamic)
            .insert(Collider::ball(0.25))
            .insert(Restitution::coefficient(0.7))
            .insert(ColliderMassProperties::Density(0.1))
            .insert(Ccd::enabled())
            .insert(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(0.5, 0.5)),
                    anchor: Anchor::Center,
                    color: Color::CYAN,
                    ..default()
                },
                transform: Transform {
                    translation: (cursor.world_pos, 0.).into(),
                    ..default()
                },
                ..default()
            });
    }
}

fn reap_balls(
    mut commands: Commands,
    balls: Query<(Entity, &GlobalTransform), With<Ball>>,
) {
    for (entity, ball) in &balls {
        let pos = ball.translation().truncate();
        if pos.y < -100. {
            commands.entity(entity).despawn();
            println!("despawn ball {:?}", entity);
        }
    }
}

fn handle_player_collisions(
    mut character_controller_outputs: Query<&mut KinematicCharacterControllerOutput, With<PlayerControlled>>,
) {
    for mut output in character_controller_outputs.iter_mut() {
        let vel = output.desired_translation;
        for collision in output.collisions.drain(..) {
            println!("Collision w/ character vs {:?} with velocity {:?} at toi {}", &collision.entity, vel, collision.toi.toi);
        }
    }
}