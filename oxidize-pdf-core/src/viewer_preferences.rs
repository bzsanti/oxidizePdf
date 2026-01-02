//! Viewer preferences control how the PDF document is displayed in the viewer
//!
//! These preferences are defined in ISO 32000-1:2008 and provide control over
//! the user interface and display behavior when the document is opened.

use crate::objects::{Dictionary, Object};

/// Page layout modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageLayout {
    /// Display one page at a time
    SinglePage,
    /// Display pages in one column
    OneColumn,
    /// Display pages in two columns, odd-numbered pages on the left
    TwoColumnLeft,
    /// Display pages in two columns, odd-numbered pages on the right  
    TwoColumnRight,
    /// Display pages in two columns, cover page displayed alone
    TwoPageLeft,
    /// Display pages in two columns, cover page displayed alone
    TwoPageRight,
}

impl PageLayout {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            PageLayout::SinglePage => "SinglePage",
            PageLayout::OneColumn => "OneColumn",
            PageLayout::TwoColumnLeft => "TwoColumnLeft",
            PageLayout::TwoColumnRight => "TwoColumnRight",
            PageLayout::TwoPageLeft => "TwoPageLeft",
            PageLayout::TwoPageRight => "TwoPageRight",
        }
    }
}

/// Page mode - how to display the document when opened
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageMode {
    /// Neither document outline nor thumbnail images visible
    UseNone,
    /// Document outline visible
    UseOutlines,
    /// Thumbnail images visible
    UseThumbs,
    /// Full-screen mode, hiding all menu bars, window controls, etc.
    FullScreen,
    /// Optional content group panel visible
    UseOC,
    /// Attachments panel visible
    UseAttachments,
}

impl PageMode {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            PageMode::UseNone => "UseNone",
            PageMode::UseOutlines => "UseOutlines",
            PageMode::UseThumbs => "UseThumbs",
            PageMode::FullScreen => "FullScreen",
            PageMode::UseOC => "UseOC",
            PageMode::UseAttachments => "UseAttachments",
        }
    }
}

/// Non-full-screen page mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NonFullScreenPageMode {
    /// Neither document outline nor thumbnail images visible
    UseNone,
    /// Document outline visible
    UseOutlines,
    /// Thumbnail images visible
    UseThumbs,
    /// Optional content group panel visible
    UseOC,
}

impl NonFullScreenPageMode {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            NonFullScreenPageMode::UseNone => "UseNone",
            NonFullScreenPageMode::UseOutlines => "UseOutlines",
            NonFullScreenPageMode::UseThumbs => "UseThumbs",
            NonFullScreenPageMode::UseOC => "UseOC",
        }
    }
}

/// Direction for reading order
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// Left to right
    L2R,
    /// Right to left
    R2L,
}

impl Direction {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            Direction::L2R => "L2R",
            Direction::R2L => "R2L",
        }
    }
}

/// Print scaling options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrintScaling {
    /// No scaling
    None,
    /// Scale to fit page
    AppDefault,
}

impl PrintScaling {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            PrintScaling::None => "None",
            PrintScaling::AppDefault => "AppDefault",
        }
    }
}

/// Duplex printing modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Duplex {
    /// Single-sided printing
    Simplex,
    /// Double-sided printing, flip on short edge
    DuplexFlipShortEdge,
    /// Double-sided printing, flip on long edge
    DuplexFlipLongEdge,
}

impl Duplex {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            Duplex::Simplex => "Simplex",
            Duplex::DuplexFlipShortEdge => "DuplexFlipShortEdge",
            Duplex::DuplexFlipLongEdge => "DuplexFlipLongEdge",
        }
    }
}

/// Viewer preferences for controlling document display
#[derive(Debug, Clone, Default)]
pub struct ViewerPreferences {
    /// Hide the application's tool bars
    pub hide_toolbar: Option<bool>,
    /// Hide the application's menu bar
    pub hide_menubar: Option<bool>,
    /// Hide user interface elements like scroll bars, navigation controls
    pub hide_window_ui: Option<bool>,
    /// Resize document window to fit the size of the first displayed page
    pub fit_window: Option<bool>,
    /// Center the document window on the screen
    pub center_window: Option<bool>,
    /// Display document title instead of filename in window title bar
    pub display_doc_title: Option<bool>,
    /// Page layout to use when document is opened
    pub page_layout: Option<PageLayout>,
    /// How to display document when opened
    pub page_mode: Option<PageMode>,
    /// Page mode after exiting full-screen mode
    pub non_full_screen_page_mode: Option<NonFullScreenPageMode>,
    /// Reading order direction
    pub direction: Option<Direction>,
    /// Area of default page to display when document is opened
    pub view_area: Option<String>,
    /// Area of page to use for clipping when displaying
    pub view_clip: Option<String>,
    /// Area to use for printing
    pub print_area: Option<String>,
    /// Area to use for clipping when printing
    pub print_clip: Option<String>,
    /// Print scaling mode
    pub print_scaling: Option<PrintScaling>,
    /// Duplex printing mode
    pub duplex: Option<Duplex>,
    /// Page ranges for printing
    pub print_page_range: Option<Vec<(u32, u32)>>,
    /// Number of copies
    pub num_copies: Option<u32>,
    /// Whether to pick tray by PDF size
    pub pick_tray_by_pdf_size: Option<bool>,
}

impl ViewerPreferences {
    /// Create new viewer preferences with default values
    pub fn new() -> Self {
        ViewerPreferences::default()
    }

    /// Hide toolbar
    pub fn hide_toolbar(mut self, hide: bool) -> Self {
        self.hide_toolbar = Some(hide);
        self
    }

    /// Hide menubar
    pub fn hide_menubar(mut self, hide: bool) -> Self {
        self.hide_menubar = Some(hide);
        self
    }

    /// Hide window UI elements
    pub fn hide_window_ui(mut self, hide: bool) -> Self {
        self.hide_window_ui = Some(hide);
        self
    }

    /// Fit window to page size
    pub fn fit_window(mut self, fit: bool) -> Self {
        self.fit_window = Some(fit);
        self
    }

    /// Center window on screen
    pub fn center_window(mut self, center: bool) -> Self {
        self.center_window = Some(center);
        self
    }

    /// Display document title in window title bar
    pub fn display_doc_title(mut self, display: bool) -> Self {
        self.display_doc_title = Some(display);
        self
    }

    /// Set page layout
    pub fn page_layout(mut self, layout: PageLayout) -> Self {
        self.page_layout = Some(layout);
        self
    }

    /// Set page mode
    pub fn page_mode(mut self, mode: PageMode) -> Self {
        self.page_mode = Some(mode);
        self
    }

    /// Set non-full-screen page mode
    pub fn non_full_screen_page_mode(mut self, mode: NonFullScreenPageMode) -> Self {
        self.non_full_screen_page_mode = Some(mode);
        self
    }

    /// Set reading direction
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Set print scaling
    pub fn print_scaling(mut self, scaling: PrintScaling) -> Self {
        self.print_scaling = Some(scaling);
        self
    }

    /// Set duplex mode
    pub fn duplex(mut self, duplex: Duplex) -> Self {
        self.duplex = Some(duplex);
        self
    }

    /// Set number of copies for printing
    pub fn num_copies(mut self, copies: u32) -> Self {
        self.num_copies = Some(copies.max(1));
        self
    }

    /// Pick tray by PDF size
    pub fn pick_tray_by_pdf_size(mut self, pick: bool) -> Self {
        self.pick_tray_by_pdf_size = Some(pick);
        self
    }

    /// Add page range for printing
    pub fn add_print_page_range(mut self, start: u32, end: u32) -> Self {
        if self.print_page_range.is_none() {
            self.print_page_range = Some(Vec::new());
        }
        if let Some(ref mut ranges) = self.print_page_range {
            ranges.push((start.min(end), start.max(end)));
        }
        self
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        if let Some(hide) = self.hide_toolbar {
            dict.set("HideToolbar", Object::Boolean(hide));
        }

        if let Some(hide) = self.hide_menubar {
            dict.set("HideMenubar", Object::Boolean(hide));
        }

        if let Some(hide) = self.hide_window_ui {
            dict.set("HideWindowUI", Object::Boolean(hide));
        }

        if let Some(fit) = self.fit_window {
            dict.set("FitWindow", Object::Boolean(fit));
        }

        if let Some(center) = self.center_window {
            dict.set("CenterWindow", Object::Boolean(center));
        }

        if let Some(display) = self.display_doc_title {
            dict.set("DisplayDocTitle", Object::Boolean(display));
        }

        if let Some(layout) = self.page_layout {
            dict.set("PageLayout", Object::Name(layout.to_pdf_name().to_string()));
        }

        if let Some(mode) = self.page_mode {
            dict.set("PageMode", Object::Name(mode.to_pdf_name().to_string()));
        }

        if let Some(mode) = self.non_full_screen_page_mode {
            dict.set(
                "NonFullScreenPageMode",
                Object::Name(mode.to_pdf_name().to_string()),
            );
        }

        if let Some(direction) = self.direction {
            dict.set(
                "Direction",
                Object::Name(direction.to_pdf_name().to_string()),
            );
        }

        if let Some(ref area) = self.view_area {
            dict.set("ViewArea", Object::Name(area.clone()));
        }

        if let Some(ref clip) = self.view_clip {
            dict.set("ViewClip", Object::Name(clip.clone()));
        }

        if let Some(ref area) = self.print_area {
            dict.set("PrintArea", Object::Name(area.clone()));
        }

        if let Some(ref clip) = self.print_clip {
            dict.set("PrintClip", Object::Name(clip.clone()));
        }

        if let Some(scaling) = self.print_scaling {
            dict.set(
                "PrintScaling",
                Object::Name(scaling.to_pdf_name().to_string()),
            );
        }

        if let Some(duplex) = self.duplex {
            dict.set("Duplex", Object::Name(duplex.to_pdf_name().to_string()));
        }

        if let Some(ref ranges) = self.print_page_range {
            let range_array: Vec<Object> = ranges
                .iter()
                .flat_map(|(start, end)| {
                    vec![Object::Integer(*start as i64), Object::Integer(*end as i64)]
                })
                .collect();
            dict.set("PrintPageRange", Object::Array(range_array));
        }

        if let Some(copies) = self.num_copies {
            dict.set("NumCopies", Object::Integer(copies as i64));
        }

        if let Some(pick) = self.pick_tray_by_pdf_size {
            dict.set("PickTrayByPDFSize", Object::Boolean(pick));
        }

        dict
    }

    // Convenience constructors

    /// Create preferences for presentation mode
    pub fn presentation() -> Self {
        ViewerPreferences::new()
            .page_mode(PageMode::FullScreen)
            .hide_toolbar(true)
            .hide_menubar(true)
            .hide_window_ui(true)
            .fit_window(true)
            .center_window(true)
    }

    /// Create preferences for reading mode
    pub fn reading() -> Self {
        ViewerPreferences::new()
            .page_layout(PageLayout::TwoColumnRight)
            .page_mode(PageMode::UseOutlines)
            .fit_window(true)
            .display_doc_title(true)
    }

    /// Create preferences for printing
    pub fn printing() -> Self {
        ViewerPreferences::new()
            .print_scaling(PrintScaling::None)
            .duplex(Duplex::DuplexFlipLongEdge)
            .pick_tray_by_pdf_size(true)
    }

    /// Create minimal UI preferences
    pub fn minimal_ui() -> Self {
        ViewerPreferences::new()
            .hide_toolbar(true)
            .hide_menubar(true)
            .center_window(true)
            .fit_window(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_layout_names() {
        assert_eq!(PageLayout::SinglePage.to_pdf_name(), "SinglePage");
        assert_eq!(PageLayout::OneColumn.to_pdf_name(), "OneColumn");
        assert_eq!(PageLayout::TwoColumnLeft.to_pdf_name(), "TwoColumnLeft");
        assert_eq!(PageLayout::TwoColumnRight.to_pdf_name(), "TwoColumnRight");
        assert_eq!(PageLayout::TwoPageLeft.to_pdf_name(), "TwoPageLeft");
        assert_eq!(PageLayout::TwoPageRight.to_pdf_name(), "TwoPageRight");
    }

    #[test]
    fn test_page_mode_names() {
        assert_eq!(PageMode::UseNone.to_pdf_name(), "UseNone");
        assert_eq!(PageMode::UseOutlines.to_pdf_name(), "UseOutlines");
        assert_eq!(PageMode::UseThumbs.to_pdf_name(), "UseThumbs");
        assert_eq!(PageMode::FullScreen.to_pdf_name(), "FullScreen");
        assert_eq!(PageMode::UseOC.to_pdf_name(), "UseOC");
        assert_eq!(PageMode::UseAttachments.to_pdf_name(), "UseAttachments");
    }

    #[test]
    fn test_basic_preferences() {
        let prefs = ViewerPreferences::new()
            .hide_toolbar(true)
            .hide_menubar(true)
            .fit_window(true);

        let dict = prefs.to_dict();
        assert_eq!(dict.get("HideToolbar"), Some(&Object::Boolean(true)));
        assert_eq!(dict.get("HideMenubar"), Some(&Object::Boolean(true)));
        assert_eq!(dict.get("FitWindow"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_page_layout_preference() {
        let prefs = ViewerPreferences::new().page_layout(PageLayout::TwoColumnLeft);

        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("PageLayout"),
            Some(&Object::Name("TwoColumnLeft".to_string()))
        );
    }

    #[test]
    fn test_print_preferences() {
        let prefs = ViewerPreferences::new()
            .print_scaling(PrintScaling::None)
            .duplex(Duplex::DuplexFlipLongEdge)
            .num_copies(3);

        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("PrintScaling"),
            Some(&Object::Name("None".to_string()))
        );
        assert_eq!(
            dict.get("Duplex"),
            Some(&Object::Name("DuplexFlipLongEdge".to_string()))
        );
        assert_eq!(dict.get("NumCopies"), Some(&Object::Integer(3)));
    }

    #[test]
    fn test_print_page_ranges() {
        let prefs = ViewerPreferences::new()
            .add_print_page_range(1, 5)
            .add_print_page_range(10, 15);

        let dict = prefs.to_dict();
        if let Some(Object::Array(ranges)) = dict.get("PrintPageRange") {
            assert_eq!(ranges.len(), 4); // Two ranges, each with start and end
            assert_eq!(ranges[0], Object::Integer(1));
            assert_eq!(ranges[1], Object::Integer(5));
            assert_eq!(ranges[2], Object::Integer(10));
            assert_eq!(ranges[3], Object::Integer(15));
        } else {
            panic!("Expected PrintPageRange array");
        }
    }

    #[test]
    fn test_convenience_constructors() {
        let presentation = ViewerPreferences::presentation();
        assert_eq!(presentation.page_mode, Some(PageMode::FullScreen));
        assert_eq!(presentation.hide_toolbar, Some(true));

        let reading = ViewerPreferences::reading();
        assert_eq!(reading.page_layout, Some(PageLayout::TwoColumnRight));
        assert_eq!(reading.page_mode, Some(PageMode::UseOutlines));

        let printing = ViewerPreferences::printing();
        assert_eq!(printing.print_scaling, Some(PrintScaling::None));
        assert_eq!(printing.duplex, Some(Duplex::DuplexFlipLongEdge));

        let minimal = ViewerPreferences::minimal_ui();
        assert_eq!(minimal.hide_toolbar, Some(true));
        assert_eq!(minimal.hide_menubar, Some(true));
    }

    #[test]
    fn test_num_copies_bounds() {
        let prefs = ViewerPreferences::new().num_copies(0);
        assert_eq!(prefs.num_copies, Some(1)); // Should be clamped to minimum 1
    }

    #[test]
    fn test_direction() {
        assert_eq!(Direction::L2R.to_pdf_name(), "L2R");
        assert_eq!(Direction::R2L.to_pdf_name(), "R2L");
    }

    #[test]
    fn test_print_scaling() {
        assert_eq!(PrintScaling::None.to_pdf_name(), "None");
        assert_eq!(PrintScaling::AppDefault.to_pdf_name(), "AppDefault");
    }

    #[test]
    fn test_duplex_modes() {
        assert_eq!(Duplex::Simplex.to_pdf_name(), "Simplex");
        assert_eq!(
            Duplex::DuplexFlipShortEdge.to_pdf_name(),
            "DuplexFlipShortEdge"
        );
        assert_eq!(
            Duplex::DuplexFlipLongEdge.to_pdf_name(),
            "DuplexFlipLongEdge"
        );
    }

    #[test]
    fn test_empty_preferences() {
        let prefs = ViewerPreferences::new();
        let dict = prefs.to_dict();
        assert!(dict.is_empty()); // No preferences set, should result in empty dictionary
    }

    #[test]
    fn test_non_full_screen_page_mode_names() {
        assert_eq!(NonFullScreenPageMode::UseNone.to_pdf_name(), "UseNone");
        assert_eq!(
            NonFullScreenPageMode::UseOutlines.to_pdf_name(),
            "UseOutlines"
        );
        assert_eq!(NonFullScreenPageMode::UseThumbs.to_pdf_name(), "UseThumbs");
        assert_eq!(NonFullScreenPageMode::UseOC.to_pdf_name(), "UseOC");
    }

    #[test]
    fn test_non_full_screen_page_mode_to_dict() {
        let prefs =
            ViewerPreferences::new().non_full_screen_page_mode(NonFullScreenPageMode::UseOutlines);
        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("NonFullScreenPageMode"),
            Some(&Object::Name("UseOutlines".to_string()))
        );
    }

    #[test]
    fn test_direction_to_dict() {
        let prefs_l2r = ViewerPreferences::new().direction(Direction::L2R);
        let dict_l2r = prefs_l2r.to_dict();
        assert_eq!(
            dict_l2r.get("Direction"),
            Some(&Object::Name("L2R".to_string()))
        );

        let prefs_r2l = ViewerPreferences::new().direction(Direction::R2L);
        let dict_r2l = prefs_r2l.to_dict();
        assert_eq!(
            dict_r2l.get("Direction"),
            Some(&Object::Name("R2L".to_string()))
        );
    }

    #[test]
    fn test_view_area_and_clip() {
        let mut prefs = ViewerPreferences::new();
        prefs.view_area = Some("MediaBox".to_string());
        prefs.view_clip = Some("CropBox".to_string());

        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("ViewArea"),
            Some(&Object::Name("MediaBox".to_string()))
        );
        assert_eq!(
            dict.get("ViewClip"),
            Some(&Object::Name("CropBox".to_string()))
        );
    }

    #[test]
    fn test_print_area_and_clip() {
        let mut prefs = ViewerPreferences::new();
        prefs.print_area = Some("BleedBox".to_string());
        prefs.print_clip = Some("TrimBox".to_string());

        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("PrintArea"),
            Some(&Object::Name("BleedBox".to_string()))
        );
        assert_eq!(
            dict.get("PrintClip"),
            Some(&Object::Name("TrimBox".to_string()))
        );
    }

