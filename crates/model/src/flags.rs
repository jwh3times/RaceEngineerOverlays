//! Sim-agnostic race-control flags.

use std::fmt;

/// A set of flags currently shown to the player.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Flags(u32);

impl Flags {
    pub const GREEN: Self = Self(1 << 0);
    pub const YELLOW: Self = Self(1 << 1);
    pub const YELLOW_WAVING: Self = Self(1 << 2);
    pub const CAUTION: Self = Self(1 << 3);
    pub const RED: Self = Self(1 << 4);
    pub const BLUE: Self = Self(1 << 5);
    pub const WHITE: Self = Self(1 << 6);
    pub const CHECKERED: Self = Self(1 << 7);
    pub const DEBRIS: Self = Self(1 << 8);
    pub const BLACK: Self = Self(1 << 9);
    /// Black flag with orange disc: the car must pit for repairs.
    pub const MEATBALL: Self = Self(1 << 10);
    pub const DISQUALIFIED: Self = Self(1 << 11);
    pub const ONE_LAP_TO_GREEN: Self = Self(1 << 12);
    pub const START_READY: Self = Self(1 << 13);
    pub const START_SET: Self = Self(1 << 14);
    pub const START_GO: Self = Self(1 << 15);

    const NAMES: [(Self, &'static str); 16] = [
        (Self::GREEN, "green"),
        (Self::YELLOW, "yellow"),
        (Self::YELLOW_WAVING, "yellow-waving"),
        (Self::CAUTION, "caution"),
        (Self::RED, "red"),
        (Self::BLUE, "blue"),
        (Self::WHITE, "white"),
        (Self::CHECKERED, "checkered"),
        (Self::DEBRIS, "debris"),
        (Self::BLACK, "black"),
        (Self::MEATBALL, "meatball"),
        (Self::DISQUALIFIED, "disqualified"),
        (Self::ONE_LAP_TO_GREEN, "one-lap-to-green"),
        (Self::START_READY, "start-ready"),
        (Self::START_SET, "start-set"),
        (Self::START_GO, "start-go"),
    ];

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn set(&mut self, other: Self, on: bool) {
        if on {
            self.0 |= other.0;
        } else {
            self.0 &= !other.0;
        }
    }

    pub fn names(self) -> impl Iterator<Item = &'static str> {
        Self::NAMES
            .into_iter()
            .filter(move |(f, _)| self.contains(*f))
            .map(|(_, n)| n)
    }
}

impl std::ops::BitOr for Flags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl fmt::Debug for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.names()).finish()
    }
}
