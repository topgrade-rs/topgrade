#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_from_str_valid() {
        let version_str = "1.2.3";
        let version = Version::from_str(version_str).unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        // ... additional assertions for the rest of the version fields
    }

    #[test]
    fn test_version_from_str_invalid() {
        let version_str = "not.a.version";
        assert!(Version::from_str(version_str).is_err());
    }

    // Additional tests for edge cases and error scenarios
}
