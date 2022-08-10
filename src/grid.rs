use std::borrow::Borrow;
use std::ops::{Index, IndexMut};
use std::slice::Iter;
use bevy::prelude::{Color, Component};
use crate::fill::Tiles;

#[derive(Component, Copy, Clone, Debug)]
pub struct TileAddress(pub u32, pub u32);

#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub enum TileState {
    #[default]
    Floor,
    #[allow(dead_code)]
    Wall,
    Water,
    Elevation(f32),
    Colored(Color),
}

#[derive(Clone)]
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

    pub fn neighbors_of(&self, address: TileAddress, nt: NeighborhoodType) -> NeighborsItr {
        let hood = Neighborhood {
            max_exclusive: TileAddress(self.width, self.height),
            center: address,
        };
        let offsets = match nt {
            NeighborhoodType::FourWay => FOUR_WAY_NEIGHBOR_OFFSETS.iter(),
            NeighborhoodType::EightWay => EIGHT_WAY_NEIGHBOR_OFFSETS.iter(),
        };
        NeighborsItr { hood, offsets }
    }

    #[allow(dead_code)]
    pub fn count_neighbors<P: Fn(&T) -> bool>(&self, address: TileAddress, predicate: P) -> u32 {
        self
            .neighbors_of(address, NeighborhoodType::EightWay)
            .filter(|n| predicate(&self[n]))
            .count() as u32
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

struct Neighborhood {
    center: TileAddress,
    max_exclusive: TileAddress,
}

impl Neighborhood {
    fn check_x(&self, x: u32) -> Option<u32> {
        if x < self.max_exclusive.0 { Some(x) } else { None }
    }
    fn check_y(&self, y: u32) -> Option<u32> {
        if y < self.max_exclusive.1 { Some(y) } else { None }
    }


    fn rel(&self, dx: i32, dy: i32) -> Option<TileAddress> {
        let TileAddress(cx, cy) = self.center;

        let dxa = dx.unsigned_abs();
        let x = self.check_x((
            if dx < 0 { cx.checked_sub(dxa) } else { cx.checked_add(dxa) }
        )?)?;

        let dya = dy.unsigned_abs();
        let y = self.check_y((
            if dy < 0 { cy.checked_sub(dya) } else { cy.checked_add(dya) }
        )?)?;

        Some(TileAddress(x, y))
    }
}

pub enum NeighborhoodType {
    FourWay,
    EightWay,
}

pub struct NeighborsItr {
    hood: Neighborhood,
    offsets: Iter<'static, (i32, i32)>,
}

impl Iterator for NeighborsItr {
    type Item = TileAddress;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((dx, dy)) = self.offsets.next() {
            self.hood.rel(*dx, *dy).or_else(|| self.next())
        } else {
            None
        }
    }
}

const FOUR_WAY_NEIGHBOR_OFFSETS: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
const EIGHT_WAY_NEIGHBOR_OFFSETS: [(i32, i32); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];
