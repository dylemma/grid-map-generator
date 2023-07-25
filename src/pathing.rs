use bevy::app::App;
use bevy::input::Input;
use bevy::prelude::*;
use bevy_rapier2d::control::KinematicCharacterController;
use pathfinding::directed::astar;
use crate::fill::Tiles;

use crate::grid::{Grid, TileAddress};
use crate::input::PlayerCursor;
use crate::PlayerControlled;
use crate::zone::{GridDimensions, TileState};

pub struct PathingPlugin;

#[derive(Component)]
struct DestinationGoal {
    pos: Vec2,
}

#[derive(Component)]
struct ComputedPath {
    waypoints: Vec<TileAddress>,
    next_waypoint: usize,
}

impl Plugin for PathingPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, update_movement_agents)
            .add_systems(Update, handle_player_nav)
            .add_systems(Update, compute_paths)
            .add_systems(Update, show_path_sprites)
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

fn handle_player_nav(
    player: Query<Entity, (With<PlayerControlled>, Without<DestinationGoal>)>,
    mut commands: Commands,
    cursor: Res<PlayerCursor>,
    button: Res<Input<MouseButton>>,
) {
    if button.just_pressed(MouseButton::Middle) {
        for player in player.iter() {
            println!("Set player nav request to {:?}", cursor.world_pos);
            commands.entity(player).insert(DestinationGoal {
                pos: cursor.world_pos,
            }).remove::<ComputedPath>();
        }
    }
}

fn compute_paths(
    agents: Query<(Entity, &DestinationGoal, &Transform), Without<ComputedPath>>,
    mut commands: Commands,
    grid: Res<Grid<TileState>>,
    dimensions: Res<GridDimensions>,
) {
    for (entity, goal, transform) in agents.iter() {
        let current_pos: Vec2 = transform.translation.truncate();
        let goal_pos: Vec2 = goal.pos;


        commands.entity(entity).remove::<DestinationGoal>();

        if let Some(current_tile) = dimensions.position_to_address(current_pos) {
            if let Some(goal_tile) = dimensions.position_to_address(goal_pos) {
                if let Some(path) = find_path(&grid, current_tile, goal_tile) {
                    commands.entity(entity).insert(ComputedPath {
                        waypoints: path,
                        next_waypoint: 0,
                    });
                }
            }
        }
    }
}

fn find_path(grid: &Grid<TileState>, start: TileAddress, goal: TileAddress) -> Option<Vec<TileAddress>> {
    let is_floor = |t: &TileAddress| {
        grid.tile_at(t).is_some_and(|state| state.is_floor())
    };

    let (path, _) = astar::astar(
        &start,
        |&tile | {
            let cardinal_ds = [(0, 1), (1, 0), (0, -1), (-1, 0)]; // NESW
            let cardinals = cardinal_ds.map(|dv|  (tile + dv).filter(is_floor));
            let try_diagonal = |i: usize, j: usize, dx: i32, dy: i32| {
                if cardinals[i].is_some() && cardinals[j].is_some() {
                    (tile + (dx, dy)).filter(is_floor)
                } else {
                    None
                }
            };
            let diagonals = [
                try_diagonal(0, 1, 1, 1),
                try_diagonal(1, 2, 1, -1),
                try_diagonal(2, 3, -1, -1),
                try_diagonal(3, 0, -1, 1),
            ];
            diagonals.into_iter().flatten().map(|t| (t, 1414))
                .chain(cardinals.into_iter().flatten().map(|t| (t, 1000)))
        },
        |tile| {
            let dx = tile.0.abs_diff(goal.0);
            let dy = tile.1.abs_diff(goal.1);
            ((dx as f32).hypot(dy as f32) * 1000f32) as u32
        },
        |&tile| tile == goal,
    )?;

    Some(path)
}

#[derive(Component)]
struct PathSprite;

#[derive(Component)]
struct PathSprites(Vec<Entity>);

fn show_path_sprites(
    mut paths: Query<(Entity, &ComputedPath, Option<&mut PathSprites>), Changed<ComputedPath>>,
    dims: Res<GridDimensions>,
    mut commands: Commands,
) {
    for (entity, comp_path, mut maybe_sprites) in paths.iter_mut() {
        if let Some(old_sprites) = maybe_sprites.take() {
            for &sprite_entity in &old_sprites.0 {
                commands.entity(sprite_entity).despawn();
            }
        }

        let mut new_sprites = comp_path.waypoints.iter().map(|tile| {
            let xy = dims.world_pos_of(tile) + Vec2::splat(dims.tile_size * 0.5);
            commands.spawn(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(0.25)),
                    color: Color::ORANGE,
                    ..default()
                },
                transform: Transform {
                    translation: (xy, 0.).into(),
                    rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
                    ..default()
                },
                ..default()
            }).insert(PathSprite).id()
        }).collect();

        commands.entity(entity).remove::<PathSprites>().insert(PathSprites(new_sprites));
    }
}