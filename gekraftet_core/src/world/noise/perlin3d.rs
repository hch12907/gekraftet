use cgmath::{ Point3, Vector3 };
use crate::utils::{ lerp, fade, Random };
use super::{ NoiseGen, NoiseGenOption };

use rand::{ Rng, SeedableRng };

/// A 3D Perlin noise generator. The implementation is largely similar to the
/// reference implementation by Ken Perlin
/// ([see here](https://mrl.nyu.edu/~perlin/noise/)). 
pub struct Perlin3D {
    pub octaves: u32,

    pub amplitude: f64,
    pub frequency: f32,
    pub lacunarity: f32, // lacunarity means "gap".
    pub persistance: f64,
    
    permutations: Box<[u8; 512]>,
}

impl Perlin3D {
    fn generate_noise(&mut self, pos: Point3<f32>) -> f64 {
        // for finding the point inside a cube (see `relative` below)
        let round_down = |a, b| if a as f32 > b { 
            b - (a - 1) as f32
        } else {
            b - a as f32
        };

        let grid = pos.cast::<i32>().unwrap();
        let cube = grid.map(|i| i & 255);
        let relative = Vector3::<f64>::new(
            round_down(grid.x, pos.x) as f64,
            round_down(grid.y, pos.y) as f64,
            round_down(grid.z, pos.z) as f64,
        );

        let weight_n = fade(relative.z);
        let weight_m = fade(relative.y);
        let weight_l = fade(relative.x);

        let  c = cube.x as usize;
        let  a = (self.permutations[c + 0] as i32 + cube.y) as usize;
        let aa = (self.permutations[a + 0] as i32 + cube.z) as usize;
        let ab = (self.permutations[a + 1] as i32 + cube.z) as usize;
        let  b = (self.permutations[c + 1] as i32 + cube.y) as usize;
        let ba = (self.permutations[b + 0] as i32 + cube.z) as usize;
        let bb = (self.permutations[b + 1] as i32 + cube.z) as usize;

        let grad = |hash, x: f64, y, z| -> f64 {
            let hash = hash & 15;
            let u = if hash < 8 { x } else { y };
            let v = if hash < 4 { 
                y 
            } else {
                if hash == 12 || hash == 14 { 
                    x 
                } else { 
                    z 
                }
            };
            let r0 = if hash & 1 == 0 { u } else { -u };
            let r1 = if hash & 2 == 0 { v } else { -v };
            r0 + r1
        };

        let (x, y, z) = (relative.x, relative.y, relative.z);

        let noise_000 = grad(self.permutations[aa+0], x,       y,       z      );
        let noise_001 = grad(self.permutations[ba+0], x - 1.0, y,       z      );
        let noise_010 = grad(self.permutations[ab+0], x,       y - 1.0, z      );
        let noise_011 = grad(self.permutations[bb+0], x - 1.0, y - 1.0, z      );
        let noise_100 = grad(self.permutations[aa+1], x,       y,       z - 1.0);
        let noise_101 = grad(self.permutations[ba+1], x - 1.0, y,       z - 1.0);
        let noise_110 = grad(self.permutations[ab+1], x,       y - 1.0, z - 1.0);
        let noise_111 = grad(self.permutations[bb+1], x - 1.0, y - 1.0, z - 1.0);

        let n1 = lerp(noise_000, noise_001, weight_n);
        let n2 = lerp(noise_010, noise_011, weight_n);
        let n3 = lerp(noise_100, noise_101, weight_n);
        let n4 = lerp(noise_110, noise_111, weight_n);

        let m1 = lerp(n1, n2, weight_m);
        let m2 = lerp(n3, n4, weight_m);

        let result = lerp(m1, m2, weight_l);
        result
    }
}

impl NoiseGen for Perlin3D {
    fn with_option_and_seed(option: NoiseGenOption, seed: u64) -> Self {
        let mut rng = Random::seed_from_u64(seed);
        
        let mut permutations = Box::new([0; 512]);
        for i in 0..256 {
            permutations[i] = rng.gen::<u8>();
            permutations[i + 256] = permutations[i];
        };
        
        Self {
            octaves: option.octaves,
            amplitude: option.amplitude,
            frequency: option.frequency,
            lacunarity: option.lacunarity,
            persistance: option.persistance,
            permutations,
        }
    }

    fn generate_noise_at(&mut self, pos: Point3<f32>) -> f64 {
        let mut total = 0.0;

        let mut amplitude = self.amplitude;
        let mut frequency = self.frequency;

        for _ in 0..self.octaves {
            total += self.generate_noise(pos * frequency) * amplitude;
            amplitude *= self.persistance;
            frequency *= self.lacunarity;
        };

        total
    }
}
