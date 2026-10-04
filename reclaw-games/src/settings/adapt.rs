use super::{
    resolve::SettingSpec,
    value::{SettingValue, Size, ValueKind},
};

/// Fit `value` into `spec`, or `None` if it cannot be made to fit. Never invents a different choice:
/// a value that is wrong for a game is skipped, not replaced (the oxide-code rule: out-of-range
/// input is ignored, not silently changed into something the user did not pick) except where the
/// docs/specs/game-settings.md table says to clamp or pick the nearest.
pub fn adapt(spec: &SettingSpec, value: &SettingValue) -> Option<SettingValue> {
    match (&spec.kind, value) {
        (ValueKind::Bool, SettingValue::Bool(_)) => Some(value.clone()),
        (ValueKind::Int { min, max, step, .. }, SettingValue::Int(v)) => snap(*min, *max, *step, *v).map(SettingValue::Int),
        (ValueKind::Choice(options), SettingValue::Choice(id)) => options.iter().any(|o| &o.id == id).then(|| value.clone()),
        (ValueKind::Size { options, .. }, SettingValue::Size(size)) => fit_size(options, *size).map(SettingValue::Size),
        (ValueKind::Size { native: true, .. }, SettingValue::Native) => Some(SettingValue::Native),
        _ => None,
    }
}

/// Clamp into `min..=max`, then move to the nearest `min + n*step` (halves round up).
fn snap(min: i64, max: i64, step: i64, value: i64) -> Option<i64> {
    if min > max {
        return None;
    }
    let step = i128::from(step.max(1));
    let (min, max) = (i128::from(min), i128::from(max));
    let offset = i128::from(value).clamp(min, max) - min;
    let mut snapped = min + (offset + step / 2) / step * step;
    if snapped > max {
        // The top of the range is not on the grid; the grid point below it is the nearest allowed.
        snapped -= step;
    }
    i64::try_from(snapped).ok()
}

/// `size` itself when listed, else the listed size with the largest area that is not above it.
fn fit_size(options: &[Size], size: Size) -> Option<Size> {
    if options.contains(&size) {
        return Some(size);
    }
    // Strictly greater keeps the first of equal areas, so the answer follows the list's order.
    options.iter().filter(|o| o.area() <= size.area()).fold(None, |best: Option<Size>, o| match best {
        Some(b) if b.area() >= o.area() => Some(b),
        _ => Some(*o),
    })
}

#[cfg(test)]
mod tests;
