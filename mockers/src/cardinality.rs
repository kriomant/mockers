use std::cmp::Ordering;
use std::ops::{Range, RangeFrom, RangeFull, RangeTo};
use std::ops::{RangeInclusive, RangeToInclusive};

/// Result of checking call count against cardinality constraints
#[derive(PartialEq, Eq, Debug)]
pub enum CardinalityCheckResult {
    /// Call count doesn't satisfy cardinality constraints, but some larger count can.
    /// For example, call count 1 doesn't satisfy 2..4 cardinality, but subsequent
    /// call count will satisfy it, so result for 1 is `Possible`.
    Possible,

    /// Call count satisfies cardinality constraints, but it's possible that larger count won't.
    /// For example, call count 3 satisifies 2..4 cardinality, but subsequent call count 4 won't,
    /// so result for 3 is `Satisfied`.
    Satisfied,

    /// Call count doesn't satsify cardinality constraints and no larger counts will.
    /// For example, call count 4 doesn't satisfy 2..4 cardinality and all subsequent call counts
    /// don't, so result for 4 is `Wrong`.
    Wrong,
}

pub trait Cardinality {
    fn check(&self, count: u32) -> CardinalityCheckResult;
    fn describe(&self) -> String;
    fn describe_upper_bound(&self) -> String;
}

impl Cardinality for u32 {
    fn check(&self, count: u32) -> CardinalityCheckResult {
        match count.cmp(self) {
            Ordering::Equal => CardinalityCheckResult::Satisfied,
            Ordering::Greater => CardinalityCheckResult::Wrong,
            Ordering::Less => CardinalityCheckResult::Possible,
        }
    }

    fn describe(&self) -> String {
        match *self {
            0 => "never called".to_string(),
            1 => "called exactly one time".to_string(),
            n => format!("called exactly {} times", n),
        }
    }

    fn describe_upper_bound(&self) -> String {
        if *self == 1 {
            "called exactly one time".to_string()
        } else {
            format!("called exactly {} times", self)
        }
    }
}

impl Cardinality for Range<u32> {
    fn check(&self, count: u32) -> CardinalityCheckResult {
        if count < self.start {
            CardinalityCheckResult::Possible
        } else if count < self.end {
            CardinalityCheckResult::Satisfied
        } else {
            CardinalityCheckResult::Wrong
        }
    }

    fn describe(&self) -> String {
        match (self.start, self.end) {
            (_, 0) | (_, 1) => "never called".to_string(),
            (b, e) => format!("called from {} and less than {} times", b, e),
        }
    }

    fn describe_upper_bound(&self) -> String {
        if self.end == 1 {
            "never called".to_string()
        } else {
            format!("called at most {} times", self.end - 1)
        }
    }
}

impl Cardinality for RangeInclusive<u32> {
    fn check(&self, count: u32) -> CardinalityCheckResult {
        if count < *self.start() {
            CardinalityCheckResult::Possible
        } else if count <= *self.end() {
            CardinalityCheckResult::Satisfied
        } else {
            CardinalityCheckResult::Wrong
        }
    }

    fn describe(&self) -> String {
        match (*self.start(), *self.end()) {
            (_, 0) => "never called".to_string(),
            (_, 1) => "called at most one time".to_string(),
            (b, e) => format!("called from {} to {} times", b, e),
        }
    }

    fn describe_upper_bound(&self) -> String {
        if *self.end() == 0 {
            "never called".to_string()
        } else if *self.end() == 1 {
            "called at most one time".to_string()
        } else {
            format!("called at most {} times", self.end())
        }
    }
}

impl Cardinality for RangeFrom<u32> {
    fn check(&self, count: u32) -> CardinalityCheckResult {
        match count.cmp(&self.start) {
            Ordering::Equal | Ordering::Greater => CardinalityCheckResult::Satisfied,
            Ordering::Less => CardinalityCheckResult::Possible,
        }
    }

    fn describe(&self) -> String {
        match self.start {
            0 => "called any number of times".to_string(),
            1 => "called at least once".to_string(),
            n => format!("called at least {} times", n),
        }
    }

    fn describe_upper_bound(&self) -> String {
        unreachable!()
    }
}

impl Cardinality for RangeTo<u32> {
    fn check(&self, count: u32) -> CardinalityCheckResult {
        match count.cmp(&self.end) {
            Ordering::Less => CardinalityCheckResult::Satisfied,
            Ordering::Equal | Ordering::Greater => CardinalityCheckResult::Wrong,
        }
    }

    fn describe(&self) -> String {
        match self.end {
            0 | 1 => "never called".to_string(),
            n => format!("called less than {} times", n),
        }
    }

    fn describe_upper_bound(&self) -> String {
        if self.end <= 1 {
            "never called".to_string()
        } else {
            format!("called less than {} times", self.end)
        }
    }
}

impl Cardinality for RangeToInclusive<u32> {
    fn check(&self, count: u32) -> CardinalityCheckResult {
        match count.cmp(&self.end) {
            Ordering::Less | Ordering::Equal => CardinalityCheckResult::Satisfied,
            Ordering::Greater => CardinalityCheckResult::Wrong,
        }
    }

    fn describe(&self) -> String {
        match self.end {
            0 => "never called".to_string(),
            1 => "at most once".to_string(),
            n => format!("called no more than {} times", n),
        }
    }

    fn describe_upper_bound(&self) -> String {
        if self.end == 0 {
            "never called".to_string()
        } else if self.end == 1 {
            "called at most one time".to_string()
        } else {
            format!("called at most {} times", self.end)
        }
    }
}

impl Cardinality for RangeFull {
    fn check(&self, _: u32) -> CardinalityCheckResult {
        CardinalityCheckResult::Satisfied
    }

    fn describe(&self) -> String {
        "called any number of times".to_string()
    }

    fn describe_upper_bound(&self) -> String {
        unreachable!()
    }
}

pub struct Never;
pub fn never() -> Never {
    Never
}
impl Cardinality for Never {
    fn check(&self, count: u32) -> CardinalityCheckResult {
        if count == 0 {
            CardinalityCheckResult::Satisfied
        } else {
            CardinalityCheckResult::Wrong
        }
    }

    fn describe(&self) -> String {
        "never called".to_string()
    }

    fn describe_upper_bound(&self) -> String {
        "never called".to_string()
    }
}
