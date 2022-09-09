use bevy::prelude::*;
use rand::prelude::*;

use crate::{flood_fill, Grid, GridDimensions, Noise, TileAddress};
use crate::fill::Tiles;

pub fn generate_island_into<T, F>(dims: &GridDimensions, noise: &Noise, out: &mut Grid<T>, f: F)
    where F: Fn(Reachability) -> T
{
    let mut grid = Grid::<TileGenState>::new_from_dims(dims);

    let shaping_func = SummingGroup::new_random_in(dims);
        // SummingGroup::new_demo_in(dims);

    // init the grid to a simplex-noise island
    for addr in grid.addresses() {
        let reachability = pick_reachability(noise, &shaping_func,dims, &addr);
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

fn pick_reachability(noise: &Noise, shaping: &dyn ShapingFunction, dims: &GridDimensions, address: &TileAddress) -> Reachability {
    let pos = dims.normalize_from_center(dims.world_pos_of(address));
    let e = pick_elevation(&noise, pos);
    let world_pos = dims.world_pos_of(address);
    let d = shaping.compute_at(world_pos) * 0.6 + 0.2;
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

trait ShapingFunction {
    fn compute_at(&self, pos: Vec2) -> f32;
}

struct SummingGroup(Vec<Box<dyn ShapingFunction>>);

impl SummingGroup {
    fn new_random_in(dims: &GridDimensions) -> Self {
        let points: Vec<Vec2> =  (0..5).map(|_| {
            dims.bottom_left + Vec2::new(dims.world_width() * random::<f32>(), dims.world_height() * random::<f32>())
        }).collect();

        let bumps = points.iter().map(|center| {
            boxed(CircleBump {
                center: center.clone(),
                radius: dims.world_width() * (0.15 + random::<f32>() * 0.15),
            })
        });

        let bridges = (0..3).map(|_| {
            let endpoints: Vec<Vec2> = points.choose_multiple(&mut thread_rng(), 2).cloned().collect();
            boxed(BridgeBump {
                start: endpoints[0],
                end: endpoints[1],
                thickness: dims.tile_size * 3.0,
            })
        });

        SummingGroup(bumps.chain(bridges).collect())
    }
}

fn boxed<F: ShapingFunction + 'static>(f: F) -> Box<dyn ShapingFunction> {
    Box::new(f)
}

impl ShapingFunction for SummingGroup {
    fn compute_at(&self, pos: Vec2) -> f32 {
        self.0.iter().map(|f| f.compute_at(pos)).sum::<f32>().min(1.0)
    }
}

struct CircleBump {
    center: Vec2,
    radius: f32,
}
impl ShapingFunction for CircleBump {
    fn compute_at(&self, pos: Vec2) -> f32 {
        let radial_dist = self.center.distance(pos) / self.radius;
        let one_at_center = 1.0 - radial_dist.min(1.0);
        one_at_center.powf(0.333)
    }
}

struct BridgeBump {
    start: Vec2,
    end: Vec2,
    thickness: f32,
}
impl ShapingFunction for  BridgeBump {
    fn compute_at(&self, pos: Vec2) -> f32 {
        let start_to_pos = pos - self.start;
        let bridge_vec = self.end - self.start;
        let t = start_to_pos.dot(bridge_vec) / bridge_vec.length_squared();
        let projection =
            if t < 0.0 { self.start }
            else if t > 1.0 { self.end }
            else { self.start + t * bridge_vec };
        let dist_ratio = pos.distance(projection) / self.thickness;

        if dist_ratio > 1.0 { 0.0 }
        else { (1.0 - dist_ratio).powf(0.5) * 0.75 }
    }
}