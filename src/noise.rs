use bevy::prelude::Vec2;
use noise_lib::{NoiseFn, OpenSimplex, Seedable};
use rand::prelude::*;

pub struct Noise(OpenSimplex);

impl Noise {
    // OpenSimplex seems to have a range of +/- 0.54397714
    // and we want to scale that to +/- 0.5
    const SIMPLEX_SCALAR: f64 = 0.5 / 0.5439777;

    pub fn new() -> Self {
        let seed = random();
        Noise(OpenSimplex::new().set_seed(seed))
    }
    pub fn reseed(&mut self) {
        let seed = random();
        self.0 = self.0.set_seed(seed);
    }
    pub fn get(&self, x: f32, y: f32) -> f32 {
        (self.0.get([(x as f64) * 4.0, (y as f64) * 4.0]) * Noise::SIMPLEX_SCALAR) as f32
    }
    pub fn get_at(&self, point: Vec2) -> f32 {
        self.get(point.x, point.y)
    }
}