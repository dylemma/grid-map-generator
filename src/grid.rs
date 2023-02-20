use std::borrow::Borrow;
use std::ops::{Index, IndexMut};

use bevy::prelude::{Component, Resource};

use crate::fill::Tiles;
use crate::GridDimensions;

#[derive(Component, Copy, Clone, Debug)]
pub struct TileAddress(pub u32, pub u32);

impl TileAddress {
    pub fn as_tuple(&self) -> (u32, u32) {
        (self.0, self.1)
    }
}

#[derive(Clone, Resource)]
pub struct Grid<T> {
    width: u32,
    height: u32,
    tiles: Vec<T>,
}

impl<T: Default + Clone> Grid<T> {
    pub fn new(width: u32, height: u32) -> Self {
        let capacity = (width as usize).checked_mul(height as usize).expect("width * height was too big for usize");
        let tiles = vec![T::default(); capacity];
        Grid {
            width,
            height,
            tiles,
        }
    }
    pub fn new_from_dims(dims: &GridDimensions) -> Self {
        let [w, h] = dims.size_in_tiles;
        Self::new(w, h)
    }
}

impl<T> Grid<T> {
    pub fn width(&self) -> u32 { self.width }

    pub fn height(&self) -> u32 { self.height }

    pub fn tile_at(&self, pos: &TileAddress) -> Option<&T> {
        if pos.0 >= self.width { None } else if pos.1 >= self.height { None } else { Some(&self.tiles[(pos.1 * self.width + pos.0) as usize]) }
    }

    pub fn tile_at_mut(&mut self, pos: &TileAddress) -> Option<&mut T> {
        if pos.0 >= self.width { None } else if pos.1 >= self.height { None } else { Some(&mut self.tiles[(pos.1 * self.width + pos.0) as usize]) }
    }

    pub fn addresses(&self) -> impl Iterator<Item=TileAddress> {
        let width = self.width();
        let height = self.height();
        (0..height).flat_map(move |y| {
            (0..width).map(move |x| {
                TileAddress(x, y)
            })
        })
    }
}

impl<T, A: Borrow<TileAddress>> Index<A> for Grid<T> {
    type Output = T;

    fn index(&self, idx: A) -> &Self::Output {
        self.tile_at(idx.borrow()).expect("index out of bounds")
    }
}

impl<T, A: Borrow<TileAddress>> IndexMut<A> for Grid<T> {
    fn index_mut(&mut self, idx: A) -> &mut Self::Output {
        self.tile_at_mut(idx.borrow()).expect("index out of bounds")
    }
}

impl<T> Tiles<u32> for Grid<T>
    where T: Sized + PartialEq
{
    type Tile = T;
    fn get_tile(&self, x: u32, y: u32) -> Option<&T> {
        self.tile_at(&TileAddress(x, y))
    }
    fn set_tile(&mut self, x: u32, y: u32, tile: T) {
        if let Some(state) = self.tile_at_mut(&TileAddress(x, y)) {
            *state = tile;
        }
    }
}
