#![allow(dead_code)]

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct EpubSpineItem {
    pub id: String,
    pub href: String,
    pub full_path: String,
    pub media_type: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct EpubMetadata {
    pub title: String,
    pub creator: Option<String>,
    pub series: Option<String>,
    pub volume: Option<String>,
    pub summary: Option<String>,
    pub language: Option<String>,
    pub opf_path: String,
    pub spine_items: Vec<EpubSpineItem>,
    pub cover_image_path: Option<String>,
}

pub fn parse_epub(file_path: &Path) -> AppResult<EpubMetadata> {
    let file = File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // 1. Read container.xml to locate the OPF package document
    let opf_path = {
        let mut container_entry = archive.by_name("META-INF/container.xml").map_err(|e| {
            AppError::BadRequest(format!("Invalid EPUB: missing container.xml: {}", e))
        })?;

        let mut container_xml = String::new();
        container_entry.read_to_string(&mut container_xml)?;

        let doc = roxmltree::Document::parse(&container_xml)
            .map_err(|e| AppError::BadRequest(format!("XML error in container.xml: {}", e)))?;

        let rootfile = doc
            .descendants()
            .find(|n| n.has_tag_name("rootfile"))
            .ok_or_else(|| {
                AppError::BadRequest("No rootfile element in container.xml".to_string())
            })?;

        rootfile
            .attribute("full-path")
            .ok_or_else(|| AppError::BadRequest("No full-path in rootfile element".to_string()))?
            .to_string()
    };

    let opf_dir = Path::new(&opf_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // 2. Read OPF file
    let opf_xml = {
        let mut opf_entry = archive.by_name(&opf_path).map_err(|e| {
            AppError::BadRequest(format!("OPF package file {} not found: {}", opf_path, e))
        })?;

        let mut content = String::new();
        opf_entry.read_to_string(&mut content)?;
        content
    };

    let opf_doc = roxmltree::Document::parse(&opf_xml)
        .map_err(|e| AppError::BadRequest(format!("XML error in OPF file: {}", e)))?;

    // 3. Parse Metadata
    let mut title = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();
    let mut creator = None;
    let mut summary = None;
    let mut language = None;
    let mut series = None;
    let mut volume = None;
    let mut cover_id_from_meta = None;

    if let Some(metadata) = opf_doc.descendants().find(|n| n.has_tag_name("metadata")) {
        for child in metadata.children() {
            if !child.is_element() {
                continue;
            }
            let tag = child.tag_name().name();
            let text = child.text().map(|s| s.trim().to_string());

            if tag.eq_ignore_ascii_case("title") {
                if let Some(t) = text {
                    if !t.is_empty() {
                        title = t;
                    }
                }
            } else if tag.eq_ignore_ascii_case("creator") {
                creator = text;
            } else if tag.eq_ignore_ascii_case("description") {
                summary = text;
            } else if tag.eq_ignore_ascii_case("language") {
                language = text;
            } else if tag.eq_ignore_ascii_case("meta") {
                let prop = child.attribute("property").unwrap_or_default();
                let name = child.attribute("name").unwrap_or_default();
                let content = child.attribute("content").map(|s| s.to_string());

                if prop == "belongs-to-collection" {
                    series = text.or(content);
                } else if prop == "group-position" {
                    volume = text.or(content);
                } else if name == "calibre:series" {
                    series = content.or(text);
                } else if name == "calibre:series_index" {
                    volume = content.or(text);
                } else if name == "cover" {
                    cover_id_from_meta = content.or(text);
                }
            }
        }
    }

    // 4. Parse Manifest
    struct ManifestItem {
        href: String,
        media_type: String,
        properties: String,
    }

    let mut manifest: HashMap<String, ManifestItem> = HashMap::new();
    if let Some(manifest_node) = opf_doc.descendants().find(|n| n.has_tag_name("manifest")) {
        for child in manifest_node.children() {
            if child.has_tag_name("item") {
                if let (Some(id), Some(href)) = (child.attribute("id"), child.attribute("href")) {
                    let media_type = child
                        .attribute("media-type")
                        .unwrap_or_default()
                        .to_string();
                    let properties = child
                        .attribute("properties")
                        .unwrap_or_default()
                        .to_string();
                    manifest.insert(
                        id.to_string(),
                        ManifestItem {
                            href: href.to_string(),
                            media_type,
                            properties,
                        },
                    );
                }
            }
        }
    }

    // 5. Parse Spine
    let mut spine_items = Vec::new();
    if let Some(spine_node) = opf_doc.descendants().find(|n| n.has_tag_name("spine")) {
        for child in spine_node.children() {
            if child.has_tag_name("itemref") {
                if let Some(idref) = child.attribute("idref") {
                    if let Some(item) = manifest.get(idref) {
                        let full_path = resolve_relative_path(&opf_dir, &item.href);
                        let title = format!("Chapter {}", spine_items.len() + 1);
                        spine_items.push(EpubSpineItem {
                            id: idref.to_string(),
                            href: item.href.clone(),
                            full_path,
                            media_type: item.media_type.clone(),
                            title,
                        });
                    }
                }
            }
        }
    }

    // 6. Cover image discovery
    let mut cover_image_path = None;

    // Check cover id from meta
    if let Some(cover_id) = cover_id_from_meta {
        if let Some(item) = manifest.get(&cover_id) {
            cover_image_path = Some(resolve_relative_path(&opf_dir, &item.href));
        }
    }

    // Check properties="cover-image"
    if cover_image_path.is_none() {
        for item in manifest.values() {
            if item.properties.contains("cover-image") {
                cover_image_path = Some(resolve_relative_path(&opf_dir, &item.href));
                break;
            }
        }
    }

    // Fallback: search for item with id or href containing 'cover'
    if cover_image_path.is_none() {
        for (id, item) in &manifest {
            let id_low = id.to_lowercase();
            let href_low = item.href.to_lowercase();
            if (id_low == "cover" || id_low.contains("cover-image") || href_low.contains("cover"))
                && item.media_type.starts_with("image/")
            {
                cover_image_path = Some(resolve_relative_path(&opf_dir, &item.href));
                break;
            }
        }
    }

    Ok(EpubMetadata {
        title,
        creator,
        series,
        volume,
        summary,
        language,
        opf_path,
        spine_items,
        cover_image_path,
    })
}

pub fn resolve_relative_path(base_dir: &str, relative: &str) -> String {
    let decoded = urlencoding_decode(relative);
    if base_dir.is_empty() {
        return normalize_zip_path(&decoded);
    }
    let combined = format!("{}/{}", base_dir, decoded);
    normalize_zip_path(&combined)
}

fn normalize_zip_path(path: &str) -> String {
    let mut components = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        } else if part == ".." {
            components.pop();
        } else {
            components.push(part);
        }
    }
    components.join("/")
}

