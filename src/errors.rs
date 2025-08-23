//! Error types for FIX protocol parsing and validation.
//!
//! This module provides comprehensive error handling for FIX protocol data parsing,
//! including missing required fields and invalid value errors.

/// Strict parse error type for generated read() APIs.
///
/// This error type is designed for high-performance parsing scenarios where
/// detailed error information is needed for debugging and validation.
#[derive(Debug)]
#[allow(dead_code)]
pub enum ReadError {
    /// Aggregated (bitmask) missing required members: fields (kind=0), components (kind=1), groups (kind=2).
    ///
    /// The meta slice layout is: (name, tag_or_count_tag, kind)
    MissingRequiredFields {
        /// Bitmask indicating which required fields are missing
        missing_mask: u64,
        /// Metadata about the fields, components, and groups
        meta: &'static [(&'static str, u16, u8)],
    },
    /// Invalid value encountered during parsing
    InvalidValue {
        /// Name of the field that had an invalid value
        name: &'static str,
        /// FIX tag number
        tag: u16,
        /// Description of what went wrong
        msg: &'static str,
    },
}

impl core::fmt::Display for ReadError {
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        match self {
            ReadError::MissingRequiredFields { missing_mask, meta } => {
                if *missing_mask == 0 {
                    return write!(
                        f,
                        "No required members are missing"
                    );
                }
                write!(
                    f,
                    "Missing required members (mask=0x{missing_mask:0X}): "
                )?;
                let mut first = true;
                for (i, (name, tag, kind)) in meta.iter().enumerate() {
                    if (missing_mask >> i) & 1 == 1 {
                        if !first {
                            write!(f, ", ")?;
                        }
                        first = false;
                        let kind_str = match kind {
                            0 => "field",
                            1 => "component",
                            _ => "group",
                        };
                        if *kind == 2 {
                            // group: tag is the count tag
                            write!(
                                f,
                                "{kind_str} {name}(countTag={tag})"
                            )?;
                        } else {
                            write!(
                                f,
                                "{kind_str} {name}(tag={tag})"
                            )?;
                        }
                    }
                }
                Ok(())
            }
            ReadError::InvalidValue { name, tag, msg } => {
                write!(
                    f,
                    "Invalid value for {name} (tag={tag}): {msg}"
                )
            }
        }
    }
}

impl std::error::Error for ReadError {}

impl ReadError {
    /// Returns the list of names of missing required members when this is
    /// `ReadError::MissingRequiredFields`.
    ///
    /// Returns `Some(Vec::new())` if the variant is present but no bits are
    /// actually missing (mask == 0), and `None` for other variants.
    #[allow(dead_code)]
    pub fn missing_member_names(&self) -> Option<Vec<&'static str>> {
        match self {
            ReadError::MissingRequiredFields { missing_mask, meta } => {
                let mut v = Vec::new();
                if *missing_mask != 0 {
                    for (i, (name, _tag, _kind)) in meta.iter().enumerate() {
                        if (missing_mask >> i) & 1 == 1 {
                            v.push(*name);
                        }
                    }
                }
                Some(v)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_error_display_no_missing_fields() {
        let error = ReadError::MissingRequiredFields {
            missing_mask: 0,
            meta: &[("TestField", 1, 0)],
        };
        assert_eq!(
            error.to_string(),
            "No required members are missing"
        );
    }

    #[test]
    fn test_read_error_display_missing_fields() {
        let meta = &[("Field1", 1, 0), ("Component1", 2, 1), ("Group1", 3, 2)];
        let error = ReadError::MissingRequiredFields {
            missing_mask: 0b101, // First and third items missing
            meta,
        };
        let error_str = error.to_string();
        assert!(error_str.contains("Missing required members"));
        assert!(error_str.contains("field Field1(tag=1)"));
        assert!(error_str.contains("group Group1(countTag=3)"));
    }

    #[test]
    fn test_read_error_display_invalid_value() {
        let error = ReadError::InvalidValue {
            name: "TestField",
            tag: 42,
            msg: "Expected numeric value",
        };
        assert_eq!(
            error.to_string(),
            "Invalid value for TestField (tag=42): Expected numeric value"
        );
    }

    #[test]
    fn test_missing_member_names() {
        let meta = &[("Field1", 1, 0), ("Field2", 2, 0), ("Field3", 3, 0)];

        // Test with missing fields
        let error = ReadError::MissingRequiredFields {
            missing_mask: 0b101, // First and third fields missing
            meta,
        };
        let names = error
            .missing_member_names()
            .unwrap();
        assert_eq!(
            names,
            vec!["Field1", "Field3"]
        );

        // Test with no missing fields
        let error = ReadError::MissingRequiredFields {
            missing_mask: 0,
            meta,
        };
        let names = error
            .missing_member_names()
            .unwrap();
        assert!(names.is_empty());

        // Test with invalid value error
        let error = ReadError::InvalidValue {
            name: "TestField",
            tag: 42,
            msg: "Test message",
        };
        assert!(error
            .missing_member_names()
            .is_none());
    }
}
