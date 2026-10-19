//! Easing for the design system's `--ease: cubic-bezier(.2,.8,.2,1)`.

/// A CSS-style cubic Bézier easing curve through (0,0) and (1,1).
#[derive(Debug, Clone, Copy)]
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// The design system curve.
    pub const PATINA: Self = Self::new(0.2, 0.8, 0.2, 1.0);

    fn sample(a: f32, b: f32, t: f32) -> f32 {
        // Bézier with endpoints 0 and 1 and control values a, b.
        3.0 * a * (1.0 - t) * (1.0 - t) * t + 3.0 * b * (1.0 - t) * t * t + t * t * t
    }

    /// Eased output for linear progress `x` in 0..=1.
    pub fn ease(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        // Solve sample_x(t) = x by bisection; 24 steps is far below one pixel.
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if Self::sample(self.x1, self.x2, mid) < x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Self::sample(self.y1, self.y2, (lo + hi) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_fixed() {
        let c = CubicBezier::PATINA;
        assert!(c.ease(0.0).abs() < 1e-4);
        assert!((c.ease(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn starts_fast_and_settles() {
        let c = CubicBezier::PATINA;
        // `ease-out`-like: well past linear early on, monotonic throughout.
        assert!(c.ease(0.25) > 0.5);
        let mut prev = 0.0;
        for i in 0..=100 {
            let y = c.ease(i as f32 / 100.0);
            assert!(y >= prev - 1e-4);
            prev = y;
        }
    }
}
