//! Source paragraph dimensions to the editor's character-based paragraph shape.

use crate::markup::ParaAttrs;

#[derive(Default, Clone, Copy)]
pub(super) struct Shape {
    pub indent: f64,
    pub align: Option<crate::markup::Align>,
    pub page_break: bool,
    pub left: f64,
    pub right: f64,
}

impl Shape {
    pub fn attrs(self, height: f64) -> ParaAttrs {
        let height = if height.is_finite() && height > 0.0 {
            height
        } else {
            1000.0
        };
        let margin = |value: f64| {
            (value / height)
                .round()
                .clamp(0.0, f64::from(ParaAttrs::MAX)) as u8
        };
        ParaAttrs {
            heading: None,
            align: self.align,
            page_break_before: self.page_break,
            left: margin(self.left),
            right: margin(self.right),
            // Zero is explicit: source flush paragraphs must not inherit the
            // project's automatic chapter/dialogue indentation rules.
            indent: Some((self.indent / height).round().clamp(
                -f64::from(ParaAttrs::MAX_INDENT),
                f64::from(ParaAttrs::MAX_INDENT),
            ) as i8),
        }
    }
}
