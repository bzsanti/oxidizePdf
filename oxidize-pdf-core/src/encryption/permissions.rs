//! PDF permissions according to ISO 32000-1 Table 22

/// Permission flags for encrypted PDFs
#[derive(Debug, Clone, Copy)]
pub struct Permissions {
    /// Permission bits (32-bit value)
    bits: u32,
}

/// Individual permission flags
#[derive(Debug, Clone, Copy)]
pub struct PermissionFlags {
    /// Print the document
    pub print: bool,
    /// Modify document contents
    pub modify_contents: bool,
    /// Copy text and graphics
    pub copy: bool,
    /// Add or modify text annotations
    pub modify_annotations: bool,
    /// Fill in form fields
    pub fill_forms: bool,
    /// Extract text and graphics (accessibility)
    pub accessibility: bool,
    /// Assemble the document (insert, rotate, delete pages)
    pub assemble: bool,
    /// Print in high quality
    pub print_high_quality: bool,
}

impl Default for PermissionFlags {
    fn default() -> Self {
        Self {
            print: false,
            modify_contents: false,
            copy: false,
            modify_annotations: false,
            fill_forms: false,
            accessibility: true, // Usually allowed for accessibility
            assemble: false,
            print_high_quality: false,
        }
    }
}

impl Permissions {
    /// Create new permissions with all operations prohibited
    pub fn new() -> Self {
        // Bits 1-2 must be 0, bits 7-8 reserved (1), bits 13-32 must be 1
        // This gives us 0xFFFFF0C0 as base
        Self { bits: 0xFFFFF0C0 }
    }

    /// Create permissions from flags
    pub fn from_flags(flags: PermissionFlags) -> Self {
        let mut perm = Self::new();

        if flags.print {
            perm.set_print(true);
        }
        if flags.modify_contents {
            perm.set_modify_contents(true);
        }
        if flags.copy {
            perm.set_copy(true);
        }
        if flags.modify_annotations {
            perm.set_modify_annotations(true);
        }
        if flags.fill_forms {
            perm.set_fill_forms(true);
        }
        if flags.accessibility {
            perm.set_accessibility(true);
        }
        if flags.assemble {
            perm.set_assemble(true);
        }
        if flags.print_high_quality {
            perm.set_print_high_quality(true);
        }

        perm
    }

    /// Create permissions allowing all operations
    pub fn all() -> Self {
        let mut perm = Self::new();
        perm.bits |= 0x0F3C; // Set all permission bits
        perm
    }

    /// Get raw permission bits
    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// Create from raw bits
    pub fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Allow/disallow printing (bit 3)
    pub fn set_print(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 2;
        } else {
            self.bits &= !(1 << 2);
        }
        self
    }

    /// Check if printing is allowed
    pub fn can_print(&self) -> bool {
        (self.bits & (1 << 2)) != 0
    }

    /// Allow/disallow modifying contents (bit 4)
    pub fn set_modify_contents(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 3;
        } else {
            self.bits &= !(1 << 3);
        }
        self
    }

    /// Check if modifying contents is allowed
    pub fn can_modify_contents(&self) -> bool {
        (self.bits & (1 << 3)) != 0
    }

    /// Allow/disallow copying (bit 5)
    pub fn set_copy(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 4;
        } else {
            self.bits &= !(1 << 4);
        }
        self
    }

    /// Check if copying is allowed
    pub fn can_copy(&self) -> bool {
        (self.bits & (1 << 4)) != 0
    }

    /// Allow/disallow modifying annotations (bit 6)
    pub fn set_modify_annotations(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 5;
        } else {
            self.bits &= !(1 << 5);
        }
        self
    }

    /// Check if modifying annotations is allowed
    pub fn can_modify_annotations(&self) -> bool {
        (self.bits & (1 << 5)) != 0
    }

    /// Allow/disallow filling forms (bit 9)
    pub fn set_fill_forms(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 8;
        } else {
            self.bits &= !(1 << 8);
        }
        self
    }

    /// Check if filling forms is allowed
    pub fn can_fill_forms(&self) -> bool {
        (self.bits & (1 << 8)) != 0
    }

    /// Allow/disallow accessibility (bit 10)
    pub fn set_accessibility(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 9;
        } else {
            self.bits &= !(1 << 9);
        }
        self
    }

    /// Check if accessibility is allowed
    pub fn can_access_for_accessibility(&self) -> bool {
        (self.bits & (1 << 9)) != 0
    }

    /// Allow/disallow document assembly (bit 11)
    pub fn set_assemble(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 10;
        } else {
            self.bits &= !(1 << 10);
        }
        self
    }

    /// Check if document assembly is allowed
    pub fn can_assemble(&self) -> bool {
        (self.bits & (1 << 10)) != 0
    }

    /// Allow/disallow high quality printing (bit 12)
    pub fn set_print_high_quality(&mut self, allow: bool) -> &mut Self {
        if allow {
            self.bits |= 1 << 11;
        } else {
            self.bits &= !(1 << 11);
        }
        self
    }

    /// Check if high quality printing is allowed
    pub fn can_print_high_quality(&self) -> bool {
        (self.bits & (1 << 11)) != 0
    }

    /// Get permission flags
    pub fn flags(&self) -> PermissionFlags {
        PermissionFlags {
            print: self.can_print(),
            modify_contents: self.can_modify_contents(),
            copy: self.can_copy(),
            modify_annotations: self.can_modify_annotations(),
            fill_forms: self.can_fill_forms(),
            accessibility: self.can_access_for_accessibility(),
            assemble: self.can_assemble(),
            print_high_quality: self.can_print_high_quality(),
        }
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions_new() {
        let perm = Permissions::new();
        assert_eq!(perm.bits(), 0xFFFFF0C0);

        // All operations should be prohibited by default
        assert!(!perm.can_print());
        assert!(!perm.can_modify_contents());
        assert!(!perm.can_copy());
    }

    #[test]
    fn test_permissions_all() {
        let perm = Permissions::all();

        assert!(perm.can_print());
        assert!(perm.can_modify_contents());
        assert!(perm.can_copy());
        assert!(perm.can_modify_annotations());
        assert!(perm.can_fill_forms());
        assert!(perm.can_access_for_accessibility());
        assert!(perm.can_assemble());
        assert!(perm.can_print_high_quality());
    }

    #[test]
    fn test_permission_flags() {
        let mut perm = Permissions::new();

        // Test individual permissions
        perm.set_print(true);
        assert!(perm.can_print());
        assert_eq!(perm.bits() & (1 << 2), 1 << 2);

        perm.set_copy(true);
        assert!(perm.can_copy());
        assert_eq!(perm.bits() & (1 << 4), 1 << 4);

        perm.set_print(false);
        assert!(!perm.can_print());
        assert_eq!(perm.bits() & (1 << 2), 0);
    }

    #[test]
    fn test_from_flags() {
        let flags = PermissionFlags {
            print: true,
            modify_contents: false,
            copy: true,
            modify_annotations: false,
            fill_forms: true,
            accessibility: true,
            assemble: false,
            print_high_quality: true,
        };

        let perm = Permissions::from_flags(flags);
        assert!(perm.can_print());
        assert!(!perm.can_modify_contents());
        assert!(perm.can_copy());
        assert!(!perm.can_modify_annotations());
        assert!(perm.can_fill_forms());
        assert!(perm.can_access_for_accessibility());
        assert!(!perm.can_assemble());
        assert!(perm.can_print_high_quality());
    }

    #[test]
    fn test_get_flags() {
        let mut perm = Permissions::new();
        perm.set_print(true).set_copy(true).set_fill_forms(true);

        let flags = perm.flags();
        assert!(flags.print);
        assert!(flags.copy);
        assert!(flags.fill_forms);
        assert!(!flags.modify_contents);
        assert!(!flags.modify_annotations);
    }

    #[test]
    fn test_permissions_default() {
        let perm = Permissions::default();
        assert_eq!(perm.bits(), Permissions::new().bits());
    }

    #[test]
    fn test_permissions_bits_manipulation() {
        let mut perm = Permissions::new();

        // Test setting all permissions individually
        perm.set_print(true);
        perm.set_modify_contents(true);
        perm.set_copy(true);
        perm.set_modify_annotations(true);
        perm.set_fill_forms(true);
        perm.set_accessibility(true);
        perm.set_assemble(true);
        perm.set_print_high_quality(true);

        assert!(perm.can_print());
        assert!(perm.can_modify_contents());
        assert!(perm.can_copy());
        assert!(perm.can_modify_annotations());
        assert!(perm.can_fill_forms());
        assert!(perm.can_access_for_accessibility());
        assert!(perm.can_assemble());
        assert!(perm.can_print_high_quality());
    }

    #[test]
    fn test_permissions_bits_clearing() {
        let mut perm = Permissions::all();

        // Test clearing all permissions individually
        perm.set_print(false);
        perm.set_modify_contents(false);
        perm.set_copy(false);
        perm.set_modify_annotations(false);
        perm.set_fill_forms(false);
        perm.set_accessibility(false);
        perm.set_assemble(false);
        perm.set_print_high_quality(false);

        assert!(!perm.can_print());
        assert!(!perm.can_modify_contents());
        assert!(!perm.can_copy());
        assert!(!perm.can_modify_annotations());
        assert!(!perm.can_fill_forms());
        assert!(!perm.can_access_for_accessibility());
        assert!(!perm.can_assemble());
        assert!(!perm.can_print_high_quality());
    }

    #[test]
    fn test_permissions_chaining() {
        let mut perm = Permissions::new();
        perm.set_print(true).set_copy(true).set_fill_forms(true);

        assert!(perm.can_print());
        assert!(perm.can_copy());
        assert!(perm.can_fill_forms());
        assert!(!perm.can_modify_contents());
    }

    #[test]
    fn test_permission_flags_debug() {
        let flags = PermissionFlags {
            print: true,
            modify_contents: false,
            copy: true,
            modify_annotations: false,
            fill_forms: true,
            accessibility: true,
            assemble: false,
            print_high_quality: true,
        };

        // Test that debug formatting works
        let debug_str = format!("{:?}", flags);
        assert!(debug_str.contains("PermissionFlags"));
    }

    #[test]
    fn test_permission_flags_clone() {
        let flags = PermissionFlags {
            print: true,
            modify_contents: false,
            copy: true,
            modify_annotations: false,
            fill_forms: true,
            accessibility: true,
            assemble: false,
            print_high_quality: true,
        };

        let cloned_flags = flags.clone();
        assert_eq!(flags.print, cloned_flags.print);
        assert_eq!(flags.copy, cloned_flags.copy);
        assert_eq!(flags.fill_forms, cloned_flags.fill_forms);
    }

    #[test]
    fn test_permissions_specific_bit_patterns() {
        // Test that specific bit combinations work correctly
        let mut perm = Permissions::from_bits(0xFFFFF0C0);

        // Set bit 2 (print permission)
        perm.set_print(true);
        assert_eq!(perm.bits() & (1 << 2), 1 << 2);

        // Set bit 3 (modify contents)
        perm.set_modify_contents(true);
        assert_eq!(perm.bits() & (1 << 3), 1 << 3);

        // Set bit 4 (copy)
        perm.set_copy(true);
        assert_eq!(perm.bits() & (1 << 4), 1 << 4);

        // Set bit 5 (modify annotations)
        perm.set_modify_annotations(true);
        assert_eq!(perm.bits() & (1 << 5), 1 << 5);
    }

    #[test]
    fn test_permissions_roundtrip_conversion() {
        let original_flags = PermissionFlags {
            print: true,
            modify_contents: false,
            copy: true,
            modify_annotations: true,
            fill_forms: false,
            accessibility: true,
            assemble: false,
            print_high_quality: true,
        };

        let perm = Permissions::from_flags(original_flags);
        let converted_flags = perm.flags();

        assert_eq!(original_flags.print, converted_flags.print);
        assert_eq!(
            original_flags.modify_contents,
            converted_flags.modify_contents
        );
        assert_eq!(original_flags.copy, converted_flags.copy);
        assert_eq!(
            original_flags.modify_annotations,
            converted_flags.modify_annotations
        );
        assert_eq!(original_flags.fill_forms, converted_flags.fill_forms);
        assert_eq!(original_flags.accessibility, converted_flags.accessibility);
        assert_eq!(original_flags.assemble, converted_flags.assemble);
        assert_eq!(
            original_flags.print_high_quality,
            converted_flags.print_high_quality
        );
    }

    #[test]
    fn test_permissions_edge_cases() {
        // Test with bits that shouldn't affect permissions
        let perm = Permissions::from_bits(0x00000000);
        assert!(!perm.can_print());
        assert!(!perm.can_copy());

        // Test with all bits set
        let perm = Permissions::from_bits(0xFFFFFFFF);
        assert!(perm.can_print());
        assert!(perm.can_modify_contents());
        assert!(perm.can_copy());
        assert!(perm.can_modify_annotations());
        assert!(perm.can_fill_forms());
        assert!(perm.can_access_for_accessibility());
        assert!(perm.can_assemble());
        assert!(perm.can_print_high_quality());
    }
}
