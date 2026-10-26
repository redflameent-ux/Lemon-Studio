// Copyright (C) 2026 redflameent-ux
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Per-note expression (MPE): curves drawn on a single note in the piano roll for its pitch bend, pressure and slide
//! (timbre / brightness, MPE's CC 74). Lemon Studio only; FL Studio's note record has no room for them, so a project
//! keeps them in a companion file next to the `.flp` (`os_flp::mpe`).
//!
//! A curve's points sit at a fraction of the note's length (0 = its start, [`Curve::END`] = its end), so a curve
//! stretches with its note and survives timebase changes. Between points the value moves in a straight line; before
//! the first point and after the last it holds.

/// One expression of a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Expr {
    /// Pitch bend in cents, `-MAX_BEND..=MAX_BEND`.
    Bend,
    /// Pressure (MPE channel pressure), `0..=1000`.
    Pressure,
    /// Slide (MPE CC 74, timbre / brightness), `0..=1000`.
    Slide,
}

impl Expr {
    pub const ALL: [Expr; 3] = [Expr::Bend, Expr::Pressure, Expr::Slide];

    /// Largest bend either way, in cents (MPE's default per-note bend range is 48 semitones).
    pub const MAX_BEND: i16 = 4800;

    /// Range of the stored value.
    pub fn range(self) -> (i16, i16) {
        match self {
            Expr::Bend => (-Self::MAX_BEND, Self::MAX_BEND),
            Expr::Pressure | Expr::Slide => (0, 1000),
        }
    }

    /// The value a note has when it has no curve (what MPE controllers rest at).
    pub fn rest(self) -> i16 {
        match self {
            Expr::Bend | Expr::Pressure => 0,
            Expr::Slide => 500,
        }
    }

    /// The stored value as the plugin-facing value: semitones for a bend, 0..=1 otherwise.
    pub fn to_plugin(self, v: f32) -> f32 {
        match self {
            Expr::Bend => v / 100.0,
            Expr::Pressure | Expr::Slide => (v / 1000.0).clamp(0.0, 1.0),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Expr::Bend => "MPE pitch bend",
            Expr::Pressure => "MPE pressure",
            Expr::Slide => "MPE slide",
        }
    }

    /// Short key used in the companion file.
    pub fn tag(self) -> &'static str {
        match self {
            Expr::Bend => "bend",
            Expr::Pressure => "pressure",
            Expr::Slide => "slide",
        }
    }

    pub fn from_tag(s: &str) -> Option<Expr> {
        Expr::ALL.into_iter().find(|e| e.tag() == s)
    }
}

/// A curve over a note's length: up to [`Curve::MAX`] points, sorted by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Curve {
    len: u8,
    pts: [(u16, i16); Curve::MAX],
}

impl Default for Curve {
    fn default() -> Self {
        Curve { len: 0, pts: [(0, 0); Curve::MAX] }
    }
}

impl Curve {
    /// Most points a curve keeps; painting more thins out the ones that matter least.
    pub const MAX: usize = 24;
    /// Position of the note's end.
    pub const END: u16 = u16::MAX;

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn points(&self) -> &[(u16, i16)] {
        &self.pts[..usize::from(self.len)]
    }

    /// A curve from points (sorted and thinned to [`Curve::MAX`]).
    pub fn from_points(points: &[(u16, i16)]) -> Curve {
        let mut v: Vec<(u16, i16)> = points.to_vec();
        v.sort_by_key(|p| p.0);
        v.dedup_by_key(|p| p.0);
        let mut c = Curve::default();
        c.store(v);
        c
    }

    fn store(&mut self, mut v: Vec<(u16, i16)>) {
        thin(&mut v, Self::MAX);
        self.len = v.len() as u8;
        self.pts = [(0, 0); Curve::MAX];
        self.pts[..v.len()].copy_from_slice(&v);
    }

    /// The value at `at` (a fraction of the note, 0..=1), `None` without points.
    pub fn value_at(&self, at: f32) -> Option<f32> {
        let p = self.points();
        let first = p.first()?;
        let x = at.clamp(0.0, 1.0) * f32::from(Self::END);
        if x <= f32::from(first.0) {
            return Some(f32::from(first.1));
        }
        for w in p.windows(2) {
            let (a, b) = (w[0], w[1]);
            if x <= f32::from(b.0) {
                let span = f32::from(b.0) - f32::from(a.0);
                let t = if span > 0.0 { (x - f32::from(a.0)) / span } else { 1.0 };
                return Some(f32::from(a.1) + (f32::from(b.1) - f32::from(a.1)) * t);
            }
        }
        Some(f32::from(p[p.len() - 1].1))
    }

    /// Paints a straight stroke from `(a, va)` to `(b, vb)` (fractions of the note): points under it are replaced.
    pub fn paint(&mut self, a: f32, va: i16, b: f32, vb: i16) {
        let (a, va, b, vb) = if a <= b { (a, va, b, vb) } else { (b, vb, a, va) };
        let (pa, pb) = (frac_pos(a), frac_pos(b));
        let mut v: Vec<(u16, i16)> = self.points().iter().copied().filter(|p| p.0 < pa || p.0 > pb).collect();
        v.push((pa, va));
        if pb != pa {
            v.push((pb, vb));
        }
        v.sort_by_key(|p| p.0);
        self.store(v);
    }

    /// Removes the points between `a` and `b` (fractions of the note).
    pub fn erase(&mut self, a: f32, b: f32) {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let (pa, pb) = (frac_pos(a), frac_pos(b));
        let v: Vec<(u16, i16)> = self.points().iter().copied().filter(|p| p.0 < pa || p.0 > pb).collect();
        self.store(v);
    }

