//! Looks for a newer release on GitHub: one request at startup, off the UI thread.

use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde::Deserialize;

const LATEST: &str = "https://api.github.com/repos/narusenia/listenosu/releases/latest";

pub struct Release {
    pub tag: String,
    pub url: String,
}

#[derive(Deserialize)]
struct Latest {
    tag_name: String,
    html_url: String,
}

/// The receiver yields a release only when it is newer than `current`. A failed check
/// (offline, rate limited, no release yet) ends silently: it is not worth interrupting anyone.
pub fn spawn_check(current: &'static str, ctx: eframe::egui::Context) -> Receiver<Release> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        if let Some(release) = fetch().filter(|r| is_newer(&r.tag, current)) {
            let _ = tx.send(release);
            ctx.request_repaint();
        }
    });
    rx
}

fn fetch() -> Option<Release> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let body = agent
        .get(LATEST)
        .header(
            "User-Agent",
            concat!("listenosu/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    let latest: Latest = serde_json::from_str(&body).ok()?;
    Some(Release {
        tag: latest.tag_name,
        url: latest.html_url,
    })
}

/// Compares `v1.2.3` against `1.2.3` by number. A pre-release suffix is ignored.
fn is_newer(tag: &str, current: &str) -> bool {
    fn parts(version: &str) -> Vec<u64> {
        version
            .trim_start_matches('v')
            .split('.')
            .map(|part| {
                let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().unwrap_or(0)
            })
            .collect()
    }
    parts(tag) > parts(current)
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions_by_number() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("v0.10.0", "0.9.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.1.0-rc1", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
    }
}
