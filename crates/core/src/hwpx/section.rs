//! section0.xml: the text, with the section settings, 머리말 and 꼬리말
//! controls in the paragraphs where they start to apply.

use std::fmt::Write as _;

use super::header::{
    CHAR_HEAD, CHAR_TITLE, PARA_HEAD_CENTER, PARA_HEAD_LEFT, PARA_HEAD_RIGHT, PARA_SCENE,
    PARA_TITLE, STYLE_HEAD, STYLE_TITLE,
};
use super::{NAMESPACES, XML_DECL, hu_mm};
use crate::export::{DocInfo, DocOptions, ExportDoc, Piece, RunStyle, pieces};
use crate::format::{HeadAlign, HeadContent, ManuscriptFormat};
use crate::markup::{Block, ParaAttrs};
use crate::xml;

/// Text inside `hp:t`: 묶음 빈칸 (no-break space) and 고정폭 빈칸 (U+2002, a
/// fixed-width space) become the elements 한글 uses for them. A full-width
/// space (U+3000) stays a character.
fn t_content(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut plain = String::new();
    for c in text.chars() {
        let element = match c {
            '\u{A0}' => "<hp:nbSpace/>",
            '\u{2002}' => "<hp:fwSpace/>",
            '\t' => {
                plain.push_str("    ");
                continue;
            }
            c => {
                plain.push(c);
                continue;
            }
        };
        out.push_str(&xml::text(&plain));
        plain.clear();
        out.push_str(element);
    }
    out.push_str(&xml::text(&plain));
    out
}

struct ParagraphWriter {
    out: String,
    next_id: u32,
    /// Section settings, opening the first paragraph's first run.
    pending_sec: Option<String>,
    /// Controls (머리말, 감추기) for the next paragraph written.
    pending_ctrls: String,
}

