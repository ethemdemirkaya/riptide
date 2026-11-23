//! Human-readable formatting for sizes, speeds and durations.

const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

/// Formats a byte count using binary multiples, e.g. `1.4 GB`.
pub fn bytes(n: u64) -> String {
    if n < 1024 {
        return format!("{n} B");
    }
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    let precision = if value >= 100.0 {
        0
    } else if value >= 10.0 {
        1
    } else {
        2
    };
    format!("{value:.precision$} {}", UNITS[unit])
}

/// Formats a transfer rate. Returns an empty string for a stalled transfer so
/// tables stay quiet instead of filling with `0 B/s`.
pub fn speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return String::new();
    }
    format!("{}/s", bytes(bytes_per_sec))
}

/// Unit suffixes for [`eta`]: seconds, minutes, hours, days.
pub type EtaUnits = [&'static str; 4];

/// Formats a remaining duration compactly: `45s`, `12m`, `3h 05m`, `2d 4h`.
pub fn eta(secs: u64, [s, m, h, d]: EtaUnits) -> String {
    const MIN: u64 = 60;
    const HOUR: u64 = 60 * MIN;
    const DAY: u64 = 24 * HOUR;
    match secs {
        0..MIN => format!("{secs}{s}"),
        MIN..HOUR => format!("{}{m}", secs / MIN),
        HOUR..DAY => format!("{}{h} {:02}{m}", secs / HOUR, (secs % HOUR) / MIN),
        _ if secs >= 100 * DAY => "∞".to_owned(),
        _ => format!("{}{d} {}{h}", secs / DAY, (secs % DAY) / HOUR),
    }
}

/// Upload/download ratio with two decimals.
pub fn ratio(uploaded: u64, downloaded: u64) -> String {
    if downloaded == 0 {
        return "—".to_owned();
    }
    format!("{:.2}", uploaded as f64 / downloaded as f64)
}

/// Percent-encodes a string for use in a URL query component.
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1023), "1023 B");
        assert_eq!(bytes(1024), "1.00 KB");
        assert_eq!(bytes(1536), "1.50 KB");
        assert_eq!(bytes(15 * 1024 * 1024), "15.0 MB");
        assert_eq!(bytes(700 * 1024 * 1024), "700 MB");
        assert_eq!(bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn formats_speed() {
        assert_eq!(speed(0), "");
        assert_eq!(speed(2048), "2.00 KB/s");
    }

    #[test]
    fn formats_eta() {
        const EN: EtaUnits = ["s", "m", "h", "d"];
        assert_eq!(eta(5, EN), "5s");
        assert_eq!(eta(125, EN), "2m");
        assert_eq!(eta(3 * 3600 + 5 * 60, EN), "3h 05m");
        assert_eq!(eta(2 * 86400 + 4 * 3600, EN), "2d 4h");
        assert_eq!(eta(200 * 86400, EN), "∞");
        assert_eq!(
            eta(3 * 3600 + 5 * 60, [" sn", " dk", " sa", " g"]),
            "3 sa 05 dk"
        );
    }

    #[test]
    fn formats_ratio() {
        assert_eq!(ratio(10, 0), "—");
        assert_eq!(ratio(150, 100), "1.50");
    }

    #[test]
    fn encodes_url() {
        assert_eq!(url_encode("Big Buck Bunny"), "Big%20Buck%20Bunny");
        assert_eq!(url_encode("a&b=ç"), "a%26b%3D%C3%A7");
    }
}
