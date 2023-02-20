use bevy::prelude::*;

use crate::{GridDimensions, TileAddress};

pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(MouseLoc(Vec2::ZERO))
            .add_system(mouse_pointing)
            .add_system(mouse_picking)
        ;
    }
}

#[derive(Resource)]
struct MouseLoc(Vec2);

fn mouse_pointing(
    mut mouse: ResMut<MouseLoc>,
    mut move_events: EventReader<CursorMoved>,
) {
    for event in move_events.iter() {
        mouse.0 = event.position;
    }
}

fn mouse_picking(
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mouse: Res<MouseLoc>,
    button: Res<Input<MouseButton>>,
    dimensions: Res<GridDimensions>,
) {
    if button.just_pressed(MouseButton::Left) {
        let (camera, camera_transform) = q_camera.single();
        if let Some(mouse_world_pos) = mouse_to_world(camera, camera_transform, mouse.0) {
            if let Some(TileAddress(x, y)) = dimensions.position_to_address(mouse_world_pos) {
                println!("clicked at {}, {}", x, y);
            }
        }
    }
}

// https://bevy-cheatbook.github.io/cookbook/cursor2world.html
fn mouse_to_world(camera: &Camera, camera_transform: &GlobalTransform, mouse_pixel_pos: Vec2) -> Option<Vec2> {
    let window_size = camera.logical_viewport_size()?;
    let ndc = (mouse_pixel_pos / window_size) * 2.0 - Vec2::ONE;
    let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
    let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
    Some(world_pos.truncate())
}