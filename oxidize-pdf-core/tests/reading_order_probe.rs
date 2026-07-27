//! Reading-order probe (#448 entrega 3, §5.7 of the design).
//!
//! Runs a geometric ordering over the corpus WITHOUT wiring it into the
//! extractor, and reports the resulting order metric against poppler. It exists
//! to decide whether an XY cut justifies rewriting the hot path, BEFORE
//! rewriting it — the same method that produced 6298 → 752 for `TD` before a
//! line of the extractor was touched.
//!
//! The orderer under test is the one the repository ALREADY has:
//! `pipeline::reading_order::XYCutReadingOrder`, used today by the partition
//! pipeline and never by the flat `.text` path (that is the #448 defect). The
//! probe measures it as-is with its absolute `min_gap`, and also with a gap
//! made relative to the page's own type scale, because "absolute threshold" is
//! precisely the bug family that produced #448.
//!
//! LIMITATIONS, so the number is not read for more than it is:
//!
//! 1. The probe groups fragments into lines with its OWN geometric rule, not
//!    with the extractor's separator gates (which only exist wired in). So this
//!    is an ESTIMATE of the reachable order. The definitive number comes from
//!    the differential gate once the wiring exists.
//! 2. It reads `.fragments`, which requires `preserve_layout = true`. That runs
//!    `merge_close_fragments` (adjacent-only, order-preserving — verified in
//!    `extraction.rs`), so the fragments still arrive in content-stream order,
//!    but they are not byte-identical to what the flat path emits.
//! 3. Its own baseline (`base`) is the same grouping with NO reordering. Every
//!    comparison is probe-baseline against probe-variant, so the grouping rule
//!    cancels out and the delta isolates the ordering. Comparing a probe
//!    variant against the gate's number would not isolate anything.
//!
//! `#[ignore]` by default: this is a manual measurement over the full corpus,
//! not a test. Run with:
//!   nice -n 10 cargo test -p oxidize-pdf --test reading_order_probe --release \
//!     -- --ignored --nocapture

mod corpus_support;

#[path = "common/order_metric.rs"]
mod order_metric;

#[path = "common/synthetic_pdf.rs"]
mod synthetic_pdf;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use order_metric::{order_metrics_words, words, OrderMetrics};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::pipeline::reading_order::{ReadingOrder, XYCutReadingOrder};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor, TextFragment};

const PER_FILE_TIMEOUT_SECS: u64 = 30;

/// The orderings compared in one pass over the corpus.
///
/// `Base` is the control: identical grouping, no reordering. Everything else is
/// judged against it, so the probe's own grouping rule cannot be mistaken for
/// an effect of the ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    /// Content-stream order (control).
    Base,
    /// The existing `XYCutReadingOrder` with its shipped `min_gap = 20.0`
    /// absolute points, over LINE GROUPS.
    ///
    /// Running it over RAW fragments — the way the partition pipeline uses it —
    /// was measured first and then dropped: it emits each fragment separately,
    /// so a word the producer split across two runs ("adminis" + "tration")
    /// tokenises differently from every group-based variant. That is a
    /// different TOKENISATION, not a different ORDER, and it made 302 of the
    /// corpus files (the long ones) incomparable.
    GroupsAbs20,
    /// Line groups with the gap threshold relative to the page's median type
    /// size (`k × median font size`), which is what §5.4 argues an absolute
    /// constant should be replaced by.
    GroupsRel(u8),
    /// What §5.4 actually specifies, which is NOT what the existing orderer
    /// does: cut on whichever axis shows the widest gap relative to the
    /// region's own type scale (the existing one always tries vertical first),
    /// and fall back to CONTENT-STREAM order where no cut is significant (the
    /// existing one re-sorts every leaf geometrically). `k` is in tenths of an
    /// em.
    WidestGapStreamLeaf(u8),
    /// Same cut rule, but leaves re-sorted geometrically (Y desc, X asc), to
    /// separate the effect of the cut rule from the effect of the leaf policy.
    WidestGapGeomLeaf(u8),
}

impl Variant {
    fn label(self) -> String {
        match self {
            Variant::Base => "base(stream)".to_string(),
            Variant::GroupsAbs20 => "grp_abs20".to_string(),
            Variant::GroupsRel(k) => format!("grp_rel{:.1}em", f64::from(k) / 10.0),
            Variant::WidestGapStreamLeaf(k) => format!("widest{:.1}em_stream", f64::from(k) / 10.0),
            Variant::WidestGapGeomLeaf(k) => format!("widest{:.1}em_geom", f64::from(k) / 10.0),
        }
    }
}

fn variants() -> Vec<Variant> {
    vec![
        Variant::Base,
        Variant::GroupsAbs20,
        Variant::GroupsRel(5),
        Variant::GroupsRel(10),
        Variant::GroupsRel(20),
        Variant::GroupsRel(40),
        Variant::WidestGapStreamLeaf(3),
        Variant::WidestGapStreamLeaf(5),
        Variant::WidestGapStreamLeaf(8),
        Variant::WidestGapStreamLeaf(10),
        Variant::WidestGapStreamLeaf(15),
        Variant::WidestGapStreamLeaf(20),
        Variant::WidestGapStreamLeaf(40),
        Variant::WidestGapGeomLeaf(10),
    ]
}

/// Options that make the extractor COLLECT fragments without reordering them:
/// `preserve_layout` turns collection on (`emit_text_fragment` is gated on it),
/// `sort_by_position = false` keeps the early positional sort from running, and
/// `reconstruct_paragraphs = false` keeps line/paragraph merging out.
fn probe_options() -> ExtractionOptions {
    ExtractionOptions {
        preserve_layout: true,
        sort_by_position: false,
        reconstruct_paragraphs: false,
        ..Default::default()
    }
}

/// A line group: its box, its type scale, and its text.
struct Group {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
    text: String,
}

/// Groups consecutive fragments into lines: a new line opens when the
/// fragment's baseline departs from the running line's by more than half its
/// type size, or when it steps back to the left of where the line started.
///
/// Within a line, a space is inserted only when the horizontal gap exceeds
/// `space_threshold × font size` — the extractor's own rule
/// (`space_gap_threshold`, `extraction.rs:627`). Joining unconditionally
/// instead splits every word the producer emitted as two runs ("adminis" +
/// "tration"), which destroys the token alignment the metric is built on: the
/// first corpus run of this probe lost 17 points of alignment coverage against
/// the flat path for exactly that reason.
///
/// This is still an approximation — see limitation 1 in the module header. Its
/// job is to be the SAME rule on both sides of the comparison.
fn group_into_lines(frags: &[TextFragment]) -> Vec<Group> {
    const SPACE_THRESHOLD: f64 = 0.3; // ExtractionOptions::default().space_threshold
    let mut groups: Vec<Group> = Vec::new();
    let mut prev_right = 0.0f64;
    for f in frags {
        let size = if f.font_size.abs() > 0.0 {
            f.font_size.abs()
        } else {
            1.0
        };
        let same_line = groups
            .last()
            .is_some_and(|g| (g.y - f.y).abs() <= size * 0.5 && f.x + f.width >= g.x);
        if same_line {
            let gap = f.x - prev_right;
            let g = groups.last_mut().expect("same_line implies non-empty");
            let right = (f.x + f.width).max(g.x + g.width);
            g.x = g.x.min(f.x);
            g.width = right - g.x;
            g.height = g.height.max(size);
            if gap > SPACE_THRESHOLD * size {
                g.text.push(' ');
            }
            g.text.push_str(&f.text);
        } else {
            groups.push(Group {
                x: f.x,
                y: f.y,
                width: f.width,
                height: size,
                scale: size,
                text: f.text.clone(),
            });
        }
        prev_right = f.x + f.width;
    }
    groups
}

/// Median of the type sizes on the page, used as the unit for the relative
/// gap. Median rather than mean so a single display heading does not set the
/// scale for a page of body text.
fn median_scale(groups: &[Group]) -> f64 {
    if groups.is_empty() {
        return 0.0;
    }
    let mut scales: Vec<f64> = groups.iter().map(|g| g.scale).collect();
    scales.sort_by(f64::total_cmp);
    scales[scales.len() / 2]
}

/// A line group carried as a `TextFragment` so the existing orderer — which
/// only ever reads `x`, `y`, `width`, `height` — can order groups instead of
/// raw fragments.
fn group_as_fragment(g: &Group) -> TextFragment {
    TextFragment {
        text: g.text.clone(),
        x: g.x,
        y: g.y,
        width: g.width,
        height: g.height,
        font_size: g.scale,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
        mcid: None,
        struct_tag: None,
    }
}

/// The page's text under one ordering.
fn page_text(frags: &[TextFragment], variant: Variant) -> String {
    let groups = group_into_lines(frags);
    let em = |k: u8| f64::from(k) / 10.0;
    match variant {
        Variant::Base => join_texts(groups.iter().map(|g| g.text.as_str())),
        Variant::GroupsAbs20 | Variant::GroupsRel(_) => {
            let min_gap = match variant {
                Variant::GroupsAbs20 => Some(20.0),
                // A page with no usable type scale gets no cut at all rather
                // than a cut at gap 0, which would split everywhere.
                Variant::GroupsRel(k) => {
                    let scale = median_scale(&groups);
                    (scale > 0.0).then(|| em(k) * scale)
                }
                _ => unreachable!("arm guarded by the outer match"),
            };
            match min_gap {
                None => join_texts(groups.iter().map(|g| g.text.as_str())),
                Some(gap) => {
                    let mut carriers: Vec<TextFragment> =
                        groups.iter().map(group_as_fragment).collect();
                    XYCutReadingOrder::new(gap).order(&mut carriers);
                    join_texts(carriers.iter().map(|f| f.text.as_str()))
                }
            }
        }
        Variant::WidestGapStreamLeaf(k) | Variant::WidestGapGeomLeaf(k) => {
            let stream_leaf = matches!(variant, Variant::WidestGapStreamLeaf(_));
            let order = widest_gap_cut(&groups, em(k), stream_leaf);
            join_texts(order.into_iter().map(|i| groups[i].text.as_str()))
        }
    }
}

