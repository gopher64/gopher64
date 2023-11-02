// SPDX-License-Identifier: MIT

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// point type (is the point a new position or a connection point)
pub enum PointType {
    Move,
    Connect,
    None,
}

/// graphic path with similar functions like html canvas
pub struct GraphicsPath {
    x: i32,
    y: i32,
    pub points: Vec<(i32, i32, PointType)>,
}

#[allow(clippy::new_without_default)]
impl GraphicsPath {
    pub fn new() -> GraphicsPath {
        GraphicsPath {
            x: 0,
            y: 0,
            points: Vec::new(),
        }
    }

    /// move to position
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.points.push((x, y, PointType::Move));
        self.x = x;
        self.y = y;
    }

    /// create a line between the last and new point
    pub fn line_to(&mut self, x: i32, y: i32) {
        self.points.push((x, y, PointType::Connect));
        self.x = x;
        self.y = y;
    }

    /// quadratic bezier curve
    pub fn quadratic_curve_to(&mut self, argx1: i32, argy1: i32, argx2: i32, argy2: i32) {
        let mut t: f32 = 0.0;
        let mut u: f32;
        let mut tt: f32;
        let mut uu: f32;
        let mut x: f32;
        let mut y: f32;

        while t < 1.0 {
            u = 1.0 - t;
            uu = u * u;
            tt = t * t;

            x = (self.x as f32) * uu;
            y = (self.y as f32) * uu;

            x += 2.0 * u * t * (argx1 as f32);
            y += 2.0 * u * t * (argy1 as f32);

            x += tt * (argx2 as f32);
            y += tt * (argy2 as f32);

            t += 0.01;
            self.points.push((x as i32, y as i32, PointType::Connect));
        }

        self.x = argx2;
        self.y = argy2;
    }

    /// cubic bezier curve
    pub fn bezier_curve_to(
        &mut self,
        argx1: i32,
        argy1: i32,
        argx2: i32,
        argy2: i32,
        argx3: i32,
        argy3: i32,
    ) {
        let mut t: f32 = 0.0;
        let mut u: f32;
        let mut tt: f32;
        let mut uu: f32;
        let mut uuu: f32;
        let mut ttt: f32;
        let mut x: f32;
        let mut y: f32;

        while t < 1.0 {
            u = 1.0 - t;
            tt = t * t;
            uu = u * u;
            uuu = uu * u;
            ttt = tt * t;

            x = (self.x as f32) * uuu;
            y = (self.y as f32) * uuu;

            x += 3.0 * uu * t * (argx1 as f32);
            y += 3.0 * uu * t * (argy1 as f32);

            x += 3.0 * u * tt * (argx2 as f32);
            y += 3.0 * u * tt * (argy2 as f32);

            x += ttt * (argx3 as f32);
            y += ttt * (argy3 as f32);

            t += 0.01;
            self.points.push((x as i32, y as i32, PointType::Connect));
        }

        self.x = argx3;
        self.y = argy3;
    }
}
