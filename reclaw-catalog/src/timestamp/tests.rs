use super::*;

fn secs(text: &str) -> i64 {
    Timestamp::parse(text).unwrap_or_else(|| panic!("{text} should parse")).secs
}

#[test]
fn known_instants_have_the_right_epoch_seconds() {
    assert_eq!(secs("1970-01-01T00:00:00Z"), 0);
    assert_eq!(secs("2000-03-01T00:00:00Z"), 951_868_800);
    assert_eq!(secs("2024-02-29T12:00:00Z"), 1_709_208_000, "a leap day");
    assert_eq!(secs("2026-10-04T12:29:25+00:00"), 1_791_116_965);
    assert_eq!(secs("1969-12-31T23:59:59Z"), -1);
    assert_eq!(secs("0001-01-01T00:00:00Z"), DEFAULT_SECS);
}

#[test]
fn an_offset_moves_the_instant_not_the_clock_reading() {
    assert_eq!(secs("2026-10-04T14:00:00+02:00"), secs("2026-10-04T12:00:00Z"));
    assert_eq!(secs("2026-10-04T07:30:00-04:30"), secs("2026-10-04T12:00:00Z"));
}

#[test]
fn the_seven_digit_fraction_dotnet_writes_is_read_and_ordered() {
    let a = Timestamp::parse("2026-10-04T12:29:25.4869655+00:00").expect("parses");
    assert_eq!((a.secs, a.nanos), (secs("2026-10-04T12:29:25Z"), 486_965_500));
    let b = Timestamp::parse("2026-10-04T12:28:25.2762956+00:00").expect("parses");
    assert!(b < a);
    assert!(Timestamp::parse("2026-10-04T12:29:25.5Z") > Timestamp::parse("2026-10-04T12:29:25.49Z"));
    assert_eq!(Timestamp::parse("2026-10-04T12:29:25.1234567891234Z").map(|t| t.nanos), Some(123_456_789), "beyond nanoseconds is dropped");
}

#[test]
fn what_is_not_a_complete_dated_instant_is_refused() {
    for bad in [
        "",
        "2026-10-04",
        "2026-10-04T12:00:00",
        "2026-10-04 12:00:00Z",
        "2026-13-01T00:00:00Z",
        "2026-02-29T00:00:00Z",
        "2026-04-31T00:00:00Z",
        "2026-10-04T24:00:00Z",
        "2026-10-04T12:60:00Z",
        "2026-10-04T12:00:60Z",
        "2026-10-04T12:00:00+24:00",
        "2026-10-04T12:00:00+0200",
        "2026-10-04T12:00:00.Z",
        "0000-01-01T00:00:00Z",
        "20261004T120000Z",
        "2026-10-04T12:00:00Zjunk",
        "-2026-10-04T12:00:00Z",
    ] {
        assert_eq!(Timestamp::parse(bad), None, "{bad:?}");
    }
    assert!(
        Timestamp::parse("2100-02-28T00:00:00Z").is_some() && Timestamp::parse("2100-02-29T00:00:00Z").is_none(),
        "2100 is not a leap year"
    );
    assert!(Timestamp::parse("2000-02-29T00:00:00Z").is_some(), "2000 is");
}

#[test]
fn the_default_value_is_recognised_and_the_clock_is_after_this_code_was_written() {
    assert!(Timestamp::parse("0001-01-01T00:00:00+00:00").is_some_and(Timestamp::is_default));
    assert!(!Timestamp::parse("2026-10-04T12:00:00Z").is_some_and(Timestamp::is_default));
    assert!(Timestamp::now().is_some_and(|t| t.secs > secs("2026-01-01T00:00:00Z")));
    assert_eq!(Timestamp::parse("2026-10-04T12:00:00Z").map(|t| t.plus_secs(300).secs), Some(secs("2026-10-04T12:05:00Z")));
}