/// Recursive XY cut as §5.4 specifies it, over line groups.
///
/// Two differences from `XYCutReadingOrder`, both deliberate and both measured
/// separately here:
///
/// - **The axis is chosen, not fixed.** Both projections are measured and the
///   cut happens on whichever shows the wider gap *relative to the region's own
///   type scale*. The existing orderer always attempts the vertical cut first,
///   so any page with a wide left or right margin band splits into columns that
///   are not there.
/// - **A region with no significant gap keeps content-stream order** when
///   `stream_leaf`, instead of being re-sorted by geometry. Stream order is
///   what the median document already gets right, so re-sorting an
///   uncuttable region can only lose.
fn widest_gap_cut(groups: &[Group], k_em: f64, stream_leaf: bool) -> Vec<usize> {
    const MAX_DEPTH: u32 = 16;

    fn scale_of(groups: &[Group], idx: &[usize]) -> f64 {
        let mut s: Vec<f64> = idx.iter().map(|&i| groups[i].scale).collect();
        s.sort_by(f64::total_cmp);
        s.get(s.len() / 2).copied().unwrap_or(0.0)
    }

    /// Widest gap in a set of 1-D intervals, and the coordinate that splits it.
    fn widest_gap(mut spans: Vec<(f64, f64)>) -> Option<(f64, f64)> {
        if spans.len() < 2 {
            return None;
        }
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut max_end = spans[0].1;
        let mut best: Option<(f64, f64)> = None;
        for w in spans.windows(2) {
            let gap = w[1].0 - max_end;
            if gap > best.map_or(0.0, |(g, _)| g) {
                best = Some((gap, max_end + gap / 2.0));
            }
            max_end = max_end.max(w[1].1);
        }
        best.filter(|(g, _)| *g > 0.0)
    }

    fn emit_leaf(groups: &[Group], idx: &mut Vec<usize>, stream_leaf: bool, out: &mut Vec<usize>) {
        if stream_leaf {
            idx.sort_unstable();
        } else {
            idx.sort_by(|&a, &b| {
                groups[b]
                    .y
                    .total_cmp(&groups[a].y)
                    .then_with(|| groups[a].x.total_cmp(&groups[b].x))
            });
        }
        out.extend(idx.iter().copied());
    }

    fn cut(
        groups: &[Group],
        idx: &mut Vec<usize>,
        k_em: f64,
        stream_leaf: bool,
        depth: u32,
        out: &mut Vec<usize>,
    ) {
        if idx.len() <= 1 || depth >= MAX_DEPTH {
            emit_leaf(groups, idx, stream_leaf, out);
            return;
        }
        let scale = scale_of(groups, idx);
        if !(scale > 0.0) {
            emit_leaf(groups, idx, stream_leaf, out);
            return;
        }
        let threshold = k_em * scale;

        let vertical = widest_gap(
            idx.iter()
                .map(|&i| (groups[i].x, groups[i].x + groups[i].width))
                .collect(),
        )
        .filter(|(g, _)| *g >= threshold);
        let horizontal = widest_gap(
            idx.iter()
                .map(|&i| (groups[i].y, groups[i].y + groups[i].height))
                .collect(),
        )
        .filter(|(g, _)| *g >= threshold);

        // Wider gap wins; a tie goes to the vertical cut, because a gutter that
        // matches a paragraph break in width is still a gutter.
        let pick_vertical = match (vertical, horizontal) {
            (Some((gv, _)), Some((gh, _))) => gv >= gh,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => {
                emit_leaf(groups, idx, stream_leaf, out);
                return;
            }
        };

        let (mut first, mut second) = if pick_vertical {
            let (_, split) = vertical.expect("pick_vertical implies a vertical gap");
            partition(idx, |i| groups[i].x + groups[i].width / 2.0 < split)
        } else {
            // Y grows UP in PDF page space, so the TOP half is the one above
            // the split and is read first.
            let (_, split) = horizontal.expect("!pick_vertical implies a horizontal gap");
            partition(idx, |i| groups[i].y + groups[i].height / 2.0 >= split)
        };
        if first.is_empty() || second.is_empty() {
            emit_leaf(groups, idx, stream_leaf, out);
            return;
        }
        cut(groups, &mut first, k_em, stream_leaf, depth + 1, out);
        cut(groups, &mut second, k_em, stream_leaf, depth + 1, out);
    }

    fn partition(idx: &[usize], pred: impl Fn(usize) -> bool) -> (Vec<usize>, Vec<usize>) {
        let mut yes = Vec::new();
        let mut no = Vec::new();
        for &i in idx {
            if pred(i) {
                yes.push(i);
            } else {
                no.push(i);
            }
        }
        (yes, no)
    }

    let mut idx: Vec<usize> = (0..groups.len()).collect();
    let mut out = Vec::with_capacity(groups.len());
    cut(groups, &mut idx, k_em, stream_leaf, 0, &mut out);
    out
}

fn join_texts<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for p in parts {
        out.push_str(p);
        out.push('\n');
    }
    out
}

/// What one parse of a document yields: its per-page fragments in
/// content-stream order, and how many of its pages declare a non-zero
/// `/Rotate` (§5.3 needs the prevalence before paying for rotation-aware
/// ordering). Both come from ONE parse: a second one would double the cost of
/// the corpus run for a page-dictionary field.
struct ParsedDoc {
    pages: Vec<Vec<TextFragment>>,
    /// `.text` of the SHIPPING flat path, one entry per page. This is the
    /// only reference that matters: a variant is worth wiring in when it beats
    /// what the extractor emits today, not when it beats the probe's own
    /// grouping. Keeping both apart is what caught the probe's first corpus
    /// run, whose baseline was 17 coverage points below the flat path.
    flat_pages: Vec<String>,
    rotated_pages: usize,
    page_count: usize,
}

fn parse_doc(path: &Path) -> Option<ParsedDoc> {
    parse_bytes(std::fs::read(path).ok()?)
}

fn parse_bytes(bytes: Vec<u8>) -> Option<ParsedDoc> {
    let doc = PdfReader::new_with_options(std::io::Cursor::new(bytes), ParseOptions::lenient())
        .ok()?
        .into_document();
    let mut ex = TextExtractor::with_options(probe_options());
    let mut flat = TextExtractor::new();
    let mut pages = Vec::new();
    let mut flat_pages: Vec<String> = Vec::new();
    let mut rotated_pages = 0usize;
    let mut page_count = 0usize;
    for i in 0..doc.page_count().unwrap_or(0) {
        if let Ok(p) = doc.get_page(i) {
            page_count += 1;
            if p.rotation % 360 != 0 {
                rotated_pages += 1;
            }
        }
        if let Ok(page) = ex.extract_from_page(&doc, i) {
            pages.push(page.fragments);
        }
        if let Ok(page) = flat.extract_from_page(&doc, i) {
            flat_pages.push(page.text);
        }
    }
    Some(ParsedDoc {
        pages,
        flat_pages,
        rotated_pages,
        page_count,
    })
}

/// Per-page fragments only, for the diagnostics that do not need the rest.
fn fragments_by_page(path: &Path) -> Option<Vec<Vec<TextFragment>>> {
    parse_doc(path).map(|d| d.pages)
}

fn poppler(path: &Path) -> Option<String> {
    let o = Command::new("timeout")
        .arg(PER_FILE_TIMEOUT_SECS.to_string())
        .arg("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    String::from_utf8(o.stdout).ok()
}

/// One file's contribution: the metrics of the shipping flat path and of every
/// variant, in `variants()` order, plus its rotated/total page counts.
struct FileSample {
    flat: OrderMetrics,
    metrics: Vec<OrderMetrics>,
    rotated_pages: usize,
    pages: usize,
}

/// Why a file contributed nothing. Counted rather than swallowed: a silent
/// skip pile is how a probe ends up reporting a number for a biased subset of
/// the corpus.
#[derive(Debug, Default)]
struct Skips {
    poppler_failed: usize,
    no_poppler_words: usize,
    parse_failed: usize,
    nothing_aligned: usize,
    not_a_permutation: usize,
    timed_out: usize,
}

enum Outcome {
    Sampled(Box<FileSample>),
    Skipped(fn(&mut Skips)),
}

fn sample_file(path: &Path) -> Outcome {
    let Some(pop) = poppler(path) else {
        return Outcome::Skipped(|s| s.poppler_failed += 1);
    };
    let pop_words = words(&pop);
    if pop_words.is_empty() {
        return Outcome::Skipped(|s| s.no_poppler_words += 1);
    }
    let Some(doc) = parse_doc(path) else {
        return Outcome::Skipped(|s| s.parse_failed += 1);
    };
    let pages = &doc.pages;

    let vs = variants();
    let mut metrics = Vec::with_capacity(vs.len());
    for v in &vs {
        let mut our_words: Vec<String> = Vec::new();
        for frags in pages {
            our_words.extend(words(&page_text(frags, *v)));
        }
        metrics.push(order_metrics_words(&our_words, &pop_words));
    }
    // Nothing alignable in this pair (a scanned page, no ≥4-letter words):
    // excluded, so an empty side cannot inflate the denominator.
    if metrics[0].common == 0 {
        return Outcome::Skipped(|s| s.nothing_aligned += 1);
    }
    // Every variant is a PERMUTATION of the same emission, so the aligned
    // multiset — and therefore `common` — must be identical across variants.
    // A file where it is not has an ordering that lost or duplicated text
    // (the failure mode that disqualified `reorder_columns`: it bought order
    // with 6.7% of the content), so its variants are not comparable and it is
    // dropped — counted, not swallowed.
    if metrics.iter().any(|m| m.common != metrics[0].common) {
        return Outcome::Skipped(|s| s.not_a_permutation += 1);
    }

    let flat_words: Vec<String> = doc.flat_pages.iter().flat_map(|t| words(t)).collect();
    let flat = order_metrics_words(&flat_words, &pop_words);
    Outcome::Sampled(Box::new(FileSample {
        flat,
        metrics,
        rotated_pages: doc.rotated_pages,
        pages: doc.page_count,
    }))
}

/// Per-file work on a worker thread with a hard timeout, so a slow parse is
/// excluded rather than fatal. Every exclusion lands in `Skips`.
fn sample_with_timeout(path: &Path, skips: &mut Skips) -> Option<FileSample> {
    let p: PathBuf = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sample_file(&p)));
            let _ = tx.send(res.unwrap_or(Outcome::Skipped(|s| s.parse_failed += 1)));
        });
    if handle.is_err() {
        skips.timed_out += 1;
        return None;
    }
    match rx.recv_timeout(Duration::from_secs(PER_FILE_TIMEOUT_SECS * 3)) {
        Ok(Outcome::Sampled(s)) => Some(*s),
        Ok(Outcome::Skipped(bump)) => {
            bump(skips);
            None
        }
        Err(_) => {
            skips.timed_out += 1;
            None
        }
    }
}

/// Running totals for one variant.
#[derive(Default)]
struct Totals {
    misplaced: usize,
    common: usize,
    in_order: usize,
    per_doc: Vec<f64>,
}

fn corpus_dir() -> PathBuf {
    match std::env::var("OXIDIZE_PROBE_CORPUS") {
        Ok(p) => PathBuf::from(p),
        Err(_) => corpus_support::corpus_root().join("t3-stress"),
    }
}

/// A two-column page whose columns are drawn RIGHT FIRST — the exact defect
/// #448 describes, with the correct answer known by construction. It is the
/// probe's own instrument check: if the probe cannot recover reading order
/// here, a corpus number saying "the XY cut does not help" would be measuring
/// the probe's plumbing, not the algorithm.
///
/// 612x792 page; left column x=60..240, right column x=360..540, six lines
/// each at 12pt on a 20pt leading — a 120pt gutter, far wider than any of the
/// thresholds under test.
fn two_column_page_drawn_right_first() -> Vec<u8> {
    let mut c = String::from("BT\n/F1 12 Tf\n");
    for (i, y) in (0..6).map(|i| (i, 700 - i * 20)) {
        c.push_str(&format!("1 0 0 1 360 {y} Tm\n[(rightword{i})] TJ\n"));
    }
    for (i, y) in (0..6).map(|i| (i, 700 - i * 20)) {
        c.push_str(&format!("1 0 0 1 60 {y} Tm\n[(leftword{i})] TJ\n"));
    }
    c.push_str("ET");
    synthetic_pdf::build_pdf_with_content_stream(c.as_bytes())
}

/// Index of the first right-column word and of the last left-column word.
/// Reading order is recovered exactly when every left word precedes every
/// right word.
fn column_span(text: &str) -> (Option<usize>, Option<usize>) {
    (text.find("rightword"), text.rfind("leftword"))
}

#[test]
fn the_probe_recovers_reading_order_on_a_page_whose_answer_is_known() {
    let pages = parse_bytes(two_column_page_drawn_right_first())
        .expect("synthetic PDF must parse")
        .pages;
    assert_eq!(pages.len(), 1, "the fixture has exactly one page");
    let frags = &pages[0];
    assert_eq!(
        frags.len(),
        12,
        "the fixture draws twelve runs; if they were merged the probe is grouping \
         columns together and no ordering can separate them: {:?}",
        frags.iter().map(|f| f.text.as_str()).collect::<Vec<_>>()
    );

    // The control reproduces the defect: the right column is emitted first.
    let base = page_text(frags, Variant::Base);
    let (first_right, last_left) = column_span(&base);
    assert!(
        first_right.expect("right column present") < last_left.expect("left column present"),
        "the control must reproduce the stream-order defect, else the fixture proves nothing: \
         {base:?}"
    );

    // Every ordering variant under test must fix it. A variant that cannot
    // separate a 120pt gutter cannot separate anything on a real page, and its
    // corpus number would be measuring nothing.
    for v in variants().into_iter().filter(|v| *v != Variant::Base) {
        let text = page_text(frags, v);
        let (first_right, last_left) = column_span(&text);
        assert!(
            last_left.expect("left column present") < first_right.expect("right column present"),
            "variant {} left the columns interleaved on a page with a 120pt gutter: {text:?}",
            v.label()
        );
    }
}

