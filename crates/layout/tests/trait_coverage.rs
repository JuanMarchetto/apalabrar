//! Phase 4.1 — exercise the trait derives on every public type so
//! coverage reflects behaviour rather than auto-derived noise. Each
//! test calls `Debug::fmt`, `Clone::clone`, `Default::default`,
//! `Display::fmt` (for `Error`), and `PartialEq::eq` so the line
//! counters mark those impls as executed.

#![allow(clippy::clone_on_copy)]

use apalabrar_doc_model::Doc;
use apalabrar_layout::{
    BlockBox, BlockKind, LETTER_AT_96DPI, Line, Page, Rect, RenderPlan, Viewport, layout,
};

#[test]
fn rect_traits() {
    let r = Rect {
        x_px: 1.0,
        y_px: 2.0,
        width_px: 3.0,
        height_px: 4.0,
    };
    let copy = r;
    let cloned = r.clone();
    assert_eq!(copy, cloned);
    let dbg = format!("{r:?}");
    assert!(dbg.contains("Rect"));
    let default = Rect::default();
    assert_ne!(default, r);
}

#[test]
fn line_traits() {
    let l = Line {
        width_px: 10.0,
        height_px: 18.0,
        baseline_y_px: 14.0,
    };
    let copy = l;
    let cloned = l.clone();
    assert_eq!(copy, cloned);
    let dbg = format!("{l:?}");
    assert!(dbg.contains("Line"));
}

#[test]
fn block_kind_traits() {
    let kinds = [
        BlockKind::Paragraph,
        BlockKind::Heading { level: 1 },
        BlockKind::ListItem { indent: 2 },
    ];
    for k in kinds {
        let cloned = k.clone();
        assert_eq!(k, cloned);
        let copy = k;
        assert_eq!(copy, cloned);
        let dbg = format!("{k:?}");
        assert!(!dbg.is_empty());
    }
}

#[test]
fn block_box_traits() {
    let bbox = BlockBox {
        block_index: 0,
        kind: BlockKind::Paragraph,
        origin_x_px: 0.0,
        origin_y_px: 0.0,
        width_px: 624.0,
        height_px: 26.0,
        lines: vec![Line {
            width_px: 50.0,
            height_px: 18.0,
            baseline_y_px: 14.0,
        }],
        line_range: 0..1,
    };
    let cloned = bbox.clone();
    assert_eq!(bbox, cloned);
    let dbg = format!("{bbox:?}");
    assert!(dbg.contains("BlockBox"));
}

#[test]
fn page_traits() {
    let p = Page::default();
    let cloned = p.clone();
    assert_eq!(p, cloned);
    let dbg = format!("{p:?}");
    assert!(dbg.contains("Page"));
}

#[test]
fn render_plan_traits() {
    let plan = RenderPlan::default();
    let cloned = plan.clone();
    assert_eq!(plan, cloned);
    let dbg = format!("{plan:?}");
    assert!(dbg.contains("RenderPlan"));
}

#[test]
fn viewport_traits() {
    let v = LETTER_AT_96DPI;
    let copy = v;
    let cloned = v.clone();
    assert_eq!(copy, cloned);
    let dbg = format!("{v:?}");
    assert!(dbg.contains("Viewport"));
}

#[test]
fn error_display_and_debug() {
    let mut d = Doc::new();
    d.insert(0, "x");
    let bad = Viewport {
        page_width_px: 100.0,
        page_height_px: 100.0,
        margin_px: 60.0,
    };
    let err = layout(&d, &bad).unwrap_err();
    let display = format!("{err}");
    let debug = format!("{err:?}");
    assert!(display.contains("non-positive"));
    assert!(debug.contains("EmptyContentArea"));
}
