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
const IGNORE_PATHS: [&str; 40] = [
    // known framework paths
    r#"node_modules[/\\]"#,
    r#"angular[/\\]README\.md"#,
    r#"package-lock\.json"#,
    // images
    r#"\.ico$"#,
    r#"\.png$"#,
    r#"\.jpg$"#,
    r#"\.jpeg$"#,
    r#"\.gif$"#,
    r#"\.svg$"#,
    r#"\.bmp$"#,
    r#"\.tif$"#,
    r#"\.tiff$"#,
    r#"\.eps$"#,
    r#"\.webp$"#,
    // audio / video
    r#"\.mp4$"#,
    r#"\.mp3$"#,
    r#"\.mpeg$"#,
    // fonts
    r#"\.ttf$"#,
    r#"\.otf$"#,
    r#"\.eot$"#,
    r#"\.woff$"#,
    r#"\.woff2$"#,
    // documents
    r#"\.pdf$"#,
    r#"\.doc$"#,
    r#"\.docx$"#,
    r#"\.txt$"#,
    // git files
    r#"\.gitignore$"#,
    r#"\.gitattributes$"#,
    r#"\.gitkeep$"#,
    // binaries
    r#"\.exe$"#,
    r#"\.dll$"#,
    r#"\.so$"#,
    r#"\.jar$"#,
    r#"\.pdb$"#,
    // archives
    r#"\.zip$"#,
    r#"\.rar$"#,
    // data files
    r#"\.csv$"#,
    r#"\.tsv$"#,
    r#"\.xls$"#,
    r#"\.xlsx$"#,
];
