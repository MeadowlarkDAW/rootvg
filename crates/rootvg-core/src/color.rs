//! This module re-exports the types from the [`rgb`](https://crates.io/crates/rgb) crate.

// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/core/src/color.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

pub use rgb::*;

/// The color black with full opacity
pub const BLACK: RGBA8 = RGBA8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
/// The color white with full opacity
pub const WHITE: RGBA8 = RGBA8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
/// A color with no opacity
pub const TRANSPARENT: RGBA8 = RGBA8 {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

/// A color packed as 4 floats representing RGBA channels.
///
/// Note that the color is assumed to be in SRGB format.
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq, bytemuck::Zeroable, bytemuck::Pod)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PackedSrgb(pub [f32; 4]);

impl PackedSrgb {
    /// The color black with full opacity
    pub const BLACK: Self = Self([0.0, 0.0, 0.0, 1.0]);
    /// The color white with full opacity
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);
    /// A color with no opacity
    pub const TRANSPARENT: Self = Self([0.0, 0.0, 0.0, 0.0]);

    /// Creates a [`Color`] from its SRGBA components.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }

    /// Creates a [`Color`] from 8 bit SRGBA components.
    pub const fn from_srgb8(r: u8, g: u8, b: u8) -> Self {
        let r = FROM_SRGB8_TABLE[r as usize];
        let g = FROM_SRGB8_TABLE[g as usize];
        let b = FROM_SRGB8_TABLE[b as usize];
        Self([r, g, b, 1.0])
    }

    /// Creates a [`Color`] from 8 bit SRGBA components.
    pub fn from_srgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        RGBA8 { r, g, b, a }.into()
    }

    /// Creates a [`Color`] from 8 bit SRGB component and an alpha component.
    pub const fn from_srgb8_alpha(r: u8, g: u8, b: u8, a: f32) -> Self {
        let r = FROM_SRGB8_TABLE[r as usize];
        let g = FROM_SRGB8_TABLE[g as usize];
        let b = FROM_SRGB8_TABLE[b as usize];
        Self([r, g, b, a])
    }

    /// Creates a [`Color`] from its linear RGBA components.
    pub fn from_linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        // As described in: https://en.wikipedia.org/wiki/SRGB
        fn gamma_component(u: f32) -> f32 {
            if u < 0.0031308 {
                12.92 * u
            } else {
                1.055 * u.powf(1.0 / 2.4) - 0.055
            }
        }

        Self([
            gamma_component(r),
            gamma_component(g),
            gamma_component(b),
            a,
        ])
    }

    pub fn r(&self) -> f32 {
        self.0[0]
    }
    pub fn g(&self) -> f32 {
        self.0[1]
    }
    pub fn b(&self) -> f32 {
        self.0[2]
    }
    pub fn a(&self) -> f32 {
        self.0[3]
    }

    pub fn r_mut(&mut self) -> &mut f32 {
        &mut self.0[0]
    }
    pub fn g_mut(&mut self) -> &mut f32 {
        &mut self.0[1]
    }
    pub fn b_mut(&mut self) -> &mut f32 {
        &mut self.0[2]
    }
    pub fn a_mut(&mut self) -> &mut f32 {
        &mut self.0[3]
    }
}

impl From<RGB8> for PackedSrgb {
    fn from(color: RGB8) -> Self {
        let r = FROM_SRGB8_TABLE[color.r as usize];
        let g = FROM_SRGB8_TABLE[color.g as usize];
        let b = FROM_SRGB8_TABLE[color.b as usize];
        Self([r, g, b, 1.0])
    }
}

impl From<RGBA8> for PackedSrgb {
    fn from(color: RGBA8) -> Self {
        let r = FROM_SRGB8_TABLE[color.r as usize];
        let g = FROM_SRGB8_TABLE[color.g as usize];
        let b = FROM_SRGB8_TABLE[color.b as usize];
        let a = f32::from(color.a) * (1.0 / 255.0);
        Self([r, g, b, a])
    }
}

impl From<[f32; 3]> for PackedSrgb {
    fn from(color: [f32; 3]) -> Self {
        Self([color[0], color[1], color[2], 1.0])
    }
}

impl From<[f32; 4]> for PackedSrgb {
    fn from(color: [f32; 4]) -> Self {
        Self(color)
    }
}

// -- The following code was copied from https://github.com/thomcc/fast-srgb8/blob/3e430039d5f252e896a174ebc7d8eb3aa1e12d95/src/lib.rs ---------------
// fast-srgb8 licenses:
//     * MIT: https://github.com/thomcc/fast-srgb8/blob/3e430039d5f252e896a174ebc7d8eb3aa1e12d95/LICENSE-MIT
//     * Apache License 2.0: https://github.com/thomcc/fast-srgb8/blob/3e430039d5f252e896a174ebc7d8eb3aa1e12d95/LICENSE-APACHE
//     * CC0: https://github.com/thomcc/fast-srgb8/blob/3e430039d5f252e896a174ebc7d8eb3aa1e12d95/LICENSE-CC0

