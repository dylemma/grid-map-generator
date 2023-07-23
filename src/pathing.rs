use bevy::app::App;
use bevy::input::Input;
use bevy::prelude::{Color, Commands, MouseButton, Plugin, Query, Res, Sprite, Startup, Transform, TransformBundle, Update, Vec2, With};
use bevy::sprite::{Anchor, SpriteBundle};
use bevy::utils::default;
use bevy_rapier2d::control::KinematicCharacterController;
use bevy_rapier2d::prelude::{Collider, RigidBody, Vect};
use crate::grid::TileAddress;
use crate::input::PlayerCursor;
use crate::PlayerControlled;

struct DestinationGoal {
    pos: Vec2,
}

struct ComputedPath {
    waypoints: Vec<TileAddress>,
    next_waypoint: usize,
}

pub struct PathingPlugin;

impl Plugin for PathingPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, update_movement_agents)
        ;
    }
}

fn update_movement_agents(
    mut agents: Query<(&Transform, &mut KinematicCharacterController), With<PlayerControlled>>,
    cursor: Res<PlayerCursor>,
    button: Res<Input<MouseButton>>,
) {
    if button.pressed(MouseButton::Left) {
        let target = cursor.world_pos;
        for (transform, mut agent) in agents.iter_mut() {
            let dir = (target - transform.translation.truncate()).clamp_length(0., 0.25);
            agent.translation = Some(dir);
        }
    }
}