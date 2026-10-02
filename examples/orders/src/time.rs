//! Just enough date handling for the example, without a date crate.
//! Every time is UTC, and says so.

use std::time::{SystemTime, UNIX_EPOCH};

/// Now, as `YYYY-MM-DDTHH:MM:SSZ`.
pub(crate) fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil(secs / 86_400);
    let t = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        t / 3600,
        t / 60 % 60,
        t % 60
    )
}

/// Today, as `YYYY-MM-DD` (UTC).
pub(crate) fn today() -> String {
    now_rfc3339()[..10].to_owned()
}

/// `2026-09-04T10:05:00Z` → `4 Sep 2026, 10:05`; anything else unchanged.
pub(crate) fn readable(rfc3339: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parse = || -> Option<String> {
        let (date, time) = rfc3339.strip_suffix('Z')?.split_once('T')?;
        let mut parts = date.split('-');
        let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
        let month = MONTHS.get(m.parse::<usize>().ok()?.checked_sub(1)?)?;
        let day: u32 = d.parse().ok()?;
        Some(format!("{day} {month} {y}, {}", time.get(..5)?))
    };
    parse().unwrap_or_else(|| rfc3339.to_owned())
}

/// Days since 1970-01-01 → (year, month, day), after Howard Hinnant's
/// `civil_from_days`.
fn civil(days: u64) -> (i64, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_become_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_723), (2024, 1, 1));
        assert_eq!(civil(20_700), (2026, 9, 4));
        assert_eq!(readable("2026-09-04T10:05:00Z"), "4 Sep 2026, 10:05");
        assert_eq!(readable("garbage"), "garbage");
        assert_eq!(now_rfc3339().len(), 20);
    }
}
