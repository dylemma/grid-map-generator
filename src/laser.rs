use bevy::input::Input;
use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use crate::input::PlayerCursor;
use crate::PlayerControlled;
use crate::raycast_world::Obstacles;

pub struct LasersPlugin;

impl Plugin for LasersPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PreUpdate, player_laser_input)
            .add_systems(Update, solve_laser_impacts)
            .add_systems(Update, sync_laser_sprites.after(solve_laser_impacts))
        ;
    }
}

#[derive(Bundle, Default)]
pub struct LaserBundle {
    laser: Laser,
    laser_sprites: LaserSprites,
}

#[derive(Component)]
pub struct Laser {
    origin: Option<Vec2>,
    direction: Option<Vec2>,
    impact_distance: Option<f32>,
    max_length: f32,
}

impl Default for Laser {
    fn default() -> Self {
        Laser {
            origin: None,
            direction: None,
            impact_distance: None,
            max_length: 100.,
        }
    }
}

#[derive(Component, Default)]
pub struct LaserSprites {
    beam: Option<Entity>,
    impact: Option<Entity>,
}

fn player_laser_input(
    mut lasers: Query<&mut Laser, With<PlayerControlled>>,
    cursor: Res<PlayerCursor>,
    button: Res<Input<MouseButton>>,
) {
    for mut laser in lasers.iter_mut() {
        if button.just_pressed(MouseButton::Left) {
            laser.origin = Some(cursor.world_pos);
        }
        if let Some(origin) = laser.origin {
            if button.pressed(MouseButton::Left) {
                if let Some(direction) = (cursor.world_pos - origin).try_normalize() {
                    laser.direction = Some(direction);
                }
            } else {
                laser.origin = None;
                laser.direction = None;
                laser.impact_distance = None;
            }
        }
    }
}

fn solve_laser_impacts(
    mut lasers: Query<&mut Laser>,
    obstacles: Res<Obstacles>,
) {
    for mut laser in lasers.iter_mut() {
        laser.impact_distance = find_laser_impact(&laser, &obstacles);
    }
}

fn find_laser_impact(laser: &Laser, obstacles: &Obstacles) -> Option<f32> {
    let origin = laser.origin?;
    let direction = laser.direction?;
    obstacles.find_ray_impact(origin, direction, laser.max_length)
}

fn sync_laser_sprites(
    mut commands: Commands,
    mut lasers: Query<(Entity, &mut Laser, &mut LaserSprites)>,
    mut beams: Query<(Entity, &LaserBeam, &mut Transform)>,
    mut impacts: Query<(Entity, &LaserImpact, &mut Transform), Without<LaserBeam>>,
) {
    for (laser_entity, laser, mut laser_sprites) in &mut lasers {
        // add a LaserBeam sprite if there isn't one and the laser seems to be "on"
        if laser_sprites.beam.is_none() {
            if let Some(origin) = laser.origin {
                if let Some(direction) = laser.direction {
                    let mut beam_transform = default();
                    update_beam_transform(
                        &mut beam_transform,
                        origin,
                        direction,
                        laser.impact_distance.unwrap_or(laser.max_length),
                    );

                    let beam_entity_id = commands.spawn((
                        LaserBeam {
                            laser: laser_entity,
                        },
                        SpriteBundle {
                            sprite: Sprite {
                                anchor: Anchor::CenterLeft,
                                color: Color::ORANGE,
                                custom_size: Some(Vec2::new(1.0, 0.1)),
                                ..default()
                            },
                            transform: beam_transform,
                            ..default()
                        })).id();
                    laser_sprites.beam = Some(beam_entity_id);
                }
            }
        }

        // add a LaserImpact sprite if there isn't one and the laser thinks there's an impact
        if laser_sprites.impact.is_none() {
            let impact_pos = laser.origin.and_then(|origin| {
                laser.direction.and_then(|direction| {
                    laser.impact_distance.map(|dist| {
                        origin + (direction * dist)
                    })
                })
            });

            if let Some(pos) = impact_pos {
                let impact_entity_id = commands.spawn((
                    LaserImpact {
                        laser: laser_entity,
                    },
                    SpriteBundle {
                        sprite: Sprite {
                            anchor: Anchor::Center,
                            color: Color::RED,
                            custom_size: Some(Vec2::splat(0.5)),
                            ..default()
                        },
                        transform: Transform {
                            translation: (pos, 0.).into(),
                            ..default()
                        },
                        ..default()
                    }
                )).id();
                laser_sprites.impact = Some(impact_entity_id);
            }
        }
    }

    for (beam_entity, beam, mut transform) in &mut beams {
        match lasers.get_mut(beam.laser).ok() {
            None => {
                commands.entity(beam_entity).despawn();
            }
            Some((_, laser, mut laser_sprites)) => {
                let did_update = laser.origin.and_then(|origin| {
                    laser.direction.map(|direction| {
                        update_beam_transform(
                            &mut transform,
                            origin,
                            direction,
                            laser.impact_distance.unwrap_or(laser.max_length)
                        );
                    })
                }).is_some();
                if !did_update {
                    commands.entity(beam_entity).despawn();
                    laser_sprites.beam = None;
                }
            }
        }
    }

    for (impact_entity, impact, mut transform) in &mut impacts {
        match lasers.get_mut(impact.laser).ok() {
            None => {
                commands.entity(impact_entity).despawn();
            }
            Some((_, laser, mut laser_sprites)) => {
                let did_update = laser.origin.and_then(|origin| {
                    laser.direction.and_then(|direction| {
                        laser.impact_distance.map(|dist| {
                            let hit_pos = origin + (direction * dist);
                            transform.translation = (hit_pos, 0.).into();
                        })
                    })
                }).is_some();
                if !did_update {
                    commands.entity(impact_entity).despawn();
                    laser_sprites.impact = None;
                }
            }
        }
    }
}

fn update_beam_transform(transform: &mut Transform, origin: Vec2, direction: Vec2, length: f32) {
    transform.translation = (origin, 0.).into();
    transform.rotation = Quat::from_rotation_arc_2d(Vec2::new(1., 0.), direction);
    transform.scale = Vec3::new(length, 1., 1.);
}

#[derive(Component)]
pub struct LaserBeam {
    laser: Entity,
}

#[derive(Component)]
pub struct LaserImpact {
    laser: Entity,
}
