use regex::Regex;

/// Returns a list of compiled regex with the list of paths that should be ignored.
/// Panics if any of the regex statements is incorrect.
pub(crate) fn compile_ignore_paths() -> Vec<Regex> {
    IGNORE_PATHS
        .iter()
        .map(|ignore_path| Regex::new(ignore_path).expect(&format!("Invalid IGNORE_TYPES regex: {}", ignore_path)))
        .collect::<Vec<Regex>>()
}

#[test]
fn test_compile_ignore_paths() {
    assert!(compile_ignore_paths().len() > 0);
}

/// A list of path fragments, file names, file extensions as Regex.
/// Files with the path matching any of regex from this list are ignored.
const IGNORE_PATHS: [&str; 53] = [
    // known framework paths
    r#"(?i)node_modules[/\\]"#,
    r#"(?i)angular[/\\]README\.md"#,
    r#"(?i)package-lock\.json"#,
    r#"(?i)/vendor/"#,
    // images
    r#"(?i)\.ico$"#,
    r#"(?i)\.png$"#,
    r#"(?i)\.jpg$"#,
    r#"(?i)\.jpeg$"#,
    r#"(?i)\.gif$"#,
    r#"(?i)\.svg$"#,
    r#"(?i)\.bmp$"#,
    r#"(?i)\.tif$"#,
    r#"(?i)\.tiff$"#,
    r#"(?i)\.eps$"#,
    r#"(?i)\.webp$"#,
    r#"(?i)\.psd$"#,
    r#"(?i)\.webm$"#,
    // audio / video
    r#"(?i)\.mp4$"#,
    r#"(?i)\.mp3$"#,
    r#"(?i)\.mpeg$"#,
    // fonts
    r#"(?i)\.ttf$"#,
    r#"(?i)\.otf$"#,
    r#"(?i)\.eot$"#,
    r#"(?i)\.woff$"#,
    r#"(?i)\.woff2$"#,
    // documents
    r#"(?i)\.pdf$"#,
    r#"(?i)\.doc$"#,
    r#"(?i)\.docx$"#,
    r#"(?i)\.txt$"#,
    // git files
    r#"(?i)\.gitignore$"#,
    r#"(?i)\.gitattributes$"#,
    r#"(?i)\.gitkeep$"#,
    r#"(?i)\.keep$"#,
    // binaries
    r#"(?i)\.exe$"#,
    r#"(?i)\.dll$"#,
    r#"(?i)\.so$"#,
    r#"(?i)\.jar$"#,
    r#"(?i)\.pdb$"#,
    r#"(?i)\.gem$"#,
    // archives
    r#"(?i)\.zip$"#,
    r#"(?i)\.rar$"#,
    r#"(?i)\.tar$"#,
    r#"(?i)\.gz$"#,
    // data files
    r#"(?i)\.csv$"#,
    r#"(?i)\.tsv$"#,
    r#"(?i)\.xls$"#,
    r#"(?i)\.xlsx$"#,
    // secrets
    r#"(?i)\.cer$"#,
    r#"(?i)\.crt$"#,
    r#"(?i)\.pfx$"#,
    r#"(?i)\.pem$"#,
    r#"(?i)\.p12$"#,
    r#"(?i)\.key$"#,
];
