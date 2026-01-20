use crate::state::global;
use chrono::{Datelike, Local, Timelike};
use noise::{NoiseFn, Perlin};
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Normal};

// Calculation

pub fn math_abs(n: f32) -> f32 {
    n.abs()
}

pub fn math_ceil(n: f32) -> f32 {
    n.ceil()
}

pub fn math_constrain(n: f32, low: f32, high: f32) -> f32 {
    n.clamp(low, high)
}

pub fn math_dist(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

pub fn math_exp(n: f32) -> f32 {
    n.exp()
}

pub fn math_floor(n: f32) -> f32 {
    n.floor()
}

pub fn math_fract(n: f32) -> f32 {
    n.fract()
}

pub fn math_lerp(start: f32, stop: f32, amt: f32) -> f32 {
    start + (stop - start) * amt
}

pub fn math_log(n: f32) -> f32 {
    n.ln()
}

pub fn math_mag(x: f32, y: f32) -> f32 {
    (x.powi(2) + y.powi(2)).sqrt()
}

pub fn math_map(value: f32, start1: f32, stop1: f32, start2: f32, stop2: f32) -> f32 {
    start2 + (stop2 - start2) * ((value - start1) / (stop1 - start1))
}

pub fn math_max(a: f32, b: f32) -> f32 {
    a.max(b)
}

pub fn math_min(a: f32, b: f32) -> f32 {
    a.min(b)
}

pub fn math_norm(value: f32, start: f32, stop: f32) -> f32 {
    (value - start) / (stop - start)
}

pub fn math_pow(n: f32, e: f32) -> f32 {
    n.powf(e)
}

pub fn math_round(n: f32) -> f32 {
    n.round()
}

pub fn math_sq(n: f32) -> f32 {
    n * n
}

pub fn math_sqrt(n: f32) -> f32 {
    n.sqrt()
}

// Trigonometry

pub fn math_acos(value: f32) -> f32 {
    value.acos()
}

pub fn math_asin(value: f32) -> f32 {
    value.asin()
}

pub fn math_atan(value: f32) -> f32 {
    value.atan()
}

pub fn math_atan2(y: f32, x: f32) -> f32 {
    y.atan2(x)
}

pub fn math_cos(angle: f32) -> f32 {
    angle.cos()
}

pub fn math_sin(angle: f32) -> f32 {
    angle.sin()
}

pub fn math_tan(angle: f32) -> f32 {
    angle.tan()
}

pub fn math_degrees(radians: f32) -> f32 {
    radians.to_degrees()
}

pub fn math_radians(degrees: f32) -> f32 {
    degrees.to_radians()
}

// Random & Noise

pub fn math_random(min: f32, max: f32) -> f32 {
    let mut s = global().lock().unwrap();
    s.rng.gen_range(min..max)
}

pub fn math_random_seed(seed: u32) {
    let mut s = global().lock().unwrap();
    s.rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
}

pub fn math_random_gaussian(mean: f32, sd: f32) -> f32 {
    let mut s = global().lock().unwrap();
    let normal = Normal::new(mean, sd).unwrap_or(Normal::new(0.0, 1.0).unwrap());
    normal.sample(&mut s.rng)
}

pub fn math_noise(x: f32, y: f32, z: f32) -> f32 {
    let s = global().lock().unwrap();
    // Perlin noise implementation using the `noise` crate
    let perlin = Perlin::new(s.noise_seed);
    // Perlin noise typically returns values in range [-1, 1],
    // Processing/p5.js noise returns [0, 1].
    let val = perlin.get([x as f64, y as f64, z as f64]) as f32;
    (val + 1.0) / 2.0
}

pub fn math_noise_seed(seed: u32) {
    let mut s = global().lock().unwrap();
    s.noise_seed = seed;
}

pub fn math_noise_detail(_lod: u32, _falloff: f32) {
    // Currently we only use a single octave of Perlin noise.
    // In a full implementation, we would store these in state and use Fbm.
}

// System Date/Time

pub fn system_day() -> u32 {
    Local::now().day()
}

pub fn system_hour() -> u32 {
    Local::now().hour()
}

pub fn system_minute() -> u32 {
    Local::now().minute()
}

pub fn system_month() -> u32 {
    Local::now().month()
}

pub fn system_second() -> u32 {
    Local::now().second()
}

pub fn system_year() -> u32 {
    Local::now().year() as u32
}