    /// The two halves of the curve cut at `at` (a fraction of the note), each over its own half's length.
    pub fn split(&self, at: f32) -> (Curve, Curve) {
        if self.is_empty() || !(0.0..1.0).contains(&at) || at <= 0.0 {
            return (*self, *self);
        }
        let cut = self.value_at(at).unwrap_or(0.0).round() as i16;
        let pc = f32::from(Self::END) * at;
        let (mut l, mut r) = (vec![], vec![]);
        for &(x, y) in self.points() {
            let xf = f32::from(x);
            if xf < pc {
                l.push((frac_pos(xf / pc), y));
            } else if xf > pc {
                r.push((frac_pos((xf - pc) / (f32::from(Self::END) - pc)), y));
            }
        }
        l.push((Self::END, cut));
        r.insert(0, (0, cut));
        (Curve::from_points(&l), Curve::from_points(&r))
    }

    /// The curve played backwards (for flipping notes in time).
    pub fn reversed(&self) -> Curve {
        let v: Vec<(u16, i16)> = self.points().iter().map(|&(x, y)| (Self::END - x, y)).collect();
        Curve::from_points(&v)
    }
}

fn frac_pos(f: f32) -> u16 {
    (f.clamp(0.0, 1.0) * f32::from(Curve::END)).round() as u16
}

/// Thins `v` to at most `max` points, each time dropping the inner point whose removal changes the curve least.
fn thin(v: &mut Vec<(u16, i16)>, max: usize) {
    while v.len() > max {
        let mut best = (f32::MAX, 1);
        for i in 1..v.len() - 1 {
            let (a, p, b) = (v[i - 1], v[i], v[i + 1]);
            let span = f32::from(b.0) - f32::from(a.0);
            let t = if span > 0.0 { (f32::from(p.0) - f32::from(a.0)) / span } else { 0.0 };
            let line = f32::from(a.1) + (f32::from(b.1) - f32::from(a.1)) * t;
            let err = (line - f32::from(p.1)).abs();
            if err < best.0 {
                best = (err, i);
            }
        }
        v.remove(best.1);
    }
}

/// A note's expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoteMpe {
    pub bend: Curve,
    pub pressure: Curve,
    pub slide: Curve,
}

impl NoteMpe {
    pub fn is_empty(&self) -> bool {
        self.bend.is_empty() && self.pressure.is_empty() && self.slide.is_empty()
    }

    pub fn get(&self, e: Expr) -> &Curve {
        match e {
            Expr::Bend => &self.bend,
            Expr::Pressure => &self.pressure,
            Expr::Slide => &self.slide,
        }
    }

    pub fn get_mut(&mut self, e: Expr) -> &mut Curve {
        match e {
            Expr::Bend => &mut self.bend,
            Expr::Pressure => &mut self.pressure,
            Expr::Slide => &mut self.slide,
        }
    }

    /// The two halves cut at `at` (a fraction of the note).
    pub fn split(&self, at: f32) -> (NoteMpe, NoteMpe) {
        let (b0, b1) = self.bend.split(at);
        let (p0, p1) = self.pressure.split(at);
        let (s0, s1) = self.slide.split(at);
        (NoteMpe { bend: b0, pressure: p0, slide: s0 }, NoteMpe { bend: b1, pressure: p1, slide: s1 })
    }

    pub fn reversed(&self) -> NoteMpe {
        NoteMpe { bend: self.bend.reversed(), pressure: self.pressure.reversed(), slide: self.slide.reversed() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_curve_has_no_value() {
        assert_eq!(Curve::default().value_at(0.5), None);
    }

    #[test]
    fn values_interpolate_and_hold() {
        let c = Curve::from_points(&[(Curve::END / 4, 0), (Curve::END / 4 * 3, 1000)]);
        assert_eq!(c.value_at(0.0), Some(0.0));
        assert!((c.value_at(0.5).unwrap() - 500.0).abs() < 1.0);
        assert_eq!(c.value_at(1.0), Some(1000.0));
    }

    #[test]
    fn painting_replaces_points_under_the_stroke_and_stays_within_max() {
        let mut c = Curve::default();
        for i in 0..200 {
            let a = i as f32 / 200.0;
            c.paint(a, (i * 10) as i16, a + 0.005, (i * 10 + 5) as i16);
        }
        assert!(c.points().len() <= Curve::MAX);
        assert!((c.value_at(0.5).unwrap() - 1000.0).abs() < 60.0);
        c.paint(0.4, -100, 0.6, -100);
        assert_eq!(c.value_at(0.5), Some(-100.0));
        c.erase(0.0, 1.0);
        assert!(c.is_empty());
    }

    #[test]
    fn split_keeps_the_shape_on_both_halves() {
        let c = Curve::from_points(&[(0, 0), (Curve::END, 1000)]);
        let (l, r) = c.split(0.5);
        assert_eq!(l.value_at(0.0), Some(0.0));
        assert!((l.value_at(1.0).unwrap() - 500.0).abs() < 1.0);
        assert!((r.value_at(0.0).unwrap() - 500.0).abs() < 1.0);
        assert_eq!(r.value_at(1.0), Some(1000.0));
        assert!((r.value_at(0.5).unwrap() - 750.0).abs() < 2.0);
    }

    #[test]
    fn reversed_mirrors() {
        let c = Curve::from_points(&[(0, 0), (Curve::END, 1000)]).reversed();
        assert_eq!(c.value_at(0.0), Some(1000.0));
        assert_eq!(c.value_at(1.0), Some(0.0));
    }
}
