#![allow(dead_code)]

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::AppResult;

pub enum PdfStreamResponse {
    Full {
        bytes: Vec<u8>,
        total_size: u64,
    },
    Partial {
        bytes: Vec<u8>,
        start: u64,
        end: u64,
        total_size: u64,
    },
}

pub fn read_pdf_range(
    file_path: &Path,
    range_header: Option<&str>,
) -> AppResult<PdfStreamResponse> {
    let mut file = File::open(file_path)?;
    let metadata = file.metadata()?;
    let total_size = metadata.len();

    if let Some(range_str) = range_header {
        if let Some(spec) = range_str.strip_prefix("bytes=") {
            let parts: Vec<&str> = spec.split('-').collect();
            if parts.len() == 2 {
                let start_opt = parts[0].parse::<u64>().ok();
                let end_opt = parts[1].parse::<u64>().ok();

                let (start, end) = match (start_opt, end_opt) {
                    (Some(s), Some(e)) => (s, e.min(total_size.saturating_sub(1))),
                    (Some(s), None) => (s, total_size.saturating_sub(1)),
                    (None, Some(suffix)) => {
                        let s = total_size.saturating_sub(suffix);
                        (s, total_size.saturating_sub(1))
                    }
                    _ => (0, total_size.saturating_sub(1)),
                };

                if start <= end && start < total_size {
                    let length = (end - start + 1) as usize;
                    file.seek(SeekFrom::Start(start))?;
                    let mut buffer = vec![0u8; length];
                    file.read_exact(&mut buffer)?;

                    return Ok(PdfStreamResponse::Partial {
                        bytes: buffer,
                        start,
                        end,
                        total_size,
                    });
                }
            }
        }
    }

    // Default: full file
    let mut buffer = Vec::with_capacity(total_size as usize);
    file.read_to_end(&mut buffer)?;

    Ok(PdfStreamResponse::Full {
        bytes: buffer,
        total_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_parsing_helpers() {
        assert_eq!("bytes=0-499".strip_prefix("bytes="), Some("0-499"));
    }
}
