//! How the site's data is worded on a game page, ported from Quiver 3.5 (`BrowseText`, `BrowseDetailsViewModel.ToLine`,
//! `FillAbout`, `ShowReleases`, `ReleaseWarnings`) so a game reads the same in both launchers. Times are JavaScript milliseconds;
//! `now` is passed in, so every sentence is tested.
use reclaw_catalog::site::{AiLevel, Checking, HistoryRelease, ReleaseState, Review, RunResult, Scan, SiteApp, SiteProject, Verdict};

/// How a verdict is coloured, as the website colours it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Positive,
    Caution,
    Negative,
    Muted,
}

/// What players said about how an app runs: one verdict, and the counts behind it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rating {
    pub label: &'static str,
    /// " · 4 players", or " · 2 run well, 1 with issues"; empty when nobody said.
    pub counts: String,
    pub tone: Tone,
    /// How many players said anything.
    pub players: u32,
}

impl Rating {
    /// Always one verdict, as ProtonDB always shows one tier: what most players said, and on a tie the more careful of the answers
    /// tied (the counts after it show the split).
    pub fn new(runs: u32, issues: u32, broken: u32) -> Self {
        let players = runs + issues + broken;
        if players == 0 {
            return Self { label: "Not rated yet", counts: String::new(), tone: Tone::Muted, players };
        }
        let most = runs.max(issues).max(broken);
        let (label, tone) = if broken == most {
            ("Doesn't run", Tone::Negative)
        } else if issues == most {
            ("Has issues", Tone::Caution)
        } else {
            ("Runs well", Tone::Positive)
        };
        // Everyone agrees: "Runs well · 4 players", rather than repeating the verdict.
        if [runs, issues, broken].iter().filter(|n| **n > 0).count() == 1 {
            return Self { label, counts: format!(" · {players} {}", plural(players, "player", "players")), tone, players };
        }
        let mut said = Vec::new();
        if runs > 0 {
            said.push(format!("{runs} {} well", plural(runs, "runs", "run")));
        }
        if issues > 0 {
            said.push(format!("{issues} with issues"));
        }
        if broken > 0 {
            said.push(format!("{broken} {} run", plural(broken, "doesn't", "don't")));
        }
        Self { label, counts: format!(" · {}", said.join(", ")), tone, players }
    }

    pub fn of(app: &SiteApp) -> Self {
        Self::new(app.recommended, app.report_issues, app.report_broken)
    }

    /// The verdict and its counts in one line.
    pub fn line(&self) -> String {
        format!("{}{}", self.label, self.counts)
    }
}

fn plural(n: u32, one: &'static str, many: &'static str) -> &'static str {
    if n == 1 { one } else { many }
}

/// The line under a rating: who to thank, or an invitation.
pub fn share_title(players: u32) -> &'static str {
    if players == 0 { "Nobody has shared how it runs yet" } else { "Your experience helps the next person" }
}

const DAY_MS: f64 = 86_400_000.;

/// "Updated 3 d ago", "No release in 2 yr+", "No releases".
pub fn release_age(released_at: Option<f64>, now: f64) -> String {
    let Some(at) = released_at.filter(|a| *a > 0.) else { return "No releases".into() };
    let days = ((now - at) / DAY_MS).max(0.);
    if days >= 365. {
        return format!("No release in {} yr+", (days / 365.) as u32);
    }
    let hours = days * 24.;
    let ago = if days >= 30. {
        format!("{} mo ago", (days / 30.) as u32)
    } else if days >= 1. {
        format!("{} d ago", days as u32)
    } else if hours >= 1. {
        format!("{} h ago", hours as u32)
    } else {
        "just now".into()
    };
    format!("Updated {ago}")
}

/// Platforms by name, in the website's icon order.
pub fn platform_names(ids: &[String]) -> String {
    const ORDER: [(&str, &str); 5] =
        [("windows", "Windows"), ("macos", "macOS"), ("linux", "Linux"), ("android", "Android"), ("ios", "iOS")];
    ORDER.iter().filter(|(id, _)| ids.iter().any(|i| i.eq_ignore_ascii_case(id))).map(|(_, name)| *name).collect::<Vec<_>>().join(", ")
}

pub fn ai_label(level: AiLevel) -> &'static str {
    match level {
        AiLevel::None => "No AI use found",
        AiLevel::Assisted => "AI-assisted",
        AiLevel::Generated => "Mostly AI-generated",
    }
}

/// The chip on an app that AI helped write; nothing for one it did not.
pub fn ai_chip(level: AiLevel) -> Option<&'static str> {
    match level {
        AiLevel::None => None,
        AiLevel::Assisted => Some("AI-ASSISTED"),
        AiLevel::Generated => Some("MOSTLY AI"),
    }
}

pub fn project_type_name(id: &str) -> &'static str {
    match id {
        "port" => "Port",
        "tool" => "Tool",
        "emulator" => "Emulator",
        "game" => "Standalone game",
        _ => "",
    }
}

fn text(t: Option<&str>) -> Option<&str> {
    t.map(str::trim).filter(|t| !t.is_empty())
}

/// The project details beside a game, as Quiver 3.5 lists them: who made it, where it runs, its latest and verified releases, AI.
pub fn facts(app: &SiteApp, project: Option<&SiteProject>, now: f64) -> Vec<(&'static str, String)> {
    let mut rows = Vec::new();
    if let Some(maker) = text(project.and_then(|p| p.author.as_deref())).or(text(app.developer.as_ref().map(|d| d.name.as_str()))) {
        rows.push(("Made by", maker.to_string()));
    }
    let platforms = platform_names(&app.supported_os);
    rows.push(("Platforms", if platforms.is_empty() { "Not confirmed yet".to_string() } else { platforms }));
    rows.push((
        "Latest release",
        match app.last_release_at {
            None => "No releases found".to_string(),
            Some(at) => [text(app.last_release_version.as_deref()).unwrap_or_default().to_string(), release_age(Some(at), now)]
                .into_iter()
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" · "),
        },
    ));
    rows.push(("Verified", text(app.verified.as_ref().map(|v| v.version.as_str())).map_or_else(|| "None yet".to_string(), str::to_string)));
    rows.push(("AI use", ai_label(app.ai_level).to_string()));
    rows
}

