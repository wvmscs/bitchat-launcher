// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Ports of the page and of the map.

use std::net::{Ipv4Addr, TcpListener};
use std::ops::RangeInclusive;

/// The page's port when it is free.
pub const PREFERRED: u16 = 8080;
/// Where a free port is looked for otherwise, and for the map.
pub const RANGE: RangeInclusive<u16> = 50000..=60000;

/// Nobody listens on the port of the loopback.
pub fn is_free(port: u16) -> bool {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok()
}

/// `preferred` if free, else the first free port of `range` from a
/// pseudo-random place (two launchers do not race for the same one), never
/// one of `taken`.
pub fn pick(
    preferred: Option<u16>,
    range: RangeInclusive<u16>,
    taken: &[u16],
    seed: u64,
    free: impl Fn(u16) -> bool,
) -> Option<u16> {
    if let Some(p) = preferred
        && !taken.contains(&p)
        && free(p)
    {
        return Some(p);
    }
    let (lo, hi) = (*range.start() as u64, *range.end() as u64);
    let n = hi - lo + 1;
    (0..n)
        .map(|i| (lo + (seed + i) % n) as u16)
        .find(|p| !taken.contains(p) && free(*p))
}

/// A seed for `pick`, from the clock and the process.
pub fn seed() -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64);
    t ^ (std::process::id() as u64).rotate_left(17)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_when_free() {
        assert_eq!(pick(Some(8080), RANGE, &[], 0, |_| true), Some(8080));
    }

    #[test]
    fn range_when_busy_or_taken() {
        let free = |p: u16| p != 8080 && p != 50003;
        assert_eq!(pick(Some(8080), RANGE, &[], 3, free), Some(50004));
        assert_eq!(pick(Some(8080), RANGE, &[50004], 3, free), Some(50005));
        assert_eq!(pick(Some(50005), RANGE, &[50005], 5, |_| true), Some(50006));
        // Wraps round the end of the range.
        assert_eq!(pick(None, 10..=12, &[], 2, |p| p != 12), Some(10));
        assert_eq!(pick(None, 10..=12, &[11], 0, |p| p == 11), None);
    }

    #[test]
    fn real_ports() {
        let l = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let busy = l.local_addr().unwrap().port();
        assert!(!is_free(busy));
        let p = pick(Some(busy), RANGE, &[], seed(), is_free).unwrap();
        assert!(RANGE.contains(&p) && p != busy);
    }
}