impl ParagraphWriter {
    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn paragraph(&mut self, para_pr: usize, page_break: bool, runs: &str) {
        let id = self.take_id();
        let ctrls = std::mem::take(&mut self.pending_ctrls);
        let lead = match self.pending_sec.take() {
            Some(sec) => format!("{sec}{ctrls}</hp:run>"),
            None if !ctrls.is_empty() => format!(r#"<hp:run charPrIDRef="0">{ctrls}</hp:run>"#),
            None => String::new(),
        };
        let runs = if runs.is_empty() {
            r#"<hp:run charPrIDRef="0"><hp:t/></hp:run>"#.to_string()
        } else {
            runs.to_string()
        };
        let _ = write!(
            self.out,
            r#"<hp:p id="{id}" paraPrIDRef="{para_pr}" styleIDRef="{}" pageBreak="{}" columnBreak="0" merged="0">{lead}{runs}</hp:p>"#,
            if para_pr == PARA_TITLE {
                STYLE_TITLE
            } else {
                0
            },
            u8::from(page_break)
        );
    }
}

/// The first run of the section, left open so controls can follow it.
fn sec_pr(format: &ManuscriptFormat) -> String {
    let (w, h) = format.page();
    let m = format.page_margins();
    let gutter = if (m.inside - m.outside).abs() > f64::EPSILON {
        "LEFT_RIGHT"
    } else {
        "LEFT_ONLY"
    };
    let page_num = if format.page_numbers {
        let pos = match format.page_number_align {
            HeadAlign::Left => "BOTTOM_LEFT",
            HeadAlign::Center => "BOTTOM_CENTER",
            HeadAlign::Right => "BOTTOM_RIGHT",
            HeadAlign::Outside => "OUTSIDE_BOTTOM",
        };
        format!(r#"<hp:ctrl><hp:pageNum pos="{pos}" formatType="DIGIT" sideChar="-"/></hp:ctrl>"#)
    } else {
        String::new()
    };
    format!(
        r##"<hp:run charPrIDRef="0"><hp:secPr id="" textDirection="HORIZONTAL" spaceColumns="1134" tabStop="8000" tabStopVal="4000" tabStopUnit="HWPUNIT" outlineShapeIDRef="1" memoShapeIDRef="0" textVerticalWidthHead="0" masterPageCnt="0"><hp:grid lineGrid="0" charGrid="0" wonggojiFormat="0"/><hp:startNum pageStartsOn="BOTH" page="0" pic="0" tbl="0" equation="0"/><hp:visibility hideFirstHeader="0" hideFirstFooter="0" hideFirstMasterPage="0" border="SHOW_ALL" fill="SHOW_ALL" hideFirstPageNum="0" hideFirstEmptyLine="0" showLineNumber="0"/><hp:lineNumberShape restartType="0" countBy="0" distance="0" startNumber="0"/><hp:pagePr landscape="WIDELY" width="{w}" height="{h}" gutterType="{gutter}"><hp:margin header="{header}" footer="{footer}" gutter="0" left="{left}" right="{right}" top="{top}" bottom="{bottom}"/></hp:pagePr><hp:footNotePr><hp:autoNumFormat type="DIGIT" userChar="" prefixChar="" suffixChar=")" supscript="0"/><hp:noteLine length="-1" type="SOLID" width="0.12 mm" color="#000000"/><hp:noteSpacing betweenNotes="283" belowLine="567" aboveLine="850"/><hp:numbering type="CONTINUOUS" newNum="1"/><hp:placement place="EACH_COLUMN" beneathText="0"/></hp:footNotePr><hp:endNotePr><hp:autoNumFormat type="DIGIT" userChar="" prefixChar="" suffixChar=")" supscript="0"/><hp:noteLine length="14692344" type="SOLID" width="0.12 mm" color="#000000"/><hp:noteSpacing betweenNotes="0" belowLine="567" aboveLine="850"/><hp:numbering type="CONTINUOUS" newNum="1"/><hp:placement place="END_OF_DOCUMENT" beneathText="0"/></hp:endNotePr><hp:pageBorderFill type="BOTH" borderFillIDRef="1" textBorder="PAPER" headerInside="0" footerInside="0" fillArea="PAPER"><hp:offset left="1417" right="1417" top="1417" bottom="1417"/></hp:pageBorderFill><hp:pageBorderFill type="EVEN" borderFillIDRef="1" textBorder="PAPER" headerInside="0" footerInside="0" fillArea="PAPER"><hp:offset left="1417" right="1417" top="1417" bottom="1417"/></hp:pageBorderFill><hp:pageBorderFill type="ODD" borderFillIDRef="1" textBorder="PAPER" headerInside="0" footerInside="0" fillArea="PAPER"><hp:offset left="1417" right="1417" top="1417" bottom="1417"/></hp:pageBorderFill></hp:secPr><hp:ctrl><hp:colPr id="" type="NEWSPAPER" layout="LEFT" colCount="1" sameSz="1" sameGap="0"/></hp:ctrl>{page_num}"##,
        w = hu_mm(w),
        h = hu_mm(h),
        header = hu_mm(m.header),
        footer = hu_mm(m.footer),
        left = hu_mm(m.inside),
        right = hu_mm(m.outside),
        top = hu_mm(m.top),
        bottom = hu_mm(m.bottom),
    )
}

/// Paragraph shape of a 머리말 or 꼬리말 line on even and odd pages.
fn head_paras(align: HeadAlign) -> (usize, usize) {
    let para = |align: HeadAlign| match align {
        HeadAlign::Left => PARA_HEAD_LEFT,
        HeadAlign::Center => PARA_HEAD_CENTER,
        HeadAlign::Right | HeadAlign::Outside => PARA_HEAD_RIGHT,
    };
    let (even, odd) = align.sides();
    (para(even), para(odd))
}

/// The page area a running line goes in.
#[derive(Clone, Copy, PartialEq)]
enum Area {
    Header,
    Footer,
}

/// Writes 머리말 and 꼬리말 controls.
struct Heads {
    next_id: u32,
    /// Text area width and header and footer heights in HWPUNIT.
    width: i64,
    head_height: i64,
    foot_height: i64,
    odd_para: usize,
    even_para: usize,
    facing: bool,
}

impl Heads {
    fn new(format: &ManuscriptFormat) -> Self {
        let (w, _) = format.page();
        let m = format.page_margins();
        let align = format.header.align;
        let (even_para, odd_para) = head_paras(align);
        Heads {
            next_id: 1,
            width: hu_mm(w - m.inside - m.outside),
            head_height: hu_mm(m.header),
            foot_height: hu_mm(m.footer),
            odd_para,
            even_para,
            facing: align == HeadAlign::Outside,
        }
    }

    fn ctrl(&mut self, w: &mut ParagraphWriter, pages: &str, para_pr: usize, text: &str) -> String {
        self.area_ctrl(w, Area::Header, pages, para_pr, text)
    }

    fn area_ctrl(
        &mut self,
        w: &mut ParagraphWriter,
        area: Area,
        pages: &str,
        para_pr: usize,
        text: &str,
    ) -> String {
        let id = self.next_id;
        self.next_id += 1;
        let p_id = w.take_id();
        let (tag, valign, height) = match area {
            Area::Header => ("header", "TOP", self.head_height),
            Area::Footer => ("footer", "BOTTOM", self.foot_height),
        };
        format!(
            r#"<hp:ctrl><hp:{tag} id="{id}" applyPageType="{pages}"><hp:subList id="" textDirection="HORIZONTAL" lineWrap="BREAK" vertAlign="{valign}" linkListIDRef="0" linkListNextIDRef="0" textWidth="{}" textHeight="{height}" hasTextRef="0" hasNumRef="0"><hp:p id="{p_id}" paraPrIDRef="{para_pr}" styleIDRef="{STYLE_HEAD}" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="{CHAR_HEAD}"><hp:t>{}</hp:t></hp:run></hp:p></hp:subList></hp:{tag}></hp:ctrl>"#,
            self.width,
            t_content(text)
        )
    }

    /// The same text on every page: one control, or one per side when the
    /// place differs on even and odd pages.
    fn both(&mut self, w: &mut ParagraphWriter, text: &str) -> String {
        if self.facing {
            let even = self.ctrl(w, "EVEN", self.even_para, text);
            let odd = self.ctrl(w, "ODD", self.odd_para, text);
            format!("{even}{odd}")
        } else {
            self.ctrl(w, "BOTH", self.odd_para, text)
        }
    }

    /// 꼬리말: the writer's line at the bottom of every page.
    fn footer(&mut self, w: &mut ParagraphWriter, format: &ManuscriptFormat) -> String {
        let text = format.footer.text.trim();
        let (even_para, odd_para) = head_paras(format.footer.align);
        if format.footer.align == HeadAlign::Outside {
            let even = self.area_ctrl(w, Area::Footer, "EVEN", even_para, text);
            let odd = self.area_ctrl(w, Area::Footer, "ODD", odd_para, text);
            format!("{even}{odd}")
        } else {
            self.area_ctrl(w, Area::Footer, "BOTH", odd_para, text)
        }
    }
}

const HIDE_HEAD: &str = r#"<hp:ctrl><hp:pageHiding hideHeader="1" hideFooter="0" hideMasterPage="0" hideBorder="0" hideFill="0" hidePageNum="0"/></hp:ctrl>"#;

/// Returns section0.xml and the preview text.
pub(super) fn section(
    docs: &[ExportDoc],
    format: &ManuscriptFormat,
    opts: &DocOptions,
    info: &DocInfo,
    char_id: &dyn Fn(RunStyle) -> usize,
    para_id: &dyn Fn(ParaAttrs) -> usize,
) -> (String, String) {
    let mut w = ParagraphWriter {
        out: format!("{XML_DECL}<hs:sec {NAMESPACES}>"),
        next_id: 0,
        pending_sec: Some(sec_pr(format)),
        pending_ctrls: String::new(),
    };
    let head = &format.header;
    let mut heads = Heads::new(format);
    let skip_first = format.head_skips_chapter_first();
    let mut preview = String::new();
    let mut add_preview = |text: &str| {
        if preview.chars().count() < 1000 {
            preview.push_str(text);
            preview.push_str("\r\n");
        }
    };

    for (i, doc) in docs.iter().enumerate() {
        let first = i == 0;
        let mut break_pending = format.chapter_new_page && !first;
        // 머리말 for this chapter goes into its first paragraph.
        if head.is_on() {
            let chapter = if doc.heading.trim().is_empty() {
                info.title.as_str()
            } else {
                doc.heading.trim()
            };
            let mut ctrls = String::new();
            match head.content {
                HeadContent::None => {}
                HeadContent::Chapter => ctrls.push_str(&heads.both(&mut w, chapter)),
                HeadContent::TitleChapter => {
                    if first {
                        let para = heads.even_para;
                        ctrls.push_str(&heads.ctrl(&mut w, "EVEN", para, &info.title));
                    }
                    let para = heads.odd_para;
                    ctrls.push_str(&heads.ctrl(&mut w, "ODD", para, chapter));
                }
                HeadContent::Title | HeadContent::Author | HeadContent::Custom if first => {
                    let text = head.fixed_text(&info.title, &info.author);
                    ctrls.push_str(&heads.both(&mut w, &text));
                }
                _ => {}
            }
            if skip_first {
                ctrls.push_str(HIDE_HEAD);
            }
            w.pending_ctrls.push_str(&ctrls);
        }
        if first && format.footer.is_on() {
            let ctrls = heads.footer(&mut w, format);
            w.pending_ctrls.push_str(&ctrls);
        }
        if opts.include_titles && !doc.heading.trim().is_empty() {
            let runs = format!(
                r#"<hp:run charPrIDRef="{CHAR_TITLE}"><hp:t>{}</hp:t></hp:run>"#,
                t_content(doc.heading.trim())
            );
            w.paragraph(PARA_TITLE, break_pending, &runs);
            add_preview(doc.heading.trim());
            break_pending = false;
        } else if !first && !format.chapter_new_page {
            w.paragraph(0, false, "");
        }
        for block in &doc.blocks {
            let page_break = std::mem::take(&mut break_pending);
            match block {
                Block::SceneBreak {} => {
                    let runs = format!(
                        r#"<hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run>"#,
                        t_content(&opts.scene_break)
                    );
                    w.paragraph(PARA_SCENE, page_break, &runs);
                    add_preview(&opts.scene_break);
                }
                Block::Paragraph { attrs, content } => {
                    // Line breaks go inside the text of the run before them.
                    let mut runs: Vec<(usize, String)> = Vec::new();
                    let mut plain = String::new();
                    for piece in pieces(content) {
                        match piece {
                            Piece::Text(text, style) => {
                                plain.push_str(&text);
                                runs.push((char_id(style), t_content(&text)));
                            }
                            Piece::Break => {
                                plain.push(' ');
                                match runs.last_mut() {
                                    Some((_, t)) => t.push_str("<hp:lineBreak/>"),
                                    None => runs.push((0, "<hp:lineBreak/>".into())),
                                }
                            }
                        }
                    }
                    let runs: String = runs
                        .into_iter()
                        .map(|(id, t)| {
                            format!(r#"<hp:run charPrIDRef="{id}"><hp:t>{t}</hp:t></hp:run>"#)
                        })
                        .collect();
                    w.paragraph(para_id(*attrs), page_break, &runs);
                    add_preview(&plain);
                }
            }
        }
        // An empty chapter still needs a paragraph for its controls.
        if !w.pending_ctrls.is_empty() {
            w.paragraph(0, std::mem::take(&mut break_pending), "");
        }
    }
    if w.pending_sec.is_some() {
        w.paragraph(0, false, "");
    }
    w.out.push_str("</hs:sec>");
    (w.out, preview)
}
