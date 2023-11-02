// SPDX-License-Identifier: MIT

/*
Inspired from http://blog.ivank.net/fastest-gaussian-blur.html the algorithm 4.
The struct MathColor is needed for the calculate with bigger numbers, the Color struct save the r,g,b values with a u8.
*/

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use core::ops::{Add, AddAssign, Sub};
use crate::color::Color;

#[derive(Copy, Clone)]
pub struct MathColor {
    pub r: isize,
    pub g: isize,
    pub b: isize,
}

impl MathColor {
    pub fn new(color: Color) -> Self {
        MathColor {
            r: color.r() as isize,
            g: color.g() as isize,
            b: color.b() as isize,
        }
    }

    pub fn get_multiplied_color(&mut self, iarr: f32) -> Color {
        Color::rgb(
            (self.r as f32 * iarr).round() as u8,
            (self.g as f32 * iarr).round() as u8,
            (self.b as f32 * iarr).round() as u8,
        )
    }
}

impl Add for MathColor {
    type Output = MathColor;

    #[inline(always)]
    fn add(self, color: MathColor) -> MathColor {
        MathColor {
            r: self.r + color.r,
            g: self.g + color.g,
            b: self.b + color.b,
        }
    }
}

impl AddAssign for MathColor {
    #[inline(always)]
    fn add_assign(&mut self, color: MathColor) {
        *self = MathColor {
            r: self.r + color.r,
            g: self.g + color.g,
            b: self.b + color.b,
        };
    }
}

impl Sub for MathColor {
    type Output = MathColor;

    #[inline(always)]
    fn sub(self, color: MathColor) -> MathColor {
        MathColor {
            r: self.r - color.r,
            g: self.g - color.g,
            b: self.b - color.b,
        }
    }
}

pub fn gauss_blur(data: &mut [Color], w: u32, h: u32, r: f32) {
    let bxs = boxes_for_gauss(r, 3);
    let mut tcl = data.to_owned();

    box_blur(
        &mut tcl,
        data,
        w as usize,
        h as usize,
        ((bxs[0] - 1) / 2) as usize,
    );
    box_blur(
        &mut tcl,
        data,
        w as usize,
        h as usize,
        ((bxs[1] - 1) / 2) as usize,
    );
    box_blur(
        &mut tcl,
        data,
        w as usize,
        h as usize,
        ((bxs[2] - 1) / 2) as usize,
    );
}

fn boxes_for_gauss(sigma: f32, n: usize) -> Vec<i32> {
    let w_ideal: f32 = ((12.0 * sigma * sigma / n as f32) + 1.0).sqrt();
    let mut wl: i32 = w_ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    };
    let wu: i32 = wl + 2;

    let m_ideal: f32 = (12.0 * sigma * sigma
        - n as f32 * wl as f32 * wl as f32
        - 4.0 * n as f32 * wl as f32
        - 3.0 * n as f32)
        / (-4.0 * wl as f32 - 4.0);
    let m: usize = m_ideal.round() as usize;

    let mut sizes = Vec::<i32>::new();
    for i in 0..n {
        sizes.push(if i < m { wl } else { wu });
    }
    sizes
}

fn box_blur(tcl: &mut [Color], scl: &mut [Color], w: usize, h: usize, r: usize) {
    box_blur_t(scl, tcl, w, h, r);
    box_blur_h(tcl, scl, w, h, r);
}

#[inline(always)]
fn box_blur_h(tcl: &mut [Color], scl: &mut [Color], w: usize, h: usize, r: usize) {
    let iarr: f32 = 1.0 / (r + r + 1) as f32;

    for i in 0..h {
        let mut ti: usize = i * w;
        let mut li: usize = ti;
        let mut ri: usize = ti + r;
        let fv = MathColor::new(tcl[ti]);
        let lv = MathColor::new(tcl[ti + w - 1]);

        let mut val: MathColor = MathColor {
            r: (r + 1) as isize * fv.r,
            g: (r + 1) as isize * fv.g,
            b: (r + 1) as isize * fv.b,
        };

        for j in 0..r {
            val += MathColor::new(tcl[ti + j]);
        }

        for _ in 0..(r + 1) {
            val += MathColor::new(tcl[ri]) - fv;
            scl[ti] = val.get_multiplied_color(iarr);
            ti += 1;
            ri += 1;
        }

        for _ in (r + 1)..(w - r) {
            val += MathColor::new(tcl[ri]) - MathColor::new(tcl[li]);
            scl[ti] = val.get_multiplied_color(iarr);
            ti += 1;
            ri += 1;
            li += 1;
        }

        for _ in (w - r)..w {
            val += lv - MathColor::new(tcl[li]);
            scl[ti] = val.get_multiplied_color(iarr);
            ti += 1;
            li += 1;
        }
    }
}

#[inline(always)]
fn box_blur_t(tcl: &mut [Color], scl: &mut [Color], w: usize, h: usize, r: usize) {
    let iarr: f32 = 1.0 / (r + r + 1) as f32;

    for i in 0..w {
        let mut ti: usize = i;
        let mut li: usize = ti;
        let mut ri: usize = ti + r * w;
        let fv = MathColor::new(tcl[ti]);
        let lv = MathColor::new(tcl[ti + w * (h - 1)]);

        let mut val: MathColor = MathColor {
            r: (r + 1) as isize * fv.r,
            g: (r + 1) as isize * fv.g,
            b: (r + 1) as isize * fv.b,
        };

        for j in 0..r {
            val += MathColor::new(tcl[ti + j * w]);
        }

        for _ in 0..(r + 1) {
            val += MathColor::new(tcl[ri]) - fv;
            scl[ti] = val.get_multiplied_color(iarr);
            ti += w;
            ri += w;
        }

        for _ in (r + 1)..(h - r) {
            val += MathColor::new(tcl[ri]) - MathColor::new(tcl[li]);
            scl[ti] = val.get_multiplied_color(iarr);
            ti += w;
            ri += w;
            li += w;
        }

        for _ in (h - r)..h {
            val += lv - MathColor::new(tcl[li]);
            scl[ti] = val.get_multiplied_color(iarr);
            ti += w;
            li += w;
        }
    }
}