fn urlencoding_decode(input: &str) -> String {
    percent_encoding::percent_decode_str(input)
        .decode_utf8_lossy()
        .to_string()
}

pub fn extract_epub_entry(epub_path: &Path, entry_path: &str) -> AppResult<Vec<u8>> {
    let file = File::open(epub_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let clean_path = normalize_zip_path(entry_path);
    let mut entry = archive
        .by_name(&clean_path)
        .map_err(|e| AppError::NotFound(format!("EPUB entry {} not found: {}", clean_path, e)))?;

    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn rewrite_epub_chapter_html(
    html_content: &str,
    chapter_full_path: &str,
    book_id: i64,
) -> String {
    let base_dir = Path::new(chapter_full_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Regex to rewrite image src attributes
    let re_img = regex::Regex::new(r#"(?i)(<img\s+[^>]*?src=["'])([^"']+)(["'])"#).unwrap();
    let replaced_img = re_img.replace_all(html_content, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let original_src = &caps[2];
        let suffix = &caps[3];

        if original_src.starts_with("http://")
            || original_src.starts_with("https://")
            || original_src.starts_with("data:")
        {
            format!("{}{}{}", prefix, original_src, suffix)
        } else {
            let resolved = resolve_relative_path(&base_dir, original_src);
            let new_src = format!("/api/books/{}/assets?file={}", book_id, resolved);
            format!("{}{}{}", prefix, new_src, suffix)
        }
    });

    // Regex to rewrite xlink:href attributes for SVG images
    let re_svg_img =
        regex::Regex::new(r#"(?i)(<image\s+[^>]*?xlink:href=["'])([^"']+)(["'])"#).unwrap();
    let replaced_svg = re_svg_img.replace_all(&replaced_img, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let original_src = &caps[2];
        let suffix = &caps[3];

        if original_src.starts_with("http://")
            || original_src.starts_with("https://")
            || original_src.starts_with("data:")
        {
            format!("{}{}{}", prefix, original_src, suffix)
        } else {
            let resolved = resolve_relative_path(&base_dir, original_src);
            let new_src = format!("/api/books/{}/assets?file={}", book_id, resolved);
            format!("{}{}{}", prefix, new_src, suffix)
        }
    });

    // Regex to rewrite stylesheet link hrefs
    let re_css = regex::Regex::new(
        r#"(?i)(<link\s+[^>]*?href=["'])([^"']+)(["'][^>]*?rel=["']stylesheet["']|[^>]*?>)"#,
    )
    .unwrap();
    let replaced_css = re_css.replace_all(&replaced_svg, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let original_href = &caps[2];
        let suffix = &caps[3];

        if original_href.starts_with("http://") || original_href.starts_with("https://") {
            format!("{}{}{}", prefix, original_href, suffix)
        } else {
            let resolved = resolve_relative_path(&base_dir, original_href);
            let new_href = format!("/api/books/{}/assets?file={}", book_id, resolved);
            format!("{}{}{}", prefix, new_href, suffix)
        }
    });

    replaced_css.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative_path() {
        assert_eq!(
            resolve_relative_path("OEBPS", "images/cover.jpg"),
            "OEBPS/images/cover.jpg"
        );
        assert_eq!(
            resolve_relative_path("OEBPS/text", "../images/cover.jpg"),
            "OEBPS/images/cover.jpg"
        );
    }

    #[test]
    fn test_rewrite_epub_chapter_html() {
        let sample = r#"<div><img src="../images/fig1.jpg" alt="Figure 1" /></div>"#;
        let rewritten = rewrite_epub_chapter_html(sample, "OEBPS/text/ch1.xhtml", 42);
        assert!(rewritten.contains("/api/books/42/assets?file=OEBPS/images/fig1.jpg"));
    }
}
