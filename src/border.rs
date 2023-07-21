use bevy::prelude::{Component, Vec2};
use bevy_rapier2d::rapier::prelude::Aabb;
use parry2d::math::{Point, Vector};

use crate::cardinal::Cardinal;
use crate::grid::{Grid, TileAddress};
use crate::zone::GridDimensions;

#[derive(Component, Debug, Copy, Clone)]
pub struct Border {
    pos: TileAddress,
    is_vertical: bool,
}

impl Border {
    fn at(pos: TileAddress, cardinal: Cardinal) -> Self {
        match cardinal {
            Cardinal::North => Border {
                pos: TileAddress(pos.0, pos.1 + 1),
                is_vertical: false,
            },
            Cardinal::East => Border {
                pos: TileAddress(pos.0 + 1, pos.1),
                is_vertical: true,
            },
            Cardinal::South => Border {
                pos,
                is_vertical: false,
            },
            Cardinal::West => Border {
                pos,
                is_vertical: true,
            }
        }
    }

    pub fn pos(&self) -> &TileAddress {
        &self.pos
    }

    pub fn is_vertical(&self) -> bool {
        self.is_vertical
    }

    pub fn get_aabb(&self, dims: &GridDimensions, radius: f32) -> Aabb {
        let tile_size = dims.tile_size;
        let Vec2 { x, y } = dims.world_pos_of(&self.pos);
        let small_dim = tile_size * radius;
        let large_dim = tile_size * (0.5 + radius);
        let half_tile = tile_size * 0.5;
        if self.is_vertical() {
            Aabb::from_half_extents(
                Point::new(x, y + half_tile),
                Vector::new(small_dim, large_dim).into(),
            )
        } else {
            Aabb::from_half_extents(
                Point::new(x + half_tile, y),
                Vector::new(large_dim, small_dim),
            )
        }
    }
}

pub fn collect_borders<T, F, FB>(grid: &Grid<T>, test_inside: &F, receiver: &mut FB)
    where F: Fn(&T) -> bool,
          FB: FnMut(Border) -> ()
{
    for addr in grid.addresses() {
        if let Some(tile) = grid.tile_at(&addr) {
            if test_inside(tile) {
                if addr.0 == 0 {
                    // west wall of X=0 is always a border
                    receiver(Border::at(addr, Cardinal::West));
                }
                if addr.1 == 0 {
                    // south wall of Y=0 is always a border
                    receiver(Border::at(addr, Cardinal::South));
                }

                // if tile to the North is not inside, that's a border
                let to_north = TileAddress(addr.0, addr.1 + 1);
                if !grid.tile_at(&to_north).is_some_and(test_inside) {
                    receiver(Border::at(addr, Cardinal::North));
                }

                // if tile to the East is not inside, that's a border
                let to_east = TileAddress(addr.0 + 1, addr.1);
                if !grid.tile_at(&to_east).is_some_and(test_inside) {
                    receiver(Border::at(addr, Cardinal::East));
                }
            } else {
                // if the tile to the North is inside, that's a border
                let to_north = TileAddress(addr.0, addr.1 + 1);
                if grid.tile_at(&to_north).is_some_and(test_inside) {
                    receiver(Border::at(to_north, Cardinal::South));
                }

                // if the tile to the East is inside, that's a border
                let to_east = TileAddress(addr.0 + 1, addr.1);
                if grid.tile_at(&to_east).is_some_and(test_inside) {
                    receiver(Border::at(to_east, Cardinal::West));
                }
            }
        }
    }
}