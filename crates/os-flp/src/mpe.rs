// Copyright (C) 2026 redflameent-ux
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! The companion file that keeps a project's per-note expression curves (MPE, `os_model::mpe`). FL Studio's note
//! record has no room for them and FL may refuse a project file it doesn't recognise, so they are not written into
//! the `.flp`: they go into `<project>.flp.mpe` beside it, a small text file. FL Studio ignores it; Lemon Studio reads
//! it back when it opens the project.
//!
//! One line per curve: `pattern position channel key occurrence expression x:y x:y ...`, where `occurrence` counts
//! earlier notes of the pattern with the same position, channel and key (a doubled note), `x` is the point's
//! position over the note (0..=65535) and `y` its value. Lines starting with `#` are comments.

use os_model::mpe::{Curve, Expr};
use os_model::Project;
use std::path::{Path, PathBuf};

const HEADER: &str = "# Lemon Studio per-note expression (MPE) curves, version 1";

/// Where the companion file of `flp` lives.
pub fn path_for(flp: &Path) -> PathBuf {
    let mut s = flp.as_os_str().to_owned();
    s.push(".mpe");
    PathBuf::from(s)
}

/// The companion file's text for `project`, `None` when no note has a curve (no file needed).
pub fn write(project: &Project) -> Option<String> {
    let mut out = String::new();
    for pat in &project.patterns {
        let mut seen: Vec<(u32, u16, u16)> = vec![];
        for n in &pat.notes {
            let id = (n.position, n.channel, n.key);
            let occurrence = seen.iter().filter(|&&s| s == id).count();
            seen.push(id);
            for e in Expr::ALL {
                let c = n.mpe.get(e);
                if c.is_empty() {
                    continue;
                }
                out.push_str(&format!("{} {} {} {} {} {}", pat.number, n.position, n.channel, n.key, occurrence, e.tag()));
                for (x, y) in c.points() {
                    out.push_str(&format!(" {x}:{y}"));
                }
                out.push('\n');
            }
        }
    }
    (!out.is_empty()).then(|| format!("{HEADER}\n{out}"))
}

/// Puts the curves in `text` on `project`'s notes; lines that no longer match a note are skipped. Returns how many
/// curves were placed.
pub fn read(project: &mut Project, text: &str) -> usize {
    let mut placed = 0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut f = line.split_whitespace();
        let mut num = || f.next().and_then(|s| s.parse::<u64>().ok());
        let (Some(pattern), Some(position), Some(channel), Some(key), Some(occurrence)) = (num(), num(), num(), num(), num())
        else {
            continue;
        };
        let Some(e) = f.next().and_then(Expr::from_tag) else { continue };
        let (lo, hi) = e.range();
        let points: Vec<(u16, i16)> = f
            .filter_map(|p| {
                let (x, y) = p.split_once(':')?;
                Some((x.parse::<u16>().ok()?, y.parse::<i32>().ok()?.clamp(i32::from(lo), i32::from(hi)) as i16))
            })
            .collect();
        if points.is_empty() {
            continue;
        }
        let Some(pat) = project.patterns.iter_mut().find(|p| u64::from(p.number) == pattern) else { continue };
        let same = |n: &os_model::Note, by_channel: bool| {
            u64::from(n.position) == position && u64::from(n.key) == key && (!by_channel || u64::from(n.channel) == channel)
        };
        // By channel first; a project whose channels were renumbered since still finds its notes by place and key.
        let found = pat.notes.iter_mut().filter(|n| same(n, true)).nth(occurrence as usize).is_some();
        let note = if found {
            pat.notes.iter_mut().filter(|n| same(n, true)).nth(occurrence as usize)
        } else {
            pat.notes.iter_mut().filter(|n| same(n, false)).nth(occurrence as usize)
        };
        if let Some(n) = note {
            *n.mpe.get_mut(e) = Curve::from_points(&points);
            placed += 1;
        }
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_model::{Note, Pattern};

    fn project() -> Project {
        let mut a = Note::step(0, 0);
        a.length = 96;
        let mut b = a;
        b.key = 64;
        a.mpe.bend = Curve::from_points(&[(0, 0), (Curve::END, 200)]);
        b.mpe.pressure = Curve::from_points(&[(0, 1000), (30000, 250)]);
        b.mpe.slide = Curve::from_points(&[(100, 600)]);
        Project { patterns: vec![Pattern { number: 1, notes: vec![a, b], ..Default::default() }], ..Default::default() }
    }

    #[test]
    fn curves_round_trip_through_the_companion_text() {
        let p = project();
        let text = write(&p).expect("curves");
        let mut q = p.clone();
        for n in &mut q.patterns[0].notes {
            n.mpe = Default::default();
        }
        assert_eq!(read(&mut q, &text), 3);
        assert_eq!(q.patterns[0].notes, p.patterns[0].notes);
    }

    #[test]
    fn no_curves_no_file() {
        let mut p = project();
        for n in &mut p.patterns[0].notes {
            n.mpe = Default::default();
        }
        assert_eq!(write(&p), None);
    }

    #[test]
    fn renumbered_channels_still_find_their_notes() {
        let p = project();
        let text = write(&p).unwrap();
        let mut q = p.clone();
        for n in &mut q.patterns[0].notes {
            n.mpe = Default::default();
            n.channel = 7;
        }
        assert_eq!(read(&mut q, &text), 3);
    }

    #[test]
    fn path_sits_beside_the_project() {
        assert_eq!(path_for(Path::new("/a/Song.flp")), PathBuf::from("/a/Song.flp.mpe"));
    }
}