/// What "verified" means, for the badge's hover text and the Releases section.
pub const VERIFIED_MEANS: &str = "Verified means Quiver checked this release: it comes from the app's usual developer, its files \
haven't been swapped since, and it sat through a 48-hour wait with its files scanned by VirusTotal.";

/// The note above the releases. Reclaw shows what the site says; it does not yet hold back the releases the site has not verified,
/// as Quiver Launcher does, and the note says so rather than promising it.
pub const RELEASES_NOTE: &str = "Quiver Launcher only updates to verified releases. Reclaw shows what quiverlauncher.com says about \
each release; it does not hold back unverified ones yet.";

/// The date a review was written, as "4 Oct 2026" (in UTC: the site gives no time zone, and a day either way does not matter here).
pub fn day(ms: f64) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    // Days since 1970 to a civil date (Howard Hinnant's algorithm), so no date crate is needed for one line.
    let z = (ms / DAY_MS).floor() as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{d} {} {y}", MONTHS[(m - 1) as usize])
}

/// One player's review, laid out like the website's: who, when and where, how it ran, what they said.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReviewLine {
    pub author: String,
    /// "4 Oct 2026 · Linux · tested on v1.2".
    pub meta: String,
    pub result: &'static str,
    pub tone: Tone,
    pub body: String,
}

impl ReviewLine {
    pub fn new(review: &Review) -> Self {
        let mut meta = Vec::new();
        if review.created_at > 0. {
            meta.push(day(review.created_at));
        }
        if let Some(platform) = text(review.platform.as_deref()) {
            let name = platform_names(&[platform.to_string()]);
            meta.push(if name.is_empty() { platform.to_string() } else { name });
        }
        if let Some(version) = text(review.version.as_deref()) {
            meta.push(format!("tested on {version}"));
        }
        let (result, tone) = match review.result {
            RunResult::Runs => ("Runs well", Tone::Positive),
            RunResult::Issues => ("Runs with issues", Tone::Caution),
            RunResult::Broken => ("Doesn't run", Tone::Negative),
            RunResult::Other => ("", Tone::Muted),
        };
        Self {
            author: text(Some(&review.author)).unwrap_or("A player").to_string(),
            meta: meta.join(" · "),
            result,
            tone,
            body: review.body.trim().to_string(),
        }
    }
}

/// VirusTotal's verdict, when it is worth the space: only a detection is something the player must weigh.
pub fn scan_line(scan: Option<&Scan>) -> Option<String> {
    let scan = scan?;
    match scan.verdict {
        Verdict::Warning => Some(format!(
            "VirusTotal: {} flag one of its files (often a false alarm).",
            text(scan.engines.as_deref()).unwrap_or("an engine")
        )),
        Verdict::Flagged => Some(format!(
            "VirusTotal: {} flag one of its files. Your antivirus may block or remove it.",
            text(scan.engines.as_deref()).unwrap_or("several engines")
        )),
        _ => None,
    }
}

/// When a wait ends, as "in about 5 hours"; nothing once it is over.
fn wait_left(ends_at: Option<f64>, now: f64) -> Option<String> {
    let left = ends_at? - now;
    (left > 0.).then(|| {
        let hours = ((left / 3_600_000.).ceil() as u32).max(1);
        format!("It should be verified in about {hours} {}.", plural(hours, "hour", "hours"))
    })
}

/// One release as the site judges it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReleaseLine {
    pub version: String,
    pub state: &'static str,
    pub tone: Tone,
    pub prerelease: bool,
    /// "Updated 3 d ago" style, or empty.
    pub when: String,
    /// Why it is not verified or is blocked, the VirusTotal verdict when it matters, and when the wait ends.
    pub reasons: Vec<String>,
}

impl ReleaseLine {
    pub fn new(release: &HistoryRelease, now: f64) -> Self {
        let (state, tone) = match release.state {
            ReleaseState::Verified => ("Verified", Tone::Positive),
            ReleaseState::Unverified => ("Unverified", Tone::Caution),
            ReleaseState::Blocked => ("Blocked", Tone::Negative),
        };
        let mut reasons: Vec<String> = release.reasons.iter().map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect();
        reasons.extend(scan_line(release.scan.as_ref()));
        if release.state == ReleaseState::Unverified {
            reasons.extend(wait_left(release.check_ends_at, now));
        }
        Self {
            version: release.version.clone(),
            state,
            tone,
            prerelease: release.prerelease,
            when: release.released_at.map(day).unwrap_or_default(),
            reasons,
        }
    }
}

/// A newer release the site is still checking: "v0.3.56 is being checked. Waiting 48 hours. It should be verified in about 5 hours."
pub fn checking_line(checking: &Checking, now: f64) -> Option<String> {
    let version = text(Some(&checking.version))?;
    let mut parts = vec![format!("{version} is being checked.")];
    parts.extend(
        checking
            .reasons
            .iter()
            .map(|r| r.trim())
            .filter(|r| !r.is_empty())
            .map(|r| if r.ends_with('.') { r.to_string() } else { format!("{r}.") }),
    );
    if checking.needs_review {
        parts.push("A maintainer has to look at it first.".into());
    } else {
        parts.extend(wait_left(checking.check_ends_at, now));
    }
    Some(parts.join(" "))
}
