use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{GridDimensions, MainCamera, TileAddress};

pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PlayerCursor::default())
            .add_systems(PreUpdate, update_player_cursor)
            .add_systems(Update, mess_with_camera)
            .add_systems(Update, mouse_picking)
        ;
    }
}

#[derive(Resource, Default, Debug)]
pub struct PlayerCursor {
    pub screen_pos: Vec2,
    pub world_pos: Vec2,
}

fn update_player_cursor(
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut move_events: EventReader<CursorMoved>,
    mut cursor: ResMut<PlayerCursor>,
) {
    for event in move_events.iter() {
        cursor.screen_pos = event.position;
    }

    let (camera, camera_transform) = camera_q.single();
    if let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor.screen_pos) {
        if !cursor.world_pos.abs_diff_eq(world_pos, 0.001) {
            cursor.world_pos = world_pos;
        }
    }
}

fn mess_with_camera(
    keys: Res<Input<KeyCode>>,
    mut camera_q: Query<&mut OrthographicProjection, (With<Camera>, With<MainCamera>)>,
    mut toggle: Local<bool>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let b = !(*toggle);
        *toggle = b;
        for mut projection in &mut camera_q {
            if b {
                projection.scale *= 2.0;
            } else {
                projection.scale *= 0.5;
            }
        }
    }
}

fn mouse_picking(
    cursor: Res<PlayerCursor>,
    button: Res<Input<MouseButton>>,
    dimensions: Res<GridDimensions>,
) {
    if button.just_pressed(MouseButton::Left) {
        if let Some(TileAddress(x, y)) = dimensions.position_to_address(cursor.world_pos) {
            println!("clicked at {}, {}", x, y);
        }
    }
}

