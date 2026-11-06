#![allow(dead_code)]

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComicInfo {
    pub title: Option<String>,
    pub series: Option<String>,
    pub number: Option<String>,
    pub volume: Option<String>,
    pub summary: Option<String>,
    pub genre: Option<String>,
    pub page_count: Option<i64>,
    pub language_iso: Option<String>,
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
}

impl ComicInfo {
    pub fn parse_xml(xml_content: &str) -> Option<Self> {
        let doc = roxmltree::Document::parse(xml_content).ok()?;
        let root = doc.root_element();

        if !root.tag_name().name().eq_ignore_ascii_case("ComicInfo") {
            return None;
        }

        let mut info = ComicInfo::default();

        for child in root.children() {
            if !child.is_element() {
                continue;
            }
            let tag = child.tag_name().name();
            let text = child.text().map(|s| s.trim().to_string());

            if let Some(val) = text {
                if val.is_empty() {
                    continue;
                }
                if tag.eq_ignore_ascii_case("Title") {
                    info.title = Some(val);
                } else if tag.eq_ignore_ascii_case("Series") {
                    info.series = Some(val);
                } else if tag.eq_ignore_ascii_case("Number") {
                    info.number = Some(val);
                } else if tag.eq_ignore_ascii_case("Volume") {
                    info.volume = Some(val);
                } else if tag.eq_ignore_ascii_case("Summary") {
                    info.summary = Some(val);
                } else if tag.eq_ignore_ascii_case("Genre") {
                    info.genre = Some(val);
                } else if tag.eq_ignore_ascii_case("LanguageISO") {
                    info.language_iso = Some(val);
                } else if tag.eq_ignore_ascii_case("PageCount") {
                    info.page_count = val.parse::<i64>().ok();
                } else if tag.eq_ignore_ascii_case("Year") {
                    info.year = val.parse::<i32>().ok();
                } else if tag.eq_ignore_ascii_case("Month") {
                    info.month = val.parse::<i32>().ok();
                } else if tag.eq_ignore_ascii_case("Day") {
                    info.day = val.parse::<i32>().ok();
                }
            }
        }

        Some(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comic_info() {
        let sample = r#"<?xml version="1.0"?>
        <ComicInfo xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <Title>The Black Swordsman</Title>
            <Series>Berserk</Series>
            <Number>1</Number>
            <Volume>1</Volume>
            <Summary>A dark fantasy manga set in a medieval Europe-inspired dark fantasy world.</Summary>
            <Genre>Action, Dark Fantasy</Genre>
            <PageCount>224</PageCount>
            <LanguageISO>en</LanguageISO>
            <Year>1990</Year>
        </ComicInfo>"#;

        let parsed = ComicInfo::parse_xml(sample).expect("Should parse ComicInfo.xml");
        assert_eq!(parsed.series.as_deref(), Some("Berserk"));
        assert_eq!(parsed.title.as_deref(), Some("The Black Swordsman"));
        assert_eq!(parsed.volume.as_deref(), Some("1"));
        assert_eq!(parsed.number.as_deref(), Some("1"));
        assert_eq!(parsed.page_count, Some(224));
        assert_eq!(parsed.year, Some(1990));
    }
}