    #[test]
    fn test_pick_tray_by_pdf_size_to_dict() {
        let prefs = ViewerPreferences::new().pick_tray_by_pdf_size(true);
        let dict = prefs.to_dict();
        assert_eq!(dict.get("PickTrayByPDFSize"), Some(&Object::Boolean(true)));

        let prefs_false = ViewerPreferences::new().pick_tray_by_pdf_size(false);
        let dict_false = prefs_false.to_dict();
        assert_eq!(
            dict_false.get("PickTrayByPDFSize"),
            Some(&Object::Boolean(false))
        );
    }

    #[test]
    fn test_hide_window_ui_to_dict() {
        let prefs = ViewerPreferences::new().hide_window_ui(true);
        let dict = prefs.to_dict();
        assert_eq!(dict.get("HideWindowUI"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_center_window_to_dict() {
        let prefs = ViewerPreferences::new().center_window(true);
        let dict = prefs.to_dict();
        assert_eq!(dict.get("CenterWindow"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_display_doc_title_to_dict() {
        let prefs = ViewerPreferences::new().display_doc_title(true);
        let dict = prefs.to_dict();
        assert_eq!(dict.get("DisplayDocTitle"), Some(&Object::Boolean(true)));
    }

    #[test]
    fn test_page_mode_to_dict() {
        let prefs = ViewerPreferences::new().page_mode(PageMode::UseAttachments);
        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("PageMode"),
            Some(&Object::Name("UseAttachments".to_string()))
        );
    }

    #[test]
    fn test_print_page_range_reversed() {
        // Test that reversed ranges are handled correctly (start > end)
        let prefs = ViewerPreferences::new().add_print_page_range(10, 5);
        let dict = prefs.to_dict();
        if let Some(Object::Array(ranges)) = dict.get("PrintPageRange") {
            assert_eq!(ranges[0], Object::Integer(5)); // min(10, 5) = 5
            assert_eq!(ranges[1], Object::Integer(10)); // max(10, 5) = 10
        } else {
            panic!("Expected PrintPageRange array");
        }
    }

    #[test]
    fn test_all_page_layouts_to_dict() {
        let layouts = [
            (PageLayout::SinglePage, "SinglePage"),
            (PageLayout::OneColumn, "OneColumn"),
            (PageLayout::TwoColumnLeft, "TwoColumnLeft"),
            (PageLayout::TwoColumnRight, "TwoColumnRight"),
            (PageLayout::TwoPageLeft, "TwoPageLeft"),
            (PageLayout::TwoPageRight, "TwoPageRight"),
        ];

        for (layout, expected_name) in layouts {
            let prefs = ViewerPreferences::new().page_layout(layout);
            let dict = prefs.to_dict();
            assert_eq!(
                dict.get("PageLayout"),
                Some(&Object::Name(expected_name.to_string()))
            );
        }
    }

    #[test]
    fn test_print_scaling_app_default_to_dict() {
        let prefs = ViewerPreferences::new().print_scaling(PrintScaling::AppDefault);
        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("PrintScaling"),
            Some(&Object::Name("AppDefault".to_string()))
        );
    }

    #[test]
    fn test_duplex_simplex_to_dict() {
        let prefs = ViewerPreferences::new().duplex(Duplex::Simplex);
        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("Duplex"),
            Some(&Object::Name("Simplex".to_string()))
        );
    }

    #[test]
    fn test_duplex_flip_short_edge_to_dict() {
        let prefs = ViewerPreferences::new().duplex(Duplex::DuplexFlipShortEdge);
        let dict = prefs.to_dict();
        assert_eq!(
            dict.get("Duplex"),
            Some(&Object::Name("DuplexFlipShortEdge".to_string()))
        );
    }

    #[test]
    fn test_full_preferences_to_dict() {
        let mut prefs = ViewerPreferences::new()
            .hide_toolbar(true)
            .hide_menubar(true)
            .hide_window_ui(true)
            .fit_window(true)
            .center_window(true)
            .display_doc_title(true)
            .page_layout(PageLayout::TwoColumnRight)
            .page_mode(PageMode::UseOutlines)
            .non_full_screen_page_mode(NonFullScreenPageMode::UseThumbs)
            .direction(Direction::R2L)
            .print_scaling(PrintScaling::None)
            .duplex(Duplex::DuplexFlipLongEdge)
            .num_copies(2)
            .pick_tray_by_pdf_size(true)
            .add_print_page_range(1, 10);

        prefs.view_area = Some("MediaBox".to_string());
        prefs.view_clip = Some("CropBox".to_string());
        prefs.print_area = Some("BleedBox".to_string());
        prefs.print_clip = Some("TrimBox".to_string());

        let dict = prefs.to_dict();

        // Verify all fields are present
        assert!(dict.contains_key("HideToolbar"));
        assert!(dict.contains_key("HideMenubar"));
        assert!(dict.contains_key("HideWindowUI"));
        assert!(dict.contains_key("FitWindow"));
        assert!(dict.contains_key("CenterWindow"));
        assert!(dict.contains_key("DisplayDocTitle"));
        assert!(dict.contains_key("PageLayout"));
        assert!(dict.contains_key("PageMode"));
        assert!(dict.contains_key("NonFullScreenPageMode"));
        assert!(dict.contains_key("Direction"));
        assert!(dict.contains_key("ViewArea"));
        assert!(dict.contains_key("ViewClip"));
        assert!(dict.contains_key("PrintArea"));
        assert!(dict.contains_key("PrintClip"));
        assert!(dict.contains_key("PrintScaling"));
        assert!(dict.contains_key("Duplex"));
        assert!(dict.contains_key("PrintPageRange"));
        assert!(dict.contains_key("NumCopies"));
        assert!(dict.contains_key("PickTrayByPDFSize"));
    }
}
