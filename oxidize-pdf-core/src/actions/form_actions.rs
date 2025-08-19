//! Form-related actions (Submit-Form, Reset-Form, Import-Data) per ISO 32000-1 §12.7.5

use crate::objects::{Dictionary, Object};

/// Submit-form action flags
#[derive(Debug, Clone, Copy)]
pub struct SubmitFormFlags {
    /// Include/exclude fields
    pub include: bool,
    /// Submit as HTML form instead of FDF
    pub html: bool,
    /// Include coordinates of mouse click
    pub coordinates: bool,
    /// Submit as XML instead of FDF
    pub xml: bool,
    /// Include annotations
    pub include_annotations: bool,
    /// Submit PDF file instead of FDF
    pub pdf: bool,
    /// Convert dates to standard format
    pub canonical_dates: bool,
    /// Include only user-entered data
    pub exclude_non_user: bool,
    /// Include field names without values
    pub include_no_value_fields: bool,
    /// Export as Windows code page
    pub export_format: bool,
}

impl Default for SubmitFormFlags {
    fn default() -> Self {
        Self {
            include: true,
            html: false,
            coordinates: false,
            xml: false,
            include_annotations: false,
            pdf: false,
            canonical_dates: false,
            exclude_non_user: false,
            include_no_value_fields: false,
            export_format: false,
        }
    }
}

impl SubmitFormFlags {
    /// Convert to integer flags value
    pub fn to_flags(&self) -> i32 {
        let mut flags = 0;
        if !self.include {
            flags |= 1 << 0;
        }
        if self.html {
            flags |= 1 << 2;
        }
        if self.coordinates {
            flags |= 1 << 4;
        }
        if self.xml {
            flags |= 1 << 5;
        }
        if self.include_annotations {
            flags |= 1 << 7;
        }
        if self.pdf {
            flags |= 1 << 8;
        }
        if self.canonical_dates {
            flags |= 1 << 9;
        }
        if self.exclude_non_user {
            flags |= 1 << 10;
        }
        if self.include_no_value_fields {
            flags |= 1 << 11;
        }
        if self.export_format {
            flags |= 1 << 12;
        }
        flags
    }
}

/// Submit-form action - submit form data to a URL
#[derive(Debug, Clone)]
pub struct SubmitFormAction {
    /// URL to submit to
    pub url: String,
    /// Fields to include/exclude (empty means all)
    pub fields: Vec<String>,
    /// Submission flags
    pub flags: SubmitFormFlags,
    /// Character set for submission
    pub charset: Option<String>,
}

impl SubmitFormAction {
    /// Create new submit-form action
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            fields: Vec::new(),
            flags: SubmitFormFlags::default(),
            charset: None,
        }
    }

    /// Submit as HTML form
    pub fn as_html(mut self) -> Self {
        self.flags.html = true;
        self
    }

    /// Submit as XML
    pub fn as_xml(mut self) -> Self {
        self.flags.xml = true;
        self
    }

    /// Submit entire PDF
    pub fn as_pdf(mut self) -> Self {
        self.flags.pdf = true;
        self
    }

    /// Include specific fields only
    pub fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self.flags.include = true;
        self
    }

    /// Exclude specific fields
    pub fn excluding_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self.flags.include = false;
        self
    }

    /// Set character set
    pub fn with_charset(mut self, charset: impl Into<String>) -> Self {
        self.charset = Some(charset.into());
        self
    }

    /// Include mouse click coordinates
    pub fn with_coordinates(mut self) -> Self {
        self.flags.coordinates = true;
        self
    }

    /// Include annotations
    pub fn with_annotations(mut self) -> Self {
        self.flags.include_annotations = true;
        self
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("SubmitForm".to_string()));
        dict.set("F", Object::String(self.url.clone()));

        // Add fields array if specified
        if !self.fields.is_empty() {
            let fields_array: Vec<Object> = self
                .fields
                .iter()
                .map(|f| Object::String(f.clone()))
                .collect();
            dict.set("Fields", Object::Array(fields_array));
        }

        // Add flags
        dict.set("Flags", Object::Integer(self.flags.to_flags() as i64));

        // Add charset if specified
        if let Some(charset) = &self.charset {
            dict.set("CharSet", Object::String(charset.clone()));
        }

        dict
    }
}

/// Reset-form action - reset form fields to default values
#[derive(Debug, Clone)]
pub struct ResetFormAction {
    /// Fields to reset (empty means all)
    pub fields: Vec<String>,
    /// Whether to include (true) or exclude (false) the specified fields
    pub include: bool,
}

impl ResetFormAction {
    /// Create new reset-form action (resets all fields)
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            include: true,
        }
    }

    /// Reset specific fields only
    pub fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self.include = true;
        self
    }

    /// Reset all fields except specified ones
    pub fn excluding_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self.include = false;
        self
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("ResetForm".to_string()));

        // Add fields array if specified
        if !self.fields.is_empty() {
            let fields_array: Vec<Object> = self
                .fields
                .iter()
                .map(|f| Object::String(f.clone()))
                .collect();
            dict.set("Fields", Object::Array(fields_array));
        }

        // Add flags (bit 0: 0=include, 1=exclude)
        let flags = if self.include { 0 } else { 1 };
        dict.set("Flags", Object::Integer(flags));

        dict
    }
}

impl Default for ResetFormAction {
    fn default() -> Self {
        Self::new()
    }
}

/// Import-data action - import FDF or XFDF data
#[derive(Debug, Clone)]
pub struct ImportDataAction {
    /// File specification for the data file
    pub file: String,
}

impl ImportDataAction {
    /// Create new import-data action
    pub fn new(file: impl Into<String>) -> Self {
        Self { file: file.into() }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("ImportData".to_string()));
        dict.set("F", Object::String(self.file.clone()));
        dict
    }
}

