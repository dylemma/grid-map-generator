use bevy::prelude::*;

use crate::{flood_fill, Grid, GridDimensions, Noise, TileAddress};
use crate::fill::Tiles;

pub fn generate_island_into<T, F>(dims: &GridDimensions, noise: &Noise, out: &mut Grid<T>, f: F)
    where F: Fn(Reachability) -> T
{
    let mut grid = Grid::<TileGenState>::new_from_dims(dims);

    // init the grid to a simplex-noise island
    for addr in grid.addresses() {
        let reachability = pick_reachability(noise, dims, &addr);
        grid[addr] = reachability.into();
    }

    // in case of multiple separate island areas, find the biggest one and treat it as the "primary"
    let primary_group_id = {
        let mut current_group_id = GroupId::default();
        let mut biggest_group = (current_group_id, 0u64);
        for addr in grid.addresses() {
            // when we find an "unassigned" tile, do a flood fill, assigning it and
            // all connected tiles to the current group, and keeping track of how
            // many tiles were in the new group to update biggest_group
            if grid[addr] == TileGenState::Unassigned {
                let mut grid_proxy = GridProxy { grid: &mut grid, group_size: 0 };
                flood_fill(
                    &mut grid_proxy,
                    addr.as_tuple(),
                    |a, b| { *a == *b },
                    TileGenState::ReachableGroup(current_group_id),
                );
                let current_group_size = grid_proxy.group_size;

                if current_group_size > biggest_group.1 {
                    biggest_group = (current_group_id, current_group_size);
                }

                current_group_id = current_group_id.next();
            }
        }

        biggest_group.0
    };

    for addr in out.addresses() {
        out[addr] = f(match grid[addr] {
            TileGenState::Unreachable => Reachability::Closed,
            TileGenState::ReachableGroup(group_id) => {
                if group_id == primary_group_id { Reachability::Open } else { Reachability::Closed }
            }
            TileGenState::Unassigned => Reachability::Closed,
        });
    }
}

struct GridProxy<'a> {
    grid: &'a mut Grid<TileGenState>,
    group_size: u64,
}

impl<'a> Tiles<u32> for GridProxy<'a> {
    type Tile = TileGenState;

    fn get_tile(&self, x: u32, y: u32) -> Option<&Self::Tile> {
        self.grid.tile_at(&TileAddress(x, y))
    }

    fn set_tile(&mut self, x: u32, y: u32, tile: Self::Tile) {
        if let Some(out) = self.grid.tile_at_mut(&TileAddress(x, y)) {
            *out = tile;
            self.group_size += 1;
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
enum TileGenState {
    #[default]
    Unreachable,
    Unassigned,
    ReachableGroup(GroupId),
}

impl From<Reachability> for TileGenState {
    fn from(r: Reachability) -> Self {
        match r {
            Reachability::Open => TileGenState::Unassigned,
            Reachability::Closed => TileGenState::Unreachable,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct GroupId(u8);

impl GroupId {
    fn next(&self) -> Self {
        GroupId(self.0 + 1)
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum Reachability {
    Open,
    #[default]
    Closed,
}

fn pick_reachability(noise: &Noise, dims: &GridDimensions, address: &TileAddress) -> Reachability {
    let pos = dims.normalize_from_center(dims.world_pos_of(address));
    let e = pick_elevation(&noise, pos);
    let d = square_bump(pos.x, pos.y);
    let e2 = (e + d) * 0.5;

    if e2 > 0.5 { Reachability::Open } else { Reachability::Closed }
}

// picks an "elevation" in the range (0.0, 1.0) for the given XY coordinate
// based on a simplex noise function.
fn pick_elevation(noise: &Noise, point: Vec2) -> f32 {
    let mut e = 0.0;
    // low-frequency noise as the baseline
    e += noise.get_at(point);
    // high-frequency noise for some variation
    e += 0.25 * noise.get_at(point * 2.0);
    // normalize magnitude
    e /= 1.25;
    // adjust range from [-0.5, 0.5] to [0, 1]
    e += 0.5;
    e
}

// compute a Z scalar based on X and Y positions relative to a center.
// `nx` and `ny` are assumed to be in the range (-1.0, 1.0), for an
// expected output of (0.0, 1.0). Output is 1 at the center, approaching
// 0 as `|nx|` and `|ny|` approach 1.
fn square_bump(nx: f32, ny: f32) -> f32 {
    (1. - nx.powi(2)) * (1. - ny.powi(2))
}