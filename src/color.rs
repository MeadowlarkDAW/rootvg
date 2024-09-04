//! This module re-exports types from the `rgb` crate.

pub use rgb::*;

pub type Color = Rgba<f32>;

/// The color black with full opacity
pub const BLACK: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
/// The color white with full opacity
pub const WHITE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
/// A color with no opacity
pub const TRANSPARENT: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

pub fn lerp_rgba(c0: Color, c1: Color, u: f32) -> Color {
    let u = u.clamp(0.0, 1.0);
    let one_minus_u = 1.0 - u;

    Color {
        r: c0.r * one_minus_u + c1.r * u,
        g: c0.g * one_minus_u + c1.g * u,
        b: c0.b * one_minus_u + c1.b * u,
        a: c0.a * one_minus_u + c1.a * u,
    }
}

pub fn hue(mut h: f32, m1: f32, m2: f32) -> f32 {
    if h < 0.0 {
        h += 1.0;
    } else if h > 1.0 {
        h -= 1.0;
    }

    if h < 1.0 / 6.0 {
        m1 + (m2 - m1) * h * 6.0
    } else if h < 3.0 / 6.0 {
        m2
    } else if h < 4.0 / 6.0 {
        m1 + (m2 - m1) * (2.0 / 3.0 - h) * 6.0
    } else {
        m1
    }
}

pub fn hsl(h: f32, s: f32, l: f32) -> Color {
    hsla(h, s, l, 1.0)
}

pub fn hsla(mut h: f32, s: f32, l: f32, a: f32) -> Color {
    h %= 1.0;
    if h < 0.0 {
        h += 1.0;
    }

    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    let m2 = if l <= 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let m1 = 2.0 * l - m2;

    Color {
        r: hue(h + 1.0 / 3.0, m1, m2).clamp(0.0, 1.0),
        g: hue(h, m1, m2).clamp(0.0, 1.0),
        b: hue(h - 1.0 / 3.0, m1, m2).clamp(0.0, 1.0),
        a: a.clamp(0.0, 1.0),
    }
}