/// Hide action - show or hide form fields and annotations
#[derive(Debug, Clone)]
pub struct HideAction {
    /// Target annotations/fields to hide
    pub targets: Vec<String>,
    /// Whether to hide (true) or show (false)
    pub hide: bool,
}

impl HideAction {
    /// Create new hide action
    pub fn new(targets: Vec<String>) -> Self {
        Self {
            targets,
            hide: true,
        }
    }

    /// Hide the targets
    pub fn hide(mut self) -> Self {
        self.hide = true;
        self
    }

    /// Show the targets
    pub fn show(mut self) -> Self {
        self.hide = false;
        self
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("Hide".to_string()));

        // Add targets
        if self.targets.len() == 1 {
            dict.set("T", Object::String(self.targets[0].clone()));
        } else {
            let targets_array: Vec<Object> = self
                .targets
                .iter()
                .map(|t| Object::String(t.clone()))
                .collect();
            dict.set("T", Object::Array(targets_array));
        }

        dict.set("H", Object::Boolean(self.hide));

        dict
    }
}

/// Set-OCG-State action - set the state of optional content groups
#[derive(Debug, Clone)]
pub struct SetOCGStateAction {
    /// State changes to apply
    pub state_changes: Vec<OCGStateChange>,
    /// Whether to preserve radio button relationships
    pub preserve_rb: bool,
}

/// OCG state change specification
#[derive(Debug, Clone)]
pub enum OCGStateChange {
    /// Turn on OCGs
    On(Vec<String>),
    /// Turn off OCGs
    Off(Vec<String>),
    /// Toggle OCGs
    Toggle(Vec<String>),
}

impl SetOCGStateAction {
    /// Create new set-OCG-state action
    pub fn new() -> Self {
        Self {
            state_changes: Vec::new(),
            preserve_rb: true,
        }
    }

    /// Turn on specified OCGs
    pub fn turn_on(mut self, ocgs: Vec<String>) -> Self {
        self.state_changes.push(OCGStateChange::On(ocgs));
        self
    }

    /// Turn off specified OCGs
    pub fn turn_off(mut self, ocgs: Vec<String>) -> Self {
        self.state_changes.push(OCGStateChange::Off(ocgs));
        self
    }

    /// Toggle specified OCGs
    pub fn toggle(mut self, ocgs: Vec<String>) -> Self {
        self.state_changes.push(OCGStateChange::Toggle(ocgs));
        self
    }

    /// Set whether to preserve radio button relationships
    pub fn preserve_radio_buttons(mut self, preserve: bool) -> Self {
        self.preserve_rb = preserve;
        self
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("SetOCGState".to_string()));

        // Build state array
        let mut state_array = Vec::new();
        for change in &self.state_changes {
            match change {
                OCGStateChange::On(ocgs) => {
                    state_array.push(Object::Name("ON".to_string()));
                    for ocg in ocgs {
                        state_array.push(Object::String(ocg.clone()));
                    }
                }
                OCGStateChange::Off(ocgs) => {
                    state_array.push(Object::Name("OFF".to_string()));
                    for ocg in ocgs {
                        state_array.push(Object::String(ocg.clone()));
                    }
                }
                OCGStateChange::Toggle(ocgs) => {
                    state_array.push(Object::Name("Toggle".to_string()));
                    for ocg in ocgs {
                        state_array.push(Object::String(ocg.clone()));
                    }
                }
            }
        }

        dict.set("State", Object::Array(state_array));
        dict.set("PreserveRB", Object::Boolean(self.preserve_rb));

        dict
    }
}

impl Default for SetOCGStateAction {
    fn default() -> Self {
        Self::new()
    }
}

/// JavaScript action - execute JavaScript code
#[derive(Debug, Clone)]
pub struct JavaScriptAction {
    /// JavaScript code to execute
    pub script: String,
}

impl JavaScriptAction {
    /// Create new JavaScript action
    pub fn new(script: impl Into<String>) -> Self {
        Self {
            script: script.into(),
        }
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("JavaScript".to_string()));
        dict.set("JS", Object::String(self.script.clone()));
        dict
    }
}

/// Sound action - play a sound
#[derive(Debug, Clone)]
pub struct SoundAction {
    /// Sound object name
    pub sound: String,
    /// Volume (0.0 to 1.0)
    pub volume: f64,
    /// Whether to play synchronously
    pub synchronous: bool,
    /// Whether to repeat
    pub repeat: bool,
    /// Whether to mix with other sounds
    pub mix: bool,
}

impl SoundAction {
    /// Create new sound action
    pub fn new(sound: impl Into<String>) -> Self {
        Self {
            sound: sound.into(),
            volume: 1.0,
            synchronous: false,
            repeat: false,
            mix: false,
        }
    }

    /// Set volume (0.0 to 1.0)
    pub fn with_volume(mut self, volume: f64) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Play synchronously
    pub fn synchronous(mut self) -> Self {
        self.synchronous = true;
        self
    }

    /// Repeat the sound
    pub fn repeat(mut self) -> Self {
        self.repeat = true;
        self
    }

    /// Mix with other sounds
    pub fn mix(mut self) -> Self {
        self.mix = true;
        self
    }

    /// Convert to dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Action".to_string()));
        dict.set("S", Object::Name("Sound".to_string()));
        dict.set("Sound", Object::String(self.sound.clone()));
        dict.set("Volume", Object::Real(self.volume));
        dict.set("Synchronous", Object::Boolean(self.synchronous));
        dict.set("Repeat", Object::Boolean(self.repeat));
        dict.set("Mix", Object::Boolean(self.mix));
        dict
    }
}
