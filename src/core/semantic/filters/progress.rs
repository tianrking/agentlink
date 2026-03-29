use regex::Regex;

pub struct ProgressFilter {
    re: Regex,
}

impl ProgressFilter {
    pub fn new() -> Self {
        Self {
            re: Regex::new(
                r"(?i)^\s*((\d{1,3}%\s*)|(\[[=\-#>.\s]+\])|(downloading)|(installing)|(fetching))",
            )
            .expect("regex must compile"),
        }
    }

    pub fn should_drop(&self, line: &str) -> bool {
        self.re.is_match(line)
    }
}

#[cfg(test)]
mod tests {
    use super::ProgressFilter;

    #[test]
    fn drops_percent_progress_line() {
        let f = ProgressFilter::new();
        assert!(f.should_drop("42%"));
    }

    #[test]
    fn keeps_regular_output_line() {
        let f = ProgressFilter::new();
        assert!(!f.should_drop("build finished successfully"));
    }
}