/// Convert from 8-bit sRGB to linear f32.
pub const fn srgb8_to_f32(c: u8) -> f32 {
    FROM_SRGB8_TABLE[c as usize]
}

#[rustfmt::skip]
const FROM_SRGB8_TABLE: [f32; 256] = [
    0.0, 0.000303527, 0.000607054, 0.00091058103, 0.001214108, 0.001517635, 0.0018211621, 0.002124689,
    0.002428216, 0.002731743, 0.00303527, 0.0033465356, 0.003676507, 0.004024717, 0.004391442,
    0.0047769533, 0.005181517, 0.0056053917, 0.0060488326, 0.006512091, 0.00699541, 0.0074990317,
    0.008023192, 0.008568125, 0.009134057, 0.009721218, 0.010329823, 0.010960094, 0.011612245,
    0.012286487, 0.012983031, 0.013702081, 0.014443844, 0.015208514, 0.015996292, 0.016807375,
    0.017641952, 0.018500218, 0.019382361, 0.020288562, 0.02121901, 0.022173883, 0.023153365,
    0.02415763, 0.025186857, 0.026241222, 0.027320892, 0.028426038, 0.029556843, 0.03071345, 0.03189604,
    0.033104774, 0.03433981, 0.035601325, 0.036889452, 0.038204376, 0.039546248, 0.04091521, 0.042311423,
    0.043735042, 0.045186214, 0.046665095, 0.048171833, 0.049706575, 0.051269468, 0.052860655, 0.05448028,
    0.056128494, 0.057805434, 0.05951124, 0.06124607, 0.06301003, 0.06480328, 0.06662595, 0.06847818,
    0.07036011, 0.07227186, 0.07421358, 0.07618539, 0.07818743, 0.08021983, 0.082282715, 0.084376216,
    0.086500466, 0.088655606, 0.09084173, 0.09305898, 0.095307484, 0.09758736, 0.09989874, 0.10224175,
    0.10461649, 0.10702311, 0.10946172, 0.111932434, 0.11443538, 0.116970696, 0.11953845, 0.12213881,
    0.12477186, 0.12743773, 0.13013652, 0.13286836, 0.13563336, 0.13843165, 0.14126332, 0.1441285,
    0.1470273, 0.14995982, 0.15292618, 0.1559265, 0.15896086, 0.16202943, 0.16513224, 0.16826946,
    0.17144115, 0.17464745, 0.17788847, 0.1811643, 0.18447503, 0.1878208, 0.19120172, 0.19461787,
    0.19806935, 0.2015563, 0.20507877, 0.2086369, 0.21223079, 0.21586053, 0.21952623, 0.22322798,
    0.22696589, 0.23074007, 0.23455065, 0.23839766, 0.2422812, 0.2462014, 0.25015837, 0.25415218,
    0.2581829, 0.26225072, 0.26635566, 0.27049786, 0.27467737, 0.27889434, 0.2831488, 0.2874409,
    0.2917707, 0.29613832, 0.30054384, 0.30498737, 0.30946895, 0.31398875, 0.31854683, 0.32314324,
    0.32777813, 0.33245158, 0.33716366, 0.34191445, 0.3467041, 0.3515327, 0.35640025, 0.36130688,
    0.3662527, 0.37123778, 0.37626222, 0.3813261, 0.38642952, 0.39157256, 0.3967553, 0.40197787,
    0.4072403, 0.4125427, 0.41788515, 0.42326775, 0.42869055, 0.4341537, 0.43965724, 0.44520125,
    0.45078585, 0.45641106, 0.46207705, 0.46778384, 0.47353154, 0.47932024, 0.48514998, 0.4910209,
    0.49693304, 0.5028866, 0.50888145, 0.5149178, 0.5209957, 0.52711535, 0.5332766, 0.5394797,
    0.5457247, 0.5520116, 0.5583406, 0.5647117, 0.57112503, 0.57758063, 0.5840786, 0.590619, 0.597202,
    0.60382754, 0.61049575, 0.61720675, 0.62396055, 0.63075733, 0.637597, 0.6444799, 0.6514058,
    0.65837497, 0.66538745, 0.67244333, 0.6795426, 0.68668544, 0.69387203, 0.70110214, 0.70837605,
    0.7156938, 0.72305536, 0.730461, 0.7379107, 0.7454045, 0.75294244, 0.76052475, 0.7681514, 0.77582246,
    0.78353804, 0.79129815, 0.79910296, 0.8069525, 0.8148468, 0.822786, 0.8307701, 0.83879924, 0.84687346,
    0.8549928, 0.8631574, 0.87136734, 0.8796226, 0.8879232, 0.89626956, 0.90466136, 0.913099, 0.92158204,
    0.93011117, 0.9386859, 0.9473069, 0.9559735, 0.9646866, 0.9734455, 0.98225087, 0.9911022, 1.0
];

// --------------------------------------------------------------------------------------------
