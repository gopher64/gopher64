use core::ops;

/// A point in 2-dimensional space, with each dimension of type `N`.
///
/// Legal operations on points are addition and subtraction by vectors, and
/// subtraction between points, to give a vector representing the offset between
/// the two points. Combined with the legal operations on vectors, meaningful
/// manipulations of vectors and points can be performed.
///
/// For example, to interpolate between two points by a factor `t`:
///
/// ```
/// # use rusttype::*;
/// # let t = 0.5; let p0 = point(0.0, 0.0); let p1 = point(0.0, 0.0);
/// let interpolated_point = p0 + (p1 - p0) * t;
/// ```
#[derive(Copy, Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Point<N> {
    pub x: N,
    pub y: N,
}

/// A vector in 2-dimensional space, with each dimension of type `N`.
///
/// Legal operations on vectors are addition and subtraction by vectors,
/// addition by points (to give points), and multiplication and division by
/// scalars.
#[derive(Copy, Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Vector<N> {
    pub x: N,
    pub y: N,
}

/// A convenience function for generating `Point`s.
#[inline]
pub fn point<N>(x: N, y: N) -> Point<N> {
    Point { x, y }
}
/// A convenience function for generating `Vector`s.
#[inline]
pub fn vector<N>(x: N, y: N) -> Vector<N> {
    Vector { x, y }
}

impl<N: ops::Sub<Output = N>> ops::Sub for Point<N> {
    type Output = Vector<N>;
    fn sub(self, rhs: Point<N>) -> Vector<N> {
        vector(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<N: ops::Add<Output = N>> ops::Add for Vector<N> {
    type Output = Vector<N>;
    fn add(self, rhs: Vector<N>) -> Vector<N> {
        vector(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<N: ops::Sub<Output = N>> ops::Sub for Vector<N> {
    type Output = Vector<N>;
    fn sub(self, rhs: Vector<N>) -> Vector<N> {
        vector(self.x - rhs.x, self.y - rhs.y)
    }
}

impl ops::Mul<f32> for Vector<f32> {
    type Output = Vector<f32>;
    fn mul(self, rhs: f32) -> Vector<f32> {
        vector(self.x * rhs, self.y * rhs)
    }
}

impl ops::Mul<Vector<f32>> for f32 {
    type Output = Vector<f32>;
    fn mul(self, rhs: Vector<f32>) -> Vector<f32> {
        vector(self * rhs.x, self * rhs.y)
    }
}

impl ops::Mul<f64> for Vector<f64> {
    type Output = Vector<f64>;
    fn mul(self, rhs: f64) -> Vector<f64> {
        vector(self.x * rhs, self.y * rhs)
    }
}

impl ops::Mul<Vector<f64>> for f64 {
    type Output = Vector<f64>;
    fn mul(self, rhs: Vector<f64>) -> Vector<f64> {
        vector(self * rhs.x, self * rhs.y)
    }
}

impl ops::Div<f32> for Vector<f32> {
    type Output = Vector<f32>;
    fn div(self, rhs: f32) -> Vector<f32> {
        vector(self.x / rhs, self.y / rhs)
    }
}

impl ops::Div<Vector<f32>> for f32 {
    type Output = Vector<f32>;
    fn div(self, rhs: Vector<f32>) -> Vector<f32> {
        vector(self / rhs.x, self / rhs.y)
    }
}

impl ops::Div<f64> for Vector<f64> {
    type Output = Vector<f64>;
    fn div(self, rhs: f64) -> Vector<f64> {
        vector(self.x / rhs, self.y / rhs)
    }
}

impl ops::Div<Vector<f64>> for f64 {
    type Output = Vector<f64>;
    fn div(self, rhs: Vector<f64>) -> Vector<f64> {
        vector(self / rhs.x, self / rhs.y)
    }
}

impl<N: ops::Add<Output = N>> ops::Add<Vector<N>> for Point<N> {
    type Output = Point<N>;
    fn add(self, rhs: Vector<N>) -> Point<N> {
        point(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<N: ops::Sub<Output = N>> ops::Sub<Vector<N>> for Point<N> {
    type Output = Point<N>;
    fn sub(self, rhs: Vector<N>) -> Point<N> {
        point(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<N: ops::Add<Output = N>> ops::Add<Point<N>> for Vector<N> {
    type Output = Point<N>;
    fn add(self, rhs: Point<N>) -> Point<N> {
        point(self.x + rhs.x, self.y + rhs.y)
    }
}

/// A rectangle, with top-left corner at `min`, and bottom-right corner at
/// `max`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rect<N> {
    pub min: Point<N>,
    pub max: Point<N>,
}

impl<N: ops::Sub<Output = N> + Copy> Rect<N> {
    pub fn width(&self) -> N {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> N {
        self.max.y - self.min.y
    }
}
