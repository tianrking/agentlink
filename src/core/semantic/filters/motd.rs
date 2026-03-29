use regex::Regex;

pub struct MotdFilter {
    re: Regex,
}

impl MotdFilter {
    pub fn new() -> Self {
        Self {
            re: Regex::new(
                r"(?i)(last login:|welcome to|system information|load average|memory usage|disk usage)",
            )
            .expect("regex must compile"),
        }
    }

    pub fn should_drop(&self, line: &str) -> bool {
        self.re.is_match(line)
    }
}
