//! Phase 4.3 RED — page-boundary regression on the OOXML corpus.
//!
//! For each `.docx` fixture we project paragraphs into a doc-model
//! `Doc`, lay it out at Letter @ 96 DPI, and snapshot the page
//! structure. The snapshot is a lossy projection (page count, per-
//! page block count, per-page first / last block_index) — robust to
//! sub-pixel font drift, sensitive to real pagination changes.

use std::fs;
use std::path::PathBuf;

use apalabrar_doc_model::Doc;
use apalabrar_format_docx as docx;
use apalabrar_layout::{LETTER_AT_96DPI, RenderPlan, layout};

#[derive(Debug, serde::Serialize)]
struct PaginationSummary {
    page_count: usize,
    pages: Vec<PageSummary>,
}

#[derive(Debug, serde::Serialize)]
struct PageSummary {
    page_number: usize,
    block_count: usize,
    first_block_index: Option<usize>,
    last_block_index: Option<usize>,
}

fn summarize(plan: &RenderPlan) -> PaginationSummary {
    let pages = plan
        .pages
        .iter()
        .map(|p| PageSummary {
            page_number: p.page_number,
            block_count: p.blocks.len(),
            first_block_index: p.blocks.first().map(|b| b.block_index),
            last_block_index: p.blocks.last().map(|b| b.block_index),
        })
        .collect();
    PaginationSummary {
        page_count: plan.page_count(),
        pages,
    }
}

fn doc_from_docx(bytes: &[u8]) -> Doc {
    let parsed = docx::read(bytes).expect("docx read");
    let texts: Vec<String> = (0..parsed.paragraph_count())
        .map(|i| parsed.paragraph_text(i).unwrap_or("").to_string())
        .collect();
    let mut doc = Doc::new();
    doc.insert(0, &texts.join("\n"));
    doc
}

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests-corpus")
}

/// Pagination snapshot for ALL fixtures of one category, projected
/// into a single Vec so the snapshot test invokes insta exactly once.
/// (Calling `insta::assert_yaml_snapshot!` per-fixture stops on the
/// first not-yet-accepted snap, hiding later regressions.)
#[derive(Debug, serde::Serialize)]
struct CategorySummary {
    category: String,
    fixtures: Vec<FixtureSummary>,
}

#[derive(Debug, serde::Serialize)]
struct FixtureSummary {
    name: String,
    summary: PaginationSummary,
}

fn fixtures_in(category: &str) -> Vec<String> {
    let dir = corpus_root().join(category);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"))
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            (path.extension().and_then(|x| x.to_str()) == Some("docx"))
                .then(|| path.file_stem()?.to_str().map(String::from))
                .flatten()
        })
        .collect();
    names.sort();
    names
}

fn build_category_summary(category: &str) -> CategorySummary {
    let fixtures = fixtures_in(category)
        .into_iter()
        .map(|name| {
            let path = corpus_root().join(category).join(format!("{name}.docx"));
            let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
            let doc = doc_from_docx(&bytes);
            let plan = layout(&doc, &LETTER_AT_96DPI).unwrap();
            FixtureSummary {
                name,
                summary: summarize(&plan),
            }
        })
        .collect();
    CategorySummary {
        category: category.to_owned(),
        fixtures,
    }
}

macro_rules! corpus_category_test {
    ($name:ident, $category:literal) => {
        #[test]
        fn $name() {
            let summary = build_category_summary($category);
            assert!(
                !summary.fixtures.is_empty(),
                "expected ≥1 fixture in tests-corpus/{}",
                $category,
            );
            insta::assert_yaml_snapshot!(stringify!($name), summary);
        }
    };
}

corpus_category_test!(corpus_academic_pagination, "academic");
corpus_category_test!(corpus_multilingual_pagination, "multilingual");
corpus_category_test!(corpus_equations_pagination, "equations");
corpus_category_test!(corpus_footnotes_pagination, "footnotes");
corpus_category_test!(corpus_synthetic_pagination, "synthetic");
corpus_category_test!(corpus_tables_pagination, "tables");
corpus_category_test!(corpus_tracked_pagination, "tracked");
