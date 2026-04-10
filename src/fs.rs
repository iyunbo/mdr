use crate::error::AppError;

pub fn read_file(path: &str) -> Result<String, AppError> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_file_returns_content() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "# Hello mdr").unwrap();
        let path = f.path().to_str().unwrap().to_string();
        let content = read_file(&path).unwrap();
        assert!(content.contains("# Hello mdr"));
    }

    #[test]
    fn test_read_file_missing_returns_error() {
        let result = read_file("/tmp/this_file_does_not_exist_mdr.md");
        assert!(result.is_err());
    }
}
