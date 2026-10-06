//! GameBanana's answers, read. Pure: addresses are built and bodies parsed here; asking is `client`'s job.
//!
//! The shapes are the ones Quiver's working client reads (read 2026-10-06; GameBanana itself could not be reached from where this
//! was written): a game's mods at `/apiv13/Mod/Index` filtered by `Generic_Game`, and one mod's files at `/apiv11/Mod/<id>`.
//! Numbers sometimes arrive as strings, and the preview picture has been seen under two names; both are read.
use serde_json::Value;

use crate::{
    package::{Download, Package, Page, Sort, encode, flag, number, text},
    source::Provider,
};

/// Mods asked for per page (the site's largest).
pub const PER_PAGE: u32 = 50;

pub fn index_url(base: &str, game: &str, page: u32, sort: Sort) -> String {
    format!(
        "{}/apiv13/Mod/Index?_nPerpage={PER_PAGE}&_aFilters%5BGeneric_Game%5D={}&_nPage={}&_sSort={}",
        base.trim_end_matches('/'),
        encode(game),
        page.max(1),
        sort.gamebanana()
    )
}

pub fn detail_url(base: &str, id: &str) -> String {
    format!(
        "{}/apiv11/Mod/{}?_csvProperties=_idRow,_sName,_sText,_sDescription,_sVersion,_aFiles,_sProfileUrl,_aSubmitter,_bIsObsolete",
        base.trim_end_matches('/'),
        encode(id)
    )
}

/// One page of a game's mods. Records that are not free mods (a paid one, a tool, a question) are left out.
pub fn parse_index(body: &[u8], game: &str, page: u32) -> Result<Page, String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| format!("GameBanana's mod list is not JSON: {e}"))?;
    let records = value.get("_aRecords").and_then(Value::as_array).ok_or("GameBanana's mod list has no records")?;
    let metadata = value.get("_aMetadata");
    let complete = flag(metadata.and_then(|m| m.get("_bIsComplete"))) || records.is_empty();
    Ok(Page {
        packages: records.iter().filter_map(|r| listed_mod(r, game)).collect(),
        next: (!complete).then_some(page.max(1) + 1),
        total: number(metadata.and_then(|m| m.get("_nRecordCount"))),
    })
}

fn listed_mod(record: &Value, game: &str) -> Option<Package> {
    let id = number(record.get("_idRow")).filter(|n| *n > 0)?.to_string();
    let name = text(record.get("_sName"))?;
    if text(record.get("_sModelName")).is_some_and(|m| !m.eq_ignore_ascii_case("mod"))
        || text(record.get("_sPayType")).is_some_and(|p| !p.eq_ignore_ascii_case("free"))
    {
        return None;
    }
    let owner = text(record.get("_aSubmitter").and_then(|s| s.get("_sName"))).unwrap_or_default();
    let nsfw = flag(record.get("_bHasContentRatings"));
    Some(Package {
        provider: Provider::GameBanana,
        source_key: game.to_string(),
        full_name: if owner.is_empty() { name.clone() } else { format!("{owner}-{name}") },
        page_url: Some(text(record.get("_sProfileUrl")).unwrap_or_else(|| format!("https://gamebanana.com/mods/{id}"))),
        summary: text(record.get("_sDescription")).unwrap_or_default(),
        icon_url: preview(record, nsfw),
        deprecated: flag(record.get("_bIsObsolete")),
        nsfw,
        downloads: number(record.get("_nDownloadCount")).unwrap_or(0),
        rating: number(record.get("_nLikeCount")).unwrap_or(0),
        version: text(record.get("_sVersion")).unwrap_or_default(),
        size: None,
        id,
        owner,
        name,
    })
}

/// The preview picture: the first image of `_aPreviewMedia`, or `_aPreviewContent.screenshot`. A mod with a content rating gets
/// its safe-for-work crop when there is one.
fn preview(record: &Value, nsfw: bool) -> Option<String> {
    let shot = record
        .get("_aPreviewMedia")
        .and_then(|m| m.get("_aImages"))
        .and_then(Value::as_array)
        .and_then(|images| images.first())
        .or_else(|| record.get("_aPreviewContent").and_then(|c| c.get("screenshot")))?;
    let base = text(shot.get("_sBaseUrl"))?;
    let pick = |keys: &[&str]| keys.iter().find_map(|k| text(shot.get(*k)));
    let file = if nsfw {
        pick(&["_sFile530Sfw", "_sFile220Sfw", "_sFile530", "_sFile220", "_sFile"])
    } else {
        pick(&["_sFile530", "_sFile220", "_sFile", "_sFile530Sfw", "_sFile220Sfw"])
    }?;
    Some(format!("{}/{}", base.trim_end_matches('/'), file.trim_start_matches('/')))
}

/// One downloadable file of a mod.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct File {
    pub id: String,
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
    pub version: String,
}

/// A mod's own page: its version and its files that are still offered.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Detail {
    pub version: String,
    pub files: Vec<File>,
}

pub fn parse_detail(body: &[u8]) -> Result<Detail, String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| format!("GameBanana's mod page is not JSON: {e}"))?;
    if !value.is_object() {
        return Err("GameBanana's mod page is not an object".to_string());
    }
    let files = value
        .get("_aFiles")
        .and_then(Value::as_array)
        .map(|files| {
            files
                .iter()
                .filter(|f| !flag(f.get("_bIsArchived")))
                .filter_map(|f| {
                    let id = number(f.get("_idRow")).filter(|n| *n > 0)?.to_string();
                    Some(File {
                        name: text(f.get("_sFile")).unwrap_or_else(|| format!("file-{id}")),
                        url: text(f.get("_sDownloadUrl"))?,
                        size: number(f.get("_nFilesize")),
                        version: text(f.get("_sVersion")).unwrap_or_default(),
                        id,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Detail { version: text(value.get("_sVersion")).unwrap_or_default(), files })
}

/// Whether a file name says it is an archive Reclaw unpacks.
fn is_archive_name(name: &str) -> bool {
    reclaw_install::Format::of_name(name).is_archive()
}

/// The file an install takes: the first archive still offered, else the first file. `None` when the mod offers nothing.
/// (Several files usually mean variants of one mod; choosing between them is not built yet.)
pub fn choose_download(detail: &Detail) -> Option<Download> {
    let file = detail.files.iter().find(|f| is_archive_name(&f.name)).or_else(|| detail.files.first())?;
    let version = if file.version.is_empty() { detail.version.clone() } else { file.version.clone() };
    Some(Download {
        url: file.url.clone(),
        version: if version.is_empty() { "0".to_string() } else { version },
        size: file.size,
        file_name: Some(file.name.clone()),
        file_id: Some(file.id.clone()),
        dependencies: Vec::new(),
    })
}

#[cfg(test)]
mod tests;
