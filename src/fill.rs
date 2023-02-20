use std::fmt::Debug;
use std::iter::{Iterator, Step};
use std::ops::{Not, RangeInclusive};

pub trait Tiles<I> {
    type Tile: Sized + PartialEq;

    fn get_tile(&self, x: I, y: I) -> Option<&Self::Tile>;
    fn set_tile(&mut self, x: I, y: I, tile: Self::Tile);
}

pub fn flood_fill<G, I, P, T>(
    tiles: &mut G,
    start: (I, I),
    color_equivalence: P,
    color: T,
)
    where G: Tiles<I, Tile=T>,
          T: Clone + Debug,
          I: Step + PartialOrd + Copy + Debug,
          P: Fn(&T, &T) -> bool,
{
    /* This is an implementation of the "combined-sacn-and-fill" algorithm described on Wikipedia:
     *  - Get start color from the starting tile
     *  - Expand left and right from the starting point to get the initial row
     *  - Add scan seeds for the Spans above and below the initial row
     *  - For each scan seed:
     *    - Find consecutive Spans in the next row up/down that are adjacent to the parent Span
     *    - If any of those Spans overhangs the parent span, recurse back to the parent row where it overhung
     *    - Also recurse (by pushing a Seed to the stack) into the next row in the current direction
     */

    let start_color: T = match tiles.get_tile(start.0, start.1) {
        Some(t) => t.clone(),
        None => return,
    };

    let mut seed_stack: Vec<(Span<I>, I, Dir)> = Vec::new();

    if let Some(start_range) = expand_range(start.0, &GridRowFloodTest {
        grid: tiles,
        tile_test: |c: &T| color_equivalence(c, &start_color),
        y: start.1,
    }) {
        // fill the initial row
        for x in start_range.into_iter() {
            tiles.set_tile(x, start.1, color.clone());
        }

        // seed the next row up
        if let Some(up_y) = Dir::Up.step(start.1) {
            seed_stack.push((start_range, up_y, Dir::Up));
        }

        // seed the next row down
        if let Some(down_y) = Dir::Down.step(start.1) {
            seed_stack.push((start_range, down_y, Dir::Down));
        }
    }

    // Stack-based recursion by pushing and popping to the seed_stack.
    // `parent_range` is a x=min..=max span representing a series of consecutive filled pixels from the previous row.
    // `y` is the current row coordinate
    // `dir` is the up/down direction that was taken to get from the parent row to the current `y`
    while let Some((parent_range, y, dir)) = seed_stack.pop() {
        let Span(parent_start, parent_end) = parent_range;

        // precalculate the "overhang" thresholds for when a parent span reaches past the parent range
        let parent_start_minus_2 = Step::backward_checked(parent_start, 2);
        let parent_end_plus_2 = Step::forward_checked(parent_end, 2);

        // scan over the "child" row for Spans of consecutive "inside" tiles
        let mut child_scan = ChildScan::start(parent_range);
        while let Some(child_range) = child_scan.next(&GridRowFloodTest {
            grid: tiles,
            y,
            tile_test: |c: &T| color_equivalence(c, &start_color),
        }) {

            // fill the tiles in the child span
            for x in child_range.into_iter() {
                tiles.set_tile(x, y, color.clone());
            }

            // add a new seed using the child range as a parent, in the same y direction
            if let Some(next_y) = dir.step(y) {
                seed_stack.push((child_range, next_y, dir));
            }

            // if the child range overhung the parent range, we've passed some obstacle
            // on the previous row, and need to jump back to that row to continue
            if let Some(prev_y) = (!dir).step(y) {
                let Span(child_start, child_end) = child_range;

                if let Some(ps2) = parent_start_minus_2 {
                    if child_start <= ps2 {
                        seed_stack.push((Span(child_start, ps2), prev_y, !dir));
                    }
                }
                if let Some(pe2) = parent_end_plus_2 {
                    if child_end >= pe2 {
                        seed_stack.push((Span(pe2, child_end), prev_y, !dir));
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Dir {
    Up,
    Down,
}

impl Dir {
    fn step<I: Step>(&self, i: I) -> Option<I> {
        match *self {
            Dir::Up => Step::forward_checked(i, 1),
            Dir::Down => Step::backward_checked(i, 1),
        }
    }
}

impl Not for Dir {
    type Output = Dir;
    fn not(self) -> Self::Output {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }
}

#[derive(Copy, Clone)]
struct Span<I: Copy>(I, I);

impl<I: Copy + Step> IntoIterator for Span<I> {
    type Item = I;
    type IntoIter = RangeInclusive<I>;

    fn into_iter(self) -> Self::IntoIter {
        self.0..=self.1
    }
}

trait FloodTest<I> {
    fn inside(&self, x: I) -> bool;
}

struct GridRowFloodTest<'g, G, P, I> {
    grid: &'g G,
    tile_test: P,
    y: I,
}

impl<'g, G, P, I, T> FloodTest<I> for GridRowFloodTest<'g, G, P, I>
    where G: Tiles<I, Tile=T>,
          P: Fn(&T) -> bool,
          I: Copy,
{
    fn inside(&self, x: I) -> bool {
        self.grid.get_tile(x, self.y).is_some_and(|t| (self.tile_test)(t))
    }
}

fn expand_range<F, I>(x: I, test: &F) -> Option<Span<I>>
    where F: FloodTest<I>,
          I: Copy + Step,
{
    if test.inside(x) {
        let x_min = DescendFrom(Some(x))
            .take_while(|x| test.inside(*x))
            .last()
            .unwrap_or_else(|| x);
        let x_max = AscendFrom(Some(x))
            .take_while(|x| test.inside(*x))
            .last()
            .unwrap_or_else(|| x);
        Some(Span(x_min, x_max))
    } else {
        None
    }
}

struct ChildScan<I: Copy> {
    parent_range: Span<I>,
    current_x: Option<I>,
}

impl<I: Copy + Step> ChildScan<I> {
    fn start(parent_range: Span<I>) -> Self {
        ChildScan {
            current_x: Some(parent_range.0),
            parent_range,
        }
    }
    fn next<F: FloodTest<I>>(&mut self, test: &F) -> Option<Span<I>> {
        let current_x = self.current_x?;
        match next_child_range_right(self.parent_range.1, current_x, test) {
            Some(next_range) => {
                let next_x = Step::forward_checked(next_range.1, 2);
                self.current_x = next_x;
                Some(next_range)
            }
            None => {
                self.current_x = None;
                None
            }
        }
    }
}

fn next_child_range_right<F, I>(parent_max_x: I, current_x: I, test: &F) -> Option<Span<I>>
    where F: FloodTest<I>,
          I: Copy + Step
{
    // Find the leftmost X in [start_x, parent_max_x] that passes the test.
    // This represents a tile adjacent to an already-filled tile in the parent row.
    let x_min = (current_x..=parent_max_x).find(|x| test.inside(*x))?;
    expand_range(x_min, test)
}

struct DescendFrom<I>(Option<I>);

impl<I: Copy + Step> Iterator for DescendFrom<I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        let prev = Step::backward_checked(self.0?, 1);
        self.0 = prev.clone();
        prev
    }
}

struct AscendFrom<I>(Option<I>);

impl<I: Copy + Step> Iterator for AscendFrom<I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        let next = Step::forward_checked(self.0?, 1);
        self.0 = next.clone();
        next
    }
}