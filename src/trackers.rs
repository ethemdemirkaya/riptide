//! Public trackers added to every torrent when "extra trackers" is on.
//!
//! Many magnet links carry one tracker or none, so finding metadata and peers
//! relies on DHT alone, which can take a minute. Announcing to a handful of
//! large, long-running open trackers usually finds peers within seconds.
//! The list favors trackers with years of uptime; dead ones just time out.

pub const PUBLIC: &[&str] = &[
    "udp://tracker.opentrackr.org:1337/announce",
    "udp://open.demonii.com:1337/announce",
    "udp://open.stealth.si:80/announce",
    "udp://tracker.torrent.eu.org:451/announce",
    "udp://exodus.desync.com:6969/announce",
    "udp://tracker.theoks.net:6969/announce",
    "udp://explodie.org:6969/announce",
    "udp://tracker.qu.ax:6969/announce",
    "udp://opentracker.io:6969/announce",
    "udp://tracker.dler.org:6969/announce",
    "https://tracker.tamersunion.org:443/announce",
    "http://tracker.opentrackr.org:1337/announce",
];

#[cfg(test)]
mod tests {
    #[test]
    fn all_trackers_parse() {
        for t in super::PUBLIC {
            url::Url::parse(t).unwrap();
        }
    }
}
