use std::ops::{Index, IndexMut};
use bevy::math::Vec2;

use bevy::prelude::{Component, Resource};
use bevy::utils::default;
use parry2d::bounding_volume::Aabb;
use parry2d::math::{Isometry, Real, Vector};
use parry2d::partitioning::{IndexedData, Qbvh, QbvhUpdateWorkspace};
use parry2d::shape::{Cuboid, Shape};

use crate::border::Border;
use crate::zone::GridDimensions;

#[derive(Component, Debug, Default, PartialEq, Eq, Hash, Copy, Clone)]
pub struct ObstacleRef(usize);

impl IndexedData for ObstacleRef {
    fn default() -> Self {
        // copying IndexedData impl for usize
        ObstacleRef(u32::MAX as usize)
    }

    fn index(&self) -> usize {
        self.0
    }
}

pub struct Obstacle {
    pub shape: Box<dyn Shape>,
    pub isometry: Isometry<Real>,
}

impl Obstacle {
    pub fn border_wall(border: Border, dims: &GridDimensions) -> Self {
        let tile_size = dims.tile_size;
        let Vec2 { x, y } = dims.world_pos_of(border.pos());
        if border.is_vertical() {
            Obstacle {
                shape: Cuboid::new(Vector::new(tile_size * 0.1, tile_size * 0.6)).clone_box(),
                isometry: Isometry::translation(x, y + tile_size * 0.5),
            }
        } else {
            Obstacle {
                shape: Cuboid::new(Vector::new(tile_size * 0.6, tile_size * 0.1)).clone_box(),
                isometry: Isometry::translation(x + tile_size * 0.5, y),
            }
        }
    }

    pub fn aabb(&self) -> Aabb {
        self.shape.compute_aabb(&self.isometry)
    }
}

#[derive(Resource)]
pub struct Obstacles {
    obstacles: Vec<Obstacle>,
    qbvh: Qbvh<ObstacleRef>,
    workspace: QbvhUpdateWorkspace,
}

impl Default for Obstacles {
    fn default() -> Self {
        Obstacles {
            obstacles: default(),
            qbvh: Qbvh::new(),
            workspace: default(),
        }
    }
}

impl Index<ObstacleRef> for Obstacles {
    type Output = Obstacle;

    fn index(&self, index: ObstacleRef) -> &Self::Output {
        &self.obstacles[index.0]
    }
}

impl IndexMut<ObstacleRef> for Obstacles {
    fn index_mut(&mut self, index: ObstacleRef) -> &mut Self::Output {
        self.qbvh.pre_update_or_insert(index);
        &mut self.obstacles[index.0]
    }
}

impl Obstacles {
    fn new() -> Self {
        default()
    }

    pub fn remove_all(&mut self) {
        for index in 0..self.obstacles.len() {
            self.qbvh.remove(ObstacleRef(index));
        }
        self.obstacles.clear();
    }

    pub fn add(&mut self, obstacle: Obstacle) -> ObstacleRef {
        let index = ObstacleRef(self.obstacles.len());
        self.obstacles.push(obstacle);
        self.qbvh.pre_update_or_insert(index);
        index
    }

    pub fn refit(&mut self) -> usize {
        self.qbvh.refit(0., &mut self.workspace, |index| {
            let obstacle = &self.obstacles[index.0];
            obstacle.aabb()
        })
    }

    pub fn rebalance(&mut self) {
        self.qbvh.rebalance(0., &mut self.workspace);
    }
}
