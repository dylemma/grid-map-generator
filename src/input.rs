use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{GridDimensions, MainCamera, TileAddress};

pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(MouseLoc(Vec2::ZERO))
            .insert_resource(MouseWorldPos(None))
            .add_systems(Update, mouse_pointing)
            .add_systems(Update, mouse_picking)
            .add_systems(PreUpdate, world_pos_tracking)
        ;
    }
}

#[derive(Resource)]
pub struct MouseLoc(pub Vec2);

fn mouse_pointing(
    mut mouse: ResMut<MouseLoc>,
    mut move_events: EventReader<CursorMoved>,
) {
    for event in move_events.iter() {
        mouse.0 = event.position;
    }
}

fn mouse_picking(
    mouse_world_pos: Res<MouseWorldPos>,
    button: Res<Input<MouseButton>>,
    dimensions: Res<GridDimensions>,
) {
    if button.just_pressed(MouseButton::Left) {
        if let Some(mwp) = &mouse_world_pos.0 {
            if let Some(TileAddress(x, y)) = dimensions.position_to_address(*mwp) {
                println!("clicked at {}, {}", x, y);
            }
        }
    }
}

#[derive(Resource)]
pub struct MouseWorldPos(pub Option<Vec2>);

fn world_pos_tracking(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut mwp: ResMut<MouseWorldPos>,
) {
    let (camera, camera_transform) = camera_q.single();
    let window = windows.single();

    mwp.0 = window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate());
}
