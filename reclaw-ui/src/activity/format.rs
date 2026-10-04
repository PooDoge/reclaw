use std::time::Duration;

/// "21 MB", "1.4 GB": decimal units, as download sites and file managers show.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000. && unit < UNITS.len() - 1 {
        value /= 1000.;
        unit += 1;
    }
    // Two significant digits are enough for a progress line: 21 MB, 1.4 GB, 340 kB.
    if value >= 10. { format!("{value:.0} {}", UNITS[unit]) } else { format!("{value:.1} {}", UNITS[unit]) }
}

/// "7.4 MB/s".
pub fn format_rate(bytes_per_second: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_second))
}

/// "35 s left", "4 min left", "2 h 10 min left".
pub fn format_eta(left: Duration) -> String {
    let secs = left.as_secs();
    match secs {
        0..=59 => format!("{} s left", secs.max(1)),
        60..=3599 => format!("{} min left", secs.div_ceil(60)),
        _ => {
            let (hours, minutes) = (secs / 3600, (secs % 3600).div_ceil(60));
            if minutes == 60 {
                format!("{} h left", hours + 1)
            } else if minutes == 0 {
                format!("{hours} h left")
            } else {
                format!("{hours} h {minutes} min left")
            }
        }
    }
}
