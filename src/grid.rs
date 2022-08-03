use std::ops::{Index, IndexMut};
use bevy::prelude::Component;

pub struct Tile {
    pub address: TileAddress,
    pub state: TileState,
}

#[derive(Component, Copy, Clone)]
pub struct TileAddress(pub u32, pub u32);

#[derive(Component, Copy, Clone, PartialEq)]
pub enum TileState {
    Floor,
    Wall,
    Elevation(f32),
}

pub struct Grid {
    width: u32,
    height: u32,
    tiles: Vec<Tile>,
}
impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        let capacity = (width as usize).checked_mul(height as usize).expect("width * height was too big for usize");
        let mut tiles = Vec::with_capacity(capacity);
        for y in 0..height {
            for x in 0..width {
                tiles.push(Tile {
                    address: TileAddress(x, y),
                    state: TileState::Floor,
                })
            }
        }
        Grid {
            width,
            height,
            tiles,
        }
    }
    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
    pub fn tile_at(&self, pos: TileAddress) -> Option<&Tile> {
        if pos.0 >= self.width { None }
        else if pos.1 >= self.height { None }
        else { Some(&self.tiles[(pos.1 * self.width + pos.0) as usize]) }
    }
    pub fn tile_at_mut(&mut self, pos: TileAddress) -> Option<&mut Tile> {
        if pos.0 >= self.width { None }
        else if pos.1 >= self.height { None }
        else { Some(&mut self.tiles[(pos.1 * self.width + pos.0) as usize]) }
    }

    #[allow(dead_code)]
    pub fn count_neighbors<P: Fn(&TileState) -> bool>(&self, address: &TileAddress, predicate: P) -> u32 {
        let mut count = 0;
        for x in address.0.saturating_sub(1) .. address.0.saturating_add(2).min(self.width) {
            for y in address.1.saturating_sub(1) .. address.1.saturating_add(2).min(self.height) {
                if x != address.0 || y != address.1 {
                    let state = &self.tiles[(y * self.width + x) as usize].state;
                    if predicate(state) {
                        count += 1;
                    }
                }
            }
        }
        count
    }
}
impl Index<TileAddress> for Grid {
    type Output = Tile;

    fn index(&self, idx: TileAddress) -> &Self::Output {
        self.tile_at(idx).expect("index out of bounds")
    }
}
impl IndexMut<TileAddress> for Grid {
    fn index_mut(&mut self, idx: TileAddress) -> &mut Self::Output {
        self.tile_at_mut(idx).expect("index out of bounds")
    }
}