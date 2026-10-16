//! Maps between source byte offsets and laid-out glyphs, precise to the
//! character. typst-ide's `jump_from_cursor` only resolves a cursor to the
//! start of its text node, which is too coarse to draw a caret or a selection.

use std::collections::HashMap;
use std::ops::Range;
use typst::World;
use typst::layout::{Abs, Frame, FrameItem, Point, Transform};
use typst::syntax::{FileId, Source, Span, SyntaxKind};
use typst_layout::PagedDocument;

/// A glyph on a page, with the source bytes it was shaped from.
pub struct PlacedGlyph {
    /// 0-based page index.
    pub page: usize,
    /// The glyph's box on the page: its advance horizontally, the font's
    /// ascender to descender vertically.
    pub left: Abs,
    pub right: Abs,
    pub top: Abs,
    pub bottom: Abs,
    /// Byte range in the source.
    pub source: Range<usize>,
}

/// A rectangle on a page.
pub struct PageRect {
    /// 0-based page index.
    pub page: usize,
    pub left: Abs,
    pub right: Abs,
    pub top: Abs,
    pub bottom: Abs,
}

/// All glyphs shaped from the source `id`, in document order.
pub fn placed_glyphs(world: &dyn World, document: &PagedDocument, id: FileId) -> Vec<PlacedGlyph> {
    let Ok(source) = world.source(id) else {
        return Vec::new();
    };
    let mut collector = Collector {
        id,
        source: &source,
        nodes: HashMap::new(),
        glyphs: Vec::new(),
    };
    for (page, frame) in document.pages().iter().map(|page| &page.frame).enumerate() {
        collector.collect(frame, Transform::identity(), page);
    }
    collector.glyphs
}

struct Collector<'a> {
    id: FileId,
    source: &'a Source,
    /// Byte range of a span's syntax node and whether it is plain text. Many
    /// glyphs share a span, and looking a span up walks the syntax tree.
    nodes: HashMap<Span, Option<(Range<usize>, bool)>>,
    glyphs: Vec<PlacedGlyph>,
}

impl Collector<'_> {
    fn collect(&mut self, frame: &Frame, ts: Transform, page: usize) {
        for (pos, item) in frame.items() {
            match item {
                FrameItem::Group(group) => {
                    let ts = ts
                        .pre_concat(Transform::translate(pos.x, pos.y))
                        .pre_concat(group.transform);
                    self.collect(&group.frame, ts, page);
                }
                FrameItem::Text(text) => {
                    let metrics = text.font.metrics();
                    let ascent = metrics.ascender.at(text.size);
                    // The descender is negative, below the baseline.
                    let descent = -metrics.descender.at(text.size);
                    let mut x = pos.x;
                    for glyph in &text.glyphs {
                        let width = glyph.x_advance.at(text.size);
                        if let Some(source) = self.source_range(glyph.span, glyph.range().len()) {
                            let top_left = Point::new(x, pos.y - ascent).transform(ts);
                            let bottom_right = Point::new(x + width, pos.y + descent).transform(ts);
                            self.glyphs.push(PlacedGlyph {
                                page,
                                left: top_left.x,
                                right: bottom_right.x,
                                top: top_left.y,
                                bottom: bottom_right.y,
                                source,
                            });
                        }
                        x += width;
                    }
                }
                _ => {}
            }
        }
    }

    /// Source bytes of a glyph, like typst-ide's `jump_from_click` resolves
    /// them: plain text maps character by character, anything else (spaces,
    /// escapes, smart quotes, code) to its whole syntax node.
    fn source_range(&mut self, (span, offset): (Span, u16), len: usize) -> Option<Range<usize>> {
        if span.id() != Some(self.id) {
            return None;
        }
        let source = self.source;
        let (node, is_text) = self
            .nodes
            .entry(span)
            .or_insert_with(|| {
                let node = source.find(span)?;
                Some((
                    node.range(),
                    matches!(node.kind(), SyntaxKind::Text | SyntaxKind::MathText),
                ))
            })
            .clone()?;
        if !is_text {
            return Some(node);
        }
        let start = (node.start + usize::from(offset)).min(node.end);
        Some(start..(start + len).min(node.end))
    }
}

/// Where the caret goes for a byte offset: a vertical line of the height of
/// the glyph next to it, `left == right`.
pub fn caret(glyphs: &[PlacedGlyph], cursor: usize) -> Option<PageRect> {
    let at = |glyph: &PlacedGlyph, x: Abs| PageRect {
        page: glyph.page,
        left: x,
        right: x,
        top: glyph.top,
        bottom: glyph.bottom,
    };
    if let Some(glyph) = glyphs.iter().find(|g| g.source.start == cursor) {
        return Some(at(glyph, glyph.left));
    }
    // Within a ligature, or a multi-byte node that is not plain text.
    if let Some(glyph) = glyphs
        .iter()
        .find(|g| g.source.start < cursor && cursor < g.source.end)
    {
        return Some(at(glyph, interpolate(glyph, cursor)));
    }
    // In markup that leaves nothing visible (`*`, `#v(..)`, a paragraph break,
    // a space trimmed at a line break): behind the closest glyph before it,
    // or in front of the first one if there is none.
    glyphs
        .iter()
        .filter(|g| g.source.end <= cursor)
        .max_by_key(|g| g.source.end)
        .map(|glyph| at(glyph, glyph.right))
        .or_else(|| {
            glyphs
                .iter()
                .filter(|g| g.source.start > cursor)
                .min_by_key(|g| g.source.start)
                .map(|glyph| at(glyph, glyph.left))
        })
}

/// Highlight rectangles for the byte range `range`, one per run of adjacent
/// glyphs, so usually one per line.
pub fn selection(glyphs: &[PlacedGlyph], range: Range<usize>) -> Vec<PageRect> {
    let mut rects: Vec<PageRect> = Vec::new();
    for glyph in glyphs {
        let start = glyph.source.start.max(range.start);
        let end = glyph.source.end.min(range.end);
        if start >= end {
            continue;
        }
        let rect = PageRect {
            page: glyph.page,
            left: interpolate(glyph, start),
            right: interpolate(glyph, end),
            top: glyph.top,
            bottom: glyph.bottom,
        };
        match rects.last_mut() {
            Some(last)
                if last.page == rect.page
                    && (last.right - rect.left).abs() < Abs::pt(0.01)
                    && rect.top < last.bottom
                    && rect.bottom > last.top =>
            {
                last.right = rect.right;
                last.top.set_min(rect.top);
                last.bottom.set_max(rect.bottom);
            }
            _ => rects.push(rect),
        }
    }
    rects
}

/// The byte offset closest to `point` on `page`: on the nearest line, at the
/// nearer edge of the nearest glyph. Unlike a click jump, this also resolves
/// points beside the text, like the margins, line ends or gaps between lines.
pub fn cursor_at(glyphs: &[PlacedGlyph], page: usize, point: Point) -> Option<usize> {
    fn distance(point: Abs, from: Abs, to: Abs) -> f64 {
        if point < from {
            (from - point).to_raw()
        } else if point > to {
            (point - to).to_raw()
        } else {
            0.0
        }
    }
    let glyph = glyphs.iter().filter(|g| g.page == page).min_by(|a, b| {
        let key = |g: &PlacedGlyph| {
            (
                distance(point.y, g.top, g.bottom),
                distance(point.x, g.left, g.right),
            )
        };
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;
    Some(if point.x > (glyph.left + glyph.right) / 2.0 {
        glyph.source.end
    } else {
        glyph.source.start
    })
}

/// The x position of byte offset `cursor` within a glyph, assuming its source
/// bytes spread evenly over its width.
fn interpolate(glyph: &PlacedGlyph, cursor: usize) -> Abs {
    let len = glyph.source.len();
    if len == 0 {
        return glyph.left;
    }
    let ratio = (cursor - glyph.source.start) as f64 / len as f64;
    glyph.left + (glyph.right - glyph.left) * ratio
}
