//! A point in time read from the text the platform metadata uses (RFC 3339 / ISO 8601 with an offset). The format
//! only ever needs "is this before that, give or take five minutes", so this is a total order on instants and
//! nothing more; there is no calendar, no zone database and no formatting.
use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since 1970-01-01T00:00:00Z, and the nanoseconds past that second. Orders by instant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Timestamp {
    pub secs: i64,
    pub nanos: u32,
}

/// 0001-01-01T00:00:00Z, which .NET calls `default` and the format treats as "not given".
const DEFAULT_SECS: i64 = -62_135_596_800;

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        _ => 28,
    }
}

/// Days from 1970-01-01 to a date in the proleptic Gregorian calendar (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// `n` digits at the front of `text` as a number, and the rest.
fn digits(text: &str, n: usize) -> Option<(i64, &str)> {
    let head = text.get(..n)?;
    head.bytes().all(|b| b.is_ascii_digit()).then(|| head.parse().ok()).flatten().map(|v| (v, &text[n..]))
}

fn expect(text: &str, c: char) -> Option<&str> {
    text.strip_prefix(c)
}

impl Timestamp {
    /// Parse `YYYY-MM-DDThh:mm:ss[.fraction](Z|±hh:mm)`. Anything else, including a time with no offset (which would
    /// mean "local time" and so a different instant on different machines) or an impossible date, is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let (year, rest) = digits(text.trim(), 4)?;
        let (month, rest) = digits(expect(rest, '-')?, 2)?;
        let (day, rest) = digits(expect(rest, '-')?, 2)?;
        let (hour, rest) = digits(expect(rest, 'T')?, 2)?;
        let (minute, rest) = digits(expect(rest, ':')?, 2)?;
        let (second, mut rest) = digits(expect(rest, ':')?, 2)?;
        let mut nanos = 0u32;
        if let Some(after) = rest.strip_prefix('.') {
            let len = after.bytes().take_while(u8::is_ascii_digit).count();
            if len == 0 {
                return None;
            }
            // Seven digits is what .NET writes; more than nine cannot be held, and the extra is below a nanosecond.
            let kept = &after[..len.min(9)];
            nanos = format!("{kept:0<9}").parse().ok()?;
            rest = &after[len..];
        }
        let offset = match rest {
            "Z" | "z" => 0,
            _ => {
                let sign = match rest.chars().next()? {
                    '+' => 1,
                    '-' => -1,
                    _ => return None,
                };
                let (oh, tail) = digits(&rest[1..], 2)?;
                let (om, tail) = digits(expect(tail, ':')?, 2)?;
                if !tail.is_empty() || oh > 23 || om > 59 {
                    return None;
                }
                sign * (oh * 3600 + om * 60)
            }
        };
        let valid = (1..=9999).contains(&year)
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && hour < 24
            && minute < 60
            && second < 60;
        valid.then(|| Self { secs: days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset, nanos })
    }

    /// The time on this machine's clock; `None` if the clock is set before 1970.
    pub fn now() -> Option<Self> {
        let since = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
        Some(Self { secs: i64::try_from(since.as_secs()).ok()?, nanos: since.subsec_nanos() })
    }

    /// Whether this is .NET's `default(DateTimeOffset)`, which a document uses for "missing".
    pub fn is_default(self) -> bool {
        self.secs == DEFAULT_SECS && self.nanos == 0
    }

    /// This instant moved later by `seconds` (earlier if negative).
    pub fn plus_secs(self, seconds: i64) -> Self {
        Self { secs: self.secs.saturating_add(seconds), nanos: self.nanos }
    }
}

#[cfg(test)]
mod tests;
