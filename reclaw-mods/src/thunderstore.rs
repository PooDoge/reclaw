//! Thunderstore's answers, read. Pure: addresses are built and bodies parsed here; asking is `client`'s job.
//!
//! The shapes are the ones Quiver's working client reads (read 2026-10-06; Thunderstore itself could not be reached from where this
//! was written): the community's listing at `/api/cyberstorm/listing/<community>/` (twenty packages a page, a `next` address), its
//! sections at `/api/cyberstorm/community/<community>/filters/` (the "Mods" section leaves out modpacks and tools), and one
//! package's newest version, with its download address and dependencies, at `/api/experimental/package/<owner>/<name>/`.
use serde_json::Value;

use crate::{
    package::{Download, Package, Page, Sort, encode, flag, number, text},
    source::Provider,
};

/// Files Thunderstore requires at the top of every package. They describe the package; they are not the mod.
pub const METADATA_FILES: [&str; 4] = ["manifest.json", "icon.png", "README.md", "CHANGELOG.md"];

pub fn listing_url(base: &str, community: &str, page: u32, sort: Sort, section: Option<&str>) -> String {
    let mut url = format!(
        "{}/api/cyberstorm/listing/{}/?page={}&ordering={}&nsfw=false&deprecated=false",
        base.trim_end_matches('/'),
        encode(community),
        page.max(1),
        sort.thunderstore()
    );
    if let Some(section) = section.filter(|s| !s.trim().is_empty()) {
        url.push_str(&format!("&section={}", encode(section.trim())));
    }
    url
}

pub fn filters_url(base: &str, community: &str) -> String {
    format!("{}/api/cyberstorm/community/{}/filters/", base.trim_end_matches('/'), encode(community))
}

pub fn package_url(base: &str, owner: &str, name: &str) -> String {
    format!("{}/api/experimental/package/{}/{}/", base.trim_end_matches('/'), encode(owner), encode(name))
}

/// The page a person opens for a package, inside its community.
pub fn package_page(community: &str, owner: &str, name: &str) -> String {
    format!("https://thunderstore.io/c/{}/p/{}/{}/", encode(community), encode(owner), encode(name))
}

/// The id of the community's "Mods" section, when it has one.
pub fn parse_mods_section(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    value.get("sections")?.as_array()?.iter().find_map(|section| {
        let named = |key: &str| text(section.get(key)).is_some_and(|t| t.eq_ignore_ascii_case("mods"));
        (named("slug") || named("name")).then(|| text(section.get("uuid"))).flatten()
    })
}

/// A listing page. Packages without a name or namespace are left out; anything else odd is tolerated.
pub fn parse_listing(body: &[u8], community: &str) -> Result<Page, String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| format!("Thunderstore's listing is not JSON: {e}"))?;
    let results = value.get("results").and_then(Value::as_array).ok_or("Thunderstore's listing has no results")?;
    let packages = results.iter().filter_map(|item| listed_package(item, community)).collect();
    Ok(Page { packages, next: value.get("next").and_then(Value::as_str).and_then(next_page), total: number(value.get("count")) })
}

fn listed_package(item: &Value, community: &str) -> Option<Package> {
    let (owner, name) = (text(item.get("namespace"))?, text(item.get("name"))?);
    let full_name = format!("{owner}-{name}");
    let icon_url = text(item.get("icon_url"));
    let version = icon_url.as_deref().and_then(|icon| version_from_icon(icon, &owner, &name)).unwrap_or_default();
    Some(Package {
        provider: Provider::Thunderstore,
        source_key: community.to_string(),
        id: full_name.clone(),
        page_url: Some(package_page(community, &owner, &name)),
        full_name,
        summary: text(item.get("description")).unwrap_or_default(),
        icon_url,
        deprecated: flag(item.get("is_deprecated")),
        nsfw: flag(item.get("is_nsfw")),
        downloads: number(item.get("download_count")).unwrap_or(0),
        rating: number(item.get("rating_count")).unwrap_or(0),
        version,
        size: number(item.get("size")),
        owner,
        name,
    })
}

/// The `page` number in a `next` address.
pub fn next_page(next: &str) -> Option<u32> {
    let query = next.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        key.eq_ignore_ascii_case("page").then(|| value.parse().ok()).flatten().filter(|n| *n > 0)
    })
}

/// Icons are named `<Owner>-<Name>-<version>.png`: the listing's only hint of the version.
pub fn version_from_icon(icon: &str, owner: &str, name: &str) -> Option<String> {
    let file = icon.rsplit('/').next()?;
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let prefix = format!("{owner}-{name}-");
    let rest = stem.get(..prefix.len()).filter(|p| p.eq_ignore_ascii_case(&prefix)).map(|_| &stem[prefix.len()..])?;
    let rest = rest.trim();
    (!rest.is_empty()).then(|| rest.to_string())
}

/// One package's newest version: what to download, and the package as the experimental API describes it.
pub fn parse_package(body: &[u8], community: &str) -> Result<(Package, Download), String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| format!("Thunderstore's package answer is not JSON: {e}"))?;
    let owner = text(value.get("namespace")).or_else(|| text(value.get("owner"))).ok_or("Thunderstore's package has no owner")?;
    let name = text(value.get("name")).ok_or("Thunderstore's package has no name")?;
    let latest = value.get("latest").ok_or("Thunderstore's package has no version")?;
    let version = text(latest.get("version_number")).ok_or("Thunderstore's package version has no number")?;
    let url = text(latest.get("download_url")).ok_or("Thunderstore's package version has no download address")?;
    let dependencies = latest
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|deps| deps.iter().filter_map(|d| text(Some(d))).collect())
        .unwrap_or_default();
    let full_name = text(value.get("full_name")).unwrap_or_else(|| format!("{owner}-{name}"));
    let package = Package {
        provider: Provider::Thunderstore,
        source_key: community.to_string(),
        id: format!("{owner}-{name}"),
        page_url: Some(package_page(community, &owner, &name)),
        full_name,
        summary: text(latest.get("description")).unwrap_or_default(),
        icon_url: text(latest.get("icon")),
        deprecated: flag(value.get("is_deprecated")),
        nsfw: false,
        downloads: number(latest.get("downloads")).unwrap_or(0),
        rating: 0,
        version: version.clone(),
        size: None,
        owner,
        name,
    };
    // Thunderstore serves every package as a zip from an address that ends in its version, which says nothing about the format.
    let file_name = Some(format!("{}-{version}.zip", package.id));
    Ok((package, Download { url, version, size: None, file_name, file_id: None, dependencies }))
}

/// `Owner-Name-1.2.3` into `Owner-Name` and the version; `Owner-Name` alone has none.
pub fn parse_dependency(dependency: &str) -> Option<(String, Option<String>)> {
    let parts: Vec<&str> = dependency.trim().split('-').filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        [] | [_] => None,
        [.., last] if parts.len() >= 3 && looks_like_version(last) => Some((parts[..parts.len() - 1].join("-"), Some(last.to_string()))),
        _ => Some((dependency.trim().to_string(), None)),
    }
}

/// `Owner-Name` at its first hyphen.
pub fn split_full_name(full_name: &str) -> Option<(&str, &str)> {
    let (owner, name) = full_name.split_once('-')?;
    (!owner.trim().is_empty() && !name.trim().is_empty()).then_some((owner, name))
}

fn looks_like_version(text: &str) -> bool {
    let segments: Vec<&str> = text.split('.').collect();
    segments.len() >= 2 && segments.iter().all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests;