/// Structure dump for one document, so a "the cut found nothing" result can be
/// attributed to the page (no gutter to find) or to the probe (grouping that
/// glued the columns together). Prints the line groups of the first pages with
/// their boxes, and how many cuts the §5.4 rule performs.
///
/// Run with `OXIDIZE_PROBE_CORPUS=<dir holding one pdf>`.
#[test]
#[ignore = "manual diagnostic on one document; see the module header"]
fn dump_line_groups_of_first_pages() {
    let dir = corpus_dir();
    let pdfs = corpus_support::find_pdfs(&dir);
    let path = pdfs.first().expect("point OXIDIZE_PROBE_CORPUS at a PDF");
    let pages = fragments_by_page(path).expect("document must parse");
    println!("\n{} · pages={}", path.display(), pages.len());

    let only: Option<usize> = std::env::var("OXIDIZE_PROBE_PAGE")
        .ok()
        .and_then(|v| v.parse().ok());
    for (pi, frags) in pages
        .iter()
        .enumerate()
        .filter(|(i, _)| only.is_none_or(|p| *i == p))
        .take(3)
    {
        let groups = group_into_lines(frags);
        let scale = median_scale(&groups);
        let x_min = groups.iter().map(|g| g.x).fold(f64::INFINITY, f64::min);
        let x_max = groups
            .iter()
            .map(|g| g.x + g.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let span = x_max - x_min;
        let wide = groups.iter().filter(|g| g.width > span * 0.6).count();
        let order = widest_gap_cut(&groups, 2.0, true);
        let moved = order.iter().enumerate().filter(|(i, &j)| *i != j).count();
        println!(
            "\npage {pi}: fragments={} groups={} median_scale={scale:.1} \
             x_span=[{x_min:.0},{x_max:.0}] groups_wider_than_60%={wide} \
             groups_moved_by_widest2.0em={moved}",
            frags.len(),
            groups.len()
        );
        for g in groups.iter().take(12) {
            let t: String = g.text.chars().take(48).collect();
            println!(
                "  x={:7.1} y={:7.1} w={:7.1} scale={:4.1} | {t}",
                g.x, g.y, g.width, g.scale
            );
        }
    }
}

/// Per-page fidelity against poppler's own per-page output, next to the
/// whole-document number.
///
/// The two answer different questions. If a document scores badly as a whole
/// but every page scores well against that same page from poppler, then the
/// words are in the right order WITHIN the page and the disagreement is about
/// which page they belong to — a defect an in-page reading order cannot fix,
/// and one #448 does not currently claim.
///
/// Run with `OXIDIZE_PROBE_CORPUS=<dir holding one pdf>`.
#[test]
#[ignore = "manual diagnostic on one document; see the module header"]
fn dump_per_page_fidelity() {
    let dir = corpus_dir();
    let pdfs = corpus_support::find_pdfs(&dir);
    let path = pdfs.first().expect("point OXIDIZE_PROBE_CORPUS at a PDF");
    let doc = parse_doc(path).expect("document must parse");

    let (mut sum_pop, mut sum_flat, mut sum_base) = (0usize, 0usize, 0usize);
    println!("\n{} · pages={}", path.display(), doc.pages.len());
    println!(
        "{:>5} {:>10} {:>9} {:>9} {:>9} {:>9}   {}",
        "page", "pop_words", "flat_cov", "flat_fid", "base_cov", "base_fid", "worse"
    );

    for (i, frags) in doc.pages.iter().enumerate() {
        let Some(pop_page) = poppler_page(path, i + 1) else {
            continue;
        };
        let pw = words(&pop_page);
        if pw.is_empty() {
            continue;
        }
        let base = order_metrics_words(&words(&page_text(frags, Variant::Base)), &pw);
        let flat = order_metrics_words(
            &words(doc.flat_pages.get(i).map(String::as_str).unwrap_or("")),
            &pw,
        );
        sum_pop += pw.len();
        sum_flat += flat.in_order;
        sum_base += base.in_order;
        // Only the pages where the two emissions actually differ are worth
        // printing: on a 429-page document the rest is noise.
        if flat.in_order + 5 < base.in_order || flat.common + 5 < base.common {
            println!(
                "{i:>5} {:>10} {:>9.4} {:>9.4} {:>9.4} {:>9.4}   flat",
                pw.len(),
                flat.common as f64 / pw.len() as f64,
                flat.in_order as f64 / flat.common.max(1) as f64,
                base.common as f64 / pw.len() as f64,
                base.in_order as f64 / base.common.max(1) as f64,
            );
        }
    }

    println!(
        "\npages aligned independently: pop_words={sum_pop} \
         flat_in_order={sum_flat} base_in_order={sum_base}"
    );
}

/// Side-by-side word sequences for one page (`OXIDIZE_PROBE_PAGE`, 0-based),
/// with the alignment marked: the words we place in poppler's relative order
/// versus the ones we do not. Without this, a bad fidelity number says a page
/// disagrees but not HOW, and the shape of the disagreement is what decides
/// whether an ordering pass can fix it.
#[test]
#[ignore = "manual diagnostic on one page; see the module header"]
fn dump_page_alignment() {
    let dir = corpus_dir();
    let pdfs = corpus_support::find_pdfs(&dir);
    let path = pdfs.first().expect("point OXIDIZE_PROBE_CORPUS at a PDF");
    let page: usize = std::env::var("OXIDIZE_PROBE_PAGE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let doc = parse_doc(path).expect("document must parse");
    let frags = doc.pages.get(page).expect("page index out of range");

    let base = words(&page_text(frags, Variant::Base));
    let flat_text = doc.flat_pages.get(page).cloned().unwrap_or_default();
    let flat = words(&flat_text);
    let pop = words(&poppler_page(path, page + 1).expect("poppler must read the page"));

    let dump = |name: &str, ws: &[String]| {
        println!("\n{name}, {} words:", ws.len());
        for (i, w) in ws.iter().enumerate() {
            print!("{w} ");
            if i % 12 == 11 {
                println!();
            }
        }
        println!();
    };
    dump("FLAT PATH (ships)", &flat);
    dump("BASE (probe grouping, stream order)", &base);
    dump("POPPLER", &pop);

    // Which of poppler's words each side fails to produce at all. A word the
    // flat path never emits is not an ordering defect — and because the metric
    // aligns by k-th occurrence, one dropped word shifts every later occurrence
    // of the same token, so losses show up magnified as disorder.
    let missing = |ws: &[String]| -> Vec<String> {
        let mut have: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for w in ws {
            *have.entry(w.as_str()).or_default() += 1;
        }
        let mut out = Vec::new();
        for w in &pop {
            match have.get_mut(w.as_str()) {
                Some(n) if *n > 0 => *n -= 1,
                _ => out.push(w.clone()),
            }
        }
        out
    };
    println!(
        "\nPOPPLER words the FLAT path never emits: {:?}",
        missing(&flat)
    );
    println!(
        "\nPOPPLER words the BASE grouping never emits: {:?}",
        missing(&base)
    );

    let mf = order_metrics_words(&flat, &pop);
    let mb = order_metrics_words(&base, &pop);
    println!(
        "\npage {page}\n  flat {mf:?} fidelity={:.4}\n  base {mb:?} fidelity={:.4}",
        mf.in_order as f64 / mf.common.max(1) as f64,
        mb.in_order as f64 / mb.common.max(1) as f64
    );
}

/// Raw fragments of one page (`OXIDIZE_PROBE_PAGE`), with the geometry and the
/// font each one carries. The unit the flat path decides separators on.
#[test]
#[ignore = "manual diagnostic on one page; see the module header"]
fn dump_raw_fragments() {
    let dir = corpus_dir();
    let pdfs = corpus_support::find_pdfs(&dir);
    let path = pdfs.first().expect("point OXIDIZE_PROBE_CORPUS at a PDF");
    let page: usize = std::env::var("OXIDIZE_PROBE_PAGE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let doc = parse_doc(path).expect("document must parse");
    let frags = doc.pages.get(page).expect("page index out of range");
    println!(
        "\n{} page {page}: {} fragments",
        path.display(),
        frags.len()
    );
    println!(
        "{:>8} {:>8} {:>8} {:>6} {:>8}  {:<16} {}",
        "x", "y", "width", "size", "gap", "font", "text"
    );
    let mut prev_right = f64::NAN;
    for f in frags.iter().take(60) {
        println!(
            "{:>8.2} {:>8.2} {:>8.2} {:>6.1} {:>8.2}  {:<16} {:?}",
            f.x,
            f.y,
            f.width,
            f.font_size,
            f.x - prev_right,
            f.font_name.as_deref().unwrap_or("-"),
            f.text
        );
        prev_right = f.x + f.width;
    }
}

fn poppler_page(path: &Path, page: usize) -> Option<String> {
    let o = Command::new("timeout")
        .arg(PER_FILE_TIMEOUT_SECS.to_string())
        .arg("pdftotext")
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg(path)
        .arg("-")
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

#[test]
#[ignore = "manual full-corpus measurement; see the module header"]
fn probe_reading_order_gain_on_corpus() {
    assert!(
        Command::new("pdftotext").arg("-v").output().is_ok(),
        "no oracle, no measurement: pdftotext (poppler) is not on PATH"
    );
    let dir = corpus_dir();
    let pdfs = corpus_support::find_pdfs(&dir);
    assert!(
        !pdfs.is_empty(),
        "no corpus, no measurement: {} holds no PDFs",
        dir.display()
    );

    let vs = variants();
    // Row 0 is the shipping flat path; rows 1.. are the probe's variants.
    let mut totals: Vec<Totals> = (0..vs.len() + 1).map(|_| Totals::default()).collect();
    let mut pop_words_total = 0usize;
    let mut compared = 0usize;
    let mut skips = Skips::default();
    let mut rotated_pages = 0usize;
    let mut total_pages = 0usize;

    // Where the flat path and the probe's un-reordered baseline disagree, per
    // file. The two emit the same words in the same stream order, so any gap
    // between them is NOT about reading order — it is the emission itself, and
    // it is worth naming the documents that carry it.
    let mut flat_vs_base: Vec<(i64, usize, &Path)> = Vec::new();
    let mut per_file_flat: Vec<(String, OrderMetrics)> = Vec::new();

    for pdf in &pdfs {
        let Some(sample) = sample_with_timeout(pdf, &mut skips) else {
            continue;
        };
        compared += 1;
        pop_words_total += sample.metrics[0].pop_words;
        rotated_pages += sample.rotated_pages;
        total_pages += sample.pages;
        flat_vs_base.push((
            sample.flat.misplaced() as i64 - sample.metrics[0].misplaced() as i64,
            sample.flat.pop_words,
            pdf.as_path(),
        ));
        per_file_flat.push((
            pdf.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string(),
            sample.flat,
        ));
        let rows = std::iter::once(&sample.flat).chain(sample.metrics.iter());
        for (t, m) in totals.iter_mut().zip(rows) {
            t.misplaced += m.misplaced();
            t.common += m.common;
            t.in_order += m.in_order;
            t.per_doc.push(m.in_order as f64 / m.common.max(1) as f64);
        }
    }

    // 0 comparisons is an instrument failure, not a perfect score: every rate
    // below would be a division by nothing.
    assert!(
        compared > 0,
        "probe measured NOTHING: 0 of {} files compared ({skips:?})",
        pdfs.len()
    );

    // Per-file flat-path metrics, for diffing one build against another:
    // an aggregate that moves says the corpus changed, not which documents did.
    if let Ok(csv) = std::env::var("OXIDIZE_PROBE_CSV") {
        let mut out = String::from("file,pop_words,common,in_order\n");
        for (name, m) in &per_file_flat {
            out.push_str(&format!(
                "{name},{},{},{}\n",
                m.pop_words, m.common, m.in_order
            ));
        }
        std::fs::write(&csv, out).expect("csv path must be writable");
        println!("per-file flat metrics written to {csv}");
    }

    let rate = |n: usize| n as f64 / pop_words_total.max(1) as f64;
    // Everything is judged against what the extractor SHIPS today, not against
    // the probe's own grouping: a variant that only beats the probe's baseline
    // has beaten a straw man.
    let flat_rate = rate(totals[0].misplaced);

    println!(
        "\nPROBE #448 · corpus={} · files={compared}/{} · poppler_words={pop_words_total} \
         · pages={total_pages} rotated_pages={rotated_pages} ({:.2}%)\n  skips: {skips:?}\n",
        dir.display(),
        pdfs.len(),
        rotated_pages as f64 / total_pages.max(1) as f64 * 100.0
    );
    println!(
        "{:<18} {:>11} {:>10} {:>10} {:>9} {:>8} {:>8} {:>10}  {}",
        "ordering",
        "misplaced",
        "misp_rate",
        "coverage",
        "fidelity",
        "median",
        "p10",
        "vs flat",
        "docs<0.95"
    );
    let labels =
        std::iter::once("FLAT PATH (ships)".to_string()).chain(vs.iter().map(|v| v.label()));
    for (label, t) in labels.zip(&totals) {
        let mut per_doc = t.per_doc.clone();
        per_doc.sort_by(f64::total_cmp);
        let median = per_doc.get(per_doc.len() / 2).copied().unwrap_or(0.0);
        let p10 = per_doc.get(per_doc.len() / 10).copied().unwrap_or(0.0);
        let below = per_doc.iter().filter(|f| **f < 0.95).count();
        let delta = if flat_rate > 0.0 {
            (rate(t.misplaced) - flat_rate) / flat_rate * 100.0
        } else {
            0.0
        };
        println!(
            "{:<18} {:>11} {:>10.6} {:>10.4} {:>9.4} {:>8.4} {:>8.4} {:>9.2}%  {}",
            label,
            t.misplaced,
            rate(t.misplaced),
            t.common as f64 / pop_words_total.max(1) as f64,
            t.in_order as f64 / t.common.max(1) as f64,
            median,
            p10,
            delta,
            below
        );
    }
    println!(
        "\nNegative 'vs flat' = fewer misplaced words than the shipping flat path. \
         'coverage' is aligned words over poppler's: a variant that gains order by \
         losing coverage has bought nothing."
    );

    // The flat path and `base(stream)` emit the same words in the same order,
    // so a difference between them is an emission defect, not an ordering one.
    // Concentration matters: a gap spread over a thousand files is a systematic
    // difference in the separator rules; a gap carried by ten files is ten
    // documents with something specific wrong.
    flat_vs_base.sort_by_key(|(d, _, _)| -*d);
    let carried: i64 = flat_vs_base
        .iter()
        .map(|(d, _, _)| *d)
        .filter(|d| *d > 0)
        .sum();
    let helped = flat_vs_base.iter().filter(|(d, _, _)| *d > 0).count();
    let hurt = flat_vs_base.iter().filter(|(d, _, _)| *d < 0).count();
    println!(
        "\nFLAT vs base(stream) — same words, same order, so this is EMISSION, not ordering:\n  \
         files where grouping wins={helped} loses={hurt} tie={} · words the winners carry={carried}\n  \
         top 12 by words gained:",
        flat_vs_base.len() - helped - hurt
    );
    for (delta, pop, path) in flat_vs_base.iter().take(12) {
        println!(
            "    {delta:>7} of {pop:>6} poppler words  {}",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("?")
        );
    }
}
