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

#[cfg(test)]
mod tests {
    use super::MotdFilter;

    #[test]
    fn drops_common_motd_line() {
        let f = MotdFilter::new();
        assert!(f.should_drop("Last login: Sun Mar 10 09:00:00"));
    }

    #[test]
    fn keeps_regular_command_output() {
        let f = MotdFilter::new();
        assert!(!f.should_drop("src/main.rs"));
    }
}
