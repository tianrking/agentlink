use regex::Regex;

pub struct ProgressFilter {
    re: Regex,
}

impl ProgressFilter {
    pub fn new() -> Self {
        Self {
            re: Regex::new(
                r"(?i)^\s*((\d{1,3}%\s*)|(\[[=\-\>#\.\s]+\])|(downloading)|(installing)|(fetching))",
            )
            .expect("regex must compile"),
        }
    }

    pub fn should_drop(&self, line: &str) -> bool {
        self.re.is_match(line)
    }
}
