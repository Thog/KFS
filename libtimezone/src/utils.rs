//! Utils used for local time managment.

use num_traits::ops::checked::CheckedAdd;
use num_traits::Num;

use core::cmp::PartialOrd;
use core::ops::{Div, Neg, Sub, SubAssign};

/// Increment the given ``ip`` with ``j`` if it doesn't overflow.
///
/// If the operation overflow, this return true otherwise false.
pub fn increment_overflow<T: Num + CheckedAdd + Copy>(ip: &mut T, j: T) -> bool {
    let res = ip.checked_add(&j);

    if let Some(value) = res {
        *ip = value;
        false
    } else {
        true
    }
}

/// Normalize and increment the given ``ip`` with the given ``unit`` and ``base`` if it doesn't overflow.
///
/// If the operation overflow, this return true otherwise false.
pub fn normalize_overflow<
    T: Num
        + Sub<Output = T>
        + Div<Output = T>
        + Neg<Output = T>
        + CheckedAdd
        + SubAssign
        + PartialOrd
        + Copy,
>(
    ip: &mut T,
    unit: &mut T,
    base: T,
) -> bool {
    let time_delta = if *unit >= T::zero() {
        *unit / base
    } else {
        -T::one() - (-T::one() - *unit) / base
    };

    *unit -= time_delta * base;

    increment_overflow(ip, time_delta)
}

/// Return true if it's a leap year.
#[inline]
pub fn is_leap_year(y: i64) -> bool {
    (((y) % 4) == 0 && (((y) % 100) != 0 || ((y) % 400) == 0))
}

/// Actual implementation of get_leap_days.
#[inline]
fn get_leap_days_not_neg(y: i64) -> i64 {
    y / 4 - y / 100 + y / 400
}

/// Get the count of leap days of the given year.
#[inline]
pub fn get_leap_days(y: i64) -> i64 {
    if y < 0 {
        -1 - get_leap_days_not_neg(-1 - y)
    } else {
        get_leap_days_not_neg(y)
    }
}