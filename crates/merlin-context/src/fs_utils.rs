use std::path::Path;

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "toml", "md", "txt", "json", "yaml", "yml", "js", "ts", "jsx", "tsx", "py", "java", "go",
    "c", "cpp", "h", "hpp",
];

pub fn is_source_file(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };
    extension.to_str().is_some_and(|ext| {
        SOURCE_EXTENSIONS
            .iter()
            .any(|allowed| ext.eq_ignore_ascii_case(allowed))
    })
}
