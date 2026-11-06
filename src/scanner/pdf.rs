#![allow(dead_code)]

use regex::Regex;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct PdfMetadata {
    pub title: String,
    pub page_count: i64,
    pub file_size: i64,
}

pub fn parse_pdf(file_path: &Path) -> AppResult<PdfMetadata> {
    let mut file = File::open(file_path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len() as i64;

    // Check PDF header
    let mut header = [0u8; 8];
    let bytes_read = file.read(&mut header)?;
    if bytes_read < 4 || !header.starts_with(b"%PDF-") {
        return Err(AppError::BadRequest("Not a valid PDF file".to_string()));
    }

    let title = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled PDF")
        .to_string();

    let page_count = count_pdf_pages(&mut file, file_size).unwrap_or(1);

    Ok(PdfMetadata {
        title,
        page_count: page_count.max(1),
        file_size,
    })
}

fn count_pdf_pages(file: &mut File, file_size: i64) -> Option<i64> {
    // 1. Try to find /Pages << /Count N >> in the trailer / catalog section near the end of the file
    let tail_size = 65536.min(file_size as usize);
    if file.seek(SeekFrom::End(-(tail_size as i64))).is_ok() {
        let mut tail_buf = vec![0u8; tail_size];
        if file.read_exact(&mut tail_buf).is_ok() {
            let tail_str = String::from_utf8_lossy(&tail_buf);
            let re_count = Regex::new(r"/Type\s*/Pages.*?/Count\s+(\d+)").ok()?;
            if let Some(caps) = re_count.captures(&tail_str) {
                if let Some(m) = caps.get(1) {
                    if let Ok(count) = m.as_str().parse::<i64>() {
                        if count > 0 {
                            return Some(count);
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback: stream scanning for /Type /Page (excluding /Pages)
    if file.seek(SeekFrom::Start(0)).is_err() {
        return Some(1);
    }

    let re_page = Regex::new(r"/Type\s*/Page\b").ok()?;
    let mut count = 0i64;
    let mut buffer = vec![0u8; 128 * 1024];

    loop {
        let n = match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(bytes) => bytes,
            Err(_) => break,
        };

        let chunk_str = String::from_utf8_lossy(&buffer[..n]);
        count += re_page.find_iter(&chunk_str).count() as i64;
    }

    if count > 0 {
        Some(count)
    } else {
        Some(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_magic_bytes() {
        let dummy_pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Pages /Count 5 >>\nendobj\n";
        assert!(dummy_pdf.starts_with(b"%PDF-"));
    }
}
