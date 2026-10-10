//! styles.xml and settings.xml: the manuscript format as Word styles.

use super::layout::Layout;
use super::{CHAPTER_STYLE_NAME, W, twips_pt};
use crate::format::{ManuscriptFormat, font};
use crate::xml;

/// Line pitch of body text in twips.
fn pitch(format: &ManuscriptFormat) -> i64 {
    twips_pt(format.size_pt * f64::from(format.line_spacing) / 100.0)
}

fn title_size(format: &ManuscriptFormat) -> f64 {
    (format.size_pt * 1.6 * 2.0).round() / 2.0
}

pub(super) fn styles(format: &ManuscriptFormat) -> String {
    let body_headings: String = [1.6,1.3,1.1].iter().enumerate().map(|(i,scale)| {
        let level=i+1; let size=(format.size_pt*scale*2.0).round() as i64;
        format!(r#"<w:style w:type="paragraph" w:customStyle="1" w:styleId="WPBodyHeading{level}"><w:name w:val="본문 제목 {level}"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:jc w:val="left"/><w:ind w:firstLineChars="0" w:firstLine="0"/><w:spacing w:line="336" w:lineRule="auto"/></w:pPr><w:rPr><w:b/><w:bCs/><w:sz w:val="{size}"/><w:szCs w:val="{size}"/></w:rPr></w:style>"#)
    }).collect();
    let face = xml::attr(font(&format.font).docx);
    let half_points = (format.size_pt * 2.0).round() as i64;
    let pitch = pitch(format);
    let after = if format.blank_line_between { pitch } else { 0 };
    let indent_chars = (format.indent * 100.0).round() as i64;
    let indent_twips = twips_pt(format.indent * format.size_pt);
    let letter = twips_pt(format.size_pt * f64::from(format.letter_spacing) / 100.0);
    let title_half = (title_size(format) * 2.0) as i64;
    let title_pitch = twips_pt(title_size(format) * 1.4);
    let break_before = if format.chapter_new_page {
        "<w:pageBreakBefore/>"
    } else {
        ""
    };
    let title_before = if format.chapter_new_page {
        pitch * 3
    } else {
        pitch
    };
    let scene_space = if format.blank_line_between { 0 } else { pitch };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="{face}" w:hAnsi="{face}" w:eastAsia="{face}" w:cs="{face}"/><w:sz w:val="{half_points}"/><w:szCs w:val="{half_points}"/><w:lang w:val="ko-KR" w:eastAsia="ko-KR" w:bidi="ar-SA"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="{pitch}" w:lineRule="exact"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="{after}" w:line="{pitch}" w:lineRule="exact"/><w:ind w:firstLineChars="{indent_chars}" w:firstLine="{indent_twips}"/><w:jc w:val="both"/></w:pPr><w:rPr><w:spacing w:val="{letter}"/></w:rPr></w:style><w:style w:type="paragraph" w:customStyle="1" w:styleId="WPChapter"><w:name w:val="{CHAPTER_STYLE_NAME}"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/>{break_before}<w:spacing w:before="{title_before}" w:after="{title_after}" w:line="{title_pitch}" w:lineRule="exact"/><w:ind w:firstLineChars="0" w:firstLine="0"/><w:jc w:val="left"/></w:pPr><w:rPr><w:b/><w:bCs/><w:sz w:val="{title_half}"/><w:szCs w:val="{title_half}"/></w:rPr></w:style><w:style w:type="paragraph" w:customStyle="1" w:styleId="WPSceneBreak"><w:name w:val="장면 나눔"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:spacing w:before="{scene_space}" w:after="{scene_after}"/><w:ind w:firstLineChars="0" w:firstLine="0"/><w:jc w:val="center"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Header"><w:name w:val="header"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/><w:ind w:firstLineChars="0" w:firstLine="0"/></w:pPr><w:rPr><w:spacing w:val="0"/><w:sz w:val="{small_half}"/><w:szCs w:val="{small_half}"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Footer"><w:name w:val="footer"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/><w:ind w:firstLineChars="0" w:firstLine="0"/><w:jc w:val="center"/></w:pPr><w:rPr><w:spacing w:val="0"/><w:sz w:val="{small_half}"/><w:szCs w:val="{small_half}"/></w:rPr></w:style>{body_headings}</w:styles>"#,
        title_after = pitch * 2,
        scene_after = scene_space.max(after),
        small_half = (half_points - 2).max(12),
    )
}

pub(super) fn settings(format: &ManuscriptFormat, layout: &Layout) -> String {
    let m = format.page_margins();
    let mirror = if (m.inside - m.outside).abs() > f64::EPSILON {
        "<w:mirrorMargins/>"
    } else {
        ""
    };
    let even_odd = if layout.even_odd {
        "<w:evenAndOddHeaders/>"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="{W}"><w:zoom w:percent="100"/>{mirror}<w:defaultTabStop w:val="800"/>{even_odd}<w:characterSpacingControl w:val="doNotCompress"/><w:compat><w:doNotExpandShiftReturn/><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    )
}
