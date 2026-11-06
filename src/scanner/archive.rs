#![allow(dead_code)]

use regex::Regex;
use std::cmp::Ordering;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::scanner::comic_info::ComicInfo;

pub fn is_image_file(name: &str) -> bool {
    let lower = name.to_lowercase();

    // Skip junk files
    if lower.contains("__macosx")
        || lower.starts_with("._")
        || lower.ends_with(".ds_store")
        || lower.ends_with("thumbs.db")
    {
        return false;
    }

    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
        || lower.ends_with(".avif")
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let mut a_num: u64 = 0;
                    while let Some(c) = a_chars.peek() {
                        if c.is_ascii_digit() {
                            a_num = a_num
                                .saturating_mul(10)
                                .saturating_add(c.to_digit(10).unwrap() as u64);
                            a_chars.next();
                        } else {
                            break;
                        }
                    }

                    let mut b_num: u64 = 0;
                    while let Some(c) = b_chars.peek() {
                        if c.is_ascii_digit() {
                            b_num = b_num
                                .saturating_mul(10)
                                .saturating_add(c.to_digit(10).unwrap() as u64);
                            b_chars.next();
                        } else {
                            break;
                        }
                    }

                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                } else {
                    let ca_lower = ca.to_ascii_lowercase();
                    let cb_lower = cb.to_ascii_lowercase();
                    match ca_lower.cmp(&cb_lower) {
                        Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                        }
                        ord => return ord,
                    }
                }
            }
        }
    }
}

pub fn list_archive_pages(archive_path: &Path) -> AppResult<Vec<String>> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut pages = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if is_image_file(&name) {
            pages.push(name);
        }
    }

    pages.sort_by(|a, b| natural_cmp(a, b));
    Ok(pages)
}

pub fn extract_comic_info(archive_path: &Path) -> AppResult<Option<ComicInfo>> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.ends_with("ComicInfo.xml") || name.eq_ignore_ascii_case("ComicInfo.xml") {
            let mut xml_str = String::new();
            entry.read_to_string(&mut xml_str)?;
            if let Some(ci) = ComicInfo::parse_xml(&xml_str) {
                return Ok(Some(ci));
            }
        }
    }

    Ok(None)
}

pub fn extract_cover_image(archive_path: &Path) -> AppResult<Option<(String, Vec<u8>)>> {
    let pages = list_archive_pages(archive_path)?;
    if pages.is_empty() {
        return Ok(None);
    }

    // Look for file containing cover or folder
    let chosen_page = pages
        .iter()
        .find(|p| {
            let l = p.to_lowercase();
            l.contains("cover") || l.contains("folder")
        })
        .unwrap_or(&pages[0]);

    let bytes = read_archive_entry(archive_path, chosen_page)?;
    Ok(Some((chosen_page.clone(), bytes)))
}

pub fn read_archive_entry(archive_path: &Path, entry_name: &str) -> AppResult<Vec<u8>> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut entry = archive.by_name(entry_name).map_err(|e| {
        AppError::NotFound(format!("Entry {} not found in archive: {}", entry_name, e))
    })?;

    let mut buffer = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn parse_filename_heuristics(filename: &str) -> (String, String, String, String) {
    // Stem without extension
    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    // Remove bracketed tags like [ScanGroup] and (2021)
    let re_brackets = Regex::new(r"\[.*?\]|\(.*?\)|<.*?>").unwrap();
    let cleaned = re_brackets.replace_all(stem, " ").to_string();

    let re_vol = Regex::new(r"(?i)(?:\b|_)(?:vol(?:ume|\.)?|v)\s*(\d+)").unwrap();
    let re_ch = Regex::new(r"(?i)(?:\b|_)(?:ch(?:apter|\.)?|c)\s*(\d+(?:\.\d+)?)").unwrap();

    let volume = re_vol
        .captures(&cleaned)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim_start_matches('0').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    let chapter = re_ch
        .captures(&cleaned)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim_start_matches('0').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    // Series name: strip volume and chapter patterns from cleaned stem
    let without_vol_ch = re_ch
        .replace_all(&re_vol.replace_all(&cleaned, " "), " ")
        .to_string();
    let series_name = without_vol_ch
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let series = if series_name.is_empty() {
        stem.to_string()
    } else {
        series_name
    };

    let title = stem.trim().to_string();

    (title, series, volume, chapter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sort() {
        let mut files = vec![
            "page10.jpg",
            "page1.jpg",
            "page2.jpg",
            "page01.jpg",
            "page20.jpg",
            "cover.jpg",
        ];
        files.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            files,
            vec![
                "cover.jpg",
                "page01.jpg",
                "page1.jpg",
                "page2.jpg",
                "page10.jpg",
                "page20.jpg"
            ]
        );
    }

    #[test]
    fn test_filename_heuristics() {
        let filename = "[ScanGroup] Berserk Vol. 01 Ch. 001 (2020).cbz";
        let (title, series, volume, chapter) = parse_filename_heuristics(filename);
        assert_eq!(series, "Berserk");
        assert_eq!(volume, "1");
        assert_eq!(chapter, "1");
        assert!(title.contains("Berserk"));
    }
}
