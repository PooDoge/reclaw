use std::time::Duration;

use crate::activity::*;

#[test]
fn sizes_use_decimal_units_with_two_significant_digits() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(999), "999 B");
    assert_eq!(format_bytes(1_500), "1.5 kB");
    assert_eq!(format_bytes(21_000_000), "21 MB");
    assert_eq!(format_bytes(1_400_000_000), "1.4 GB");
    assert_eq!(format_bytes(340_000), "340 kB");
}

#[test]
fn rates_are_sizes_per_second() {
    assert_eq!(format_rate(7_400_000), "7.4 MB/s");
}

#[test]
fn time_left_is_in_the_biggest_sensible_unit() {
    assert_eq!(format_eta(Duration::from_secs(0)), "1 s left");
    assert_eq!(format_eta(Duration::from_secs(35)), "35 s left");
    assert_eq!(format_eta(Duration::from_secs(61)), "2 min left");
    assert_eq!(format_eta(Duration::from_secs(3_599)), "60 min left");
    assert_eq!(format_eta(Duration::from_secs(3_600)), "1 h left");
    assert_eq!(format_eta(Duration::from_secs(7_800)), "2 h 10 min left");
    assert_eq!(format_eta(Duration::from_secs(3_600 + 3_599)), "2 h left", "59m59s rounds up to the next hour");
}
