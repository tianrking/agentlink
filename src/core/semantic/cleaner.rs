use crate::core::semantic::filters::{MotdFilter, ProgressFilter};

#[derive(Debug, Clone, Copy)]
pub struct CleanerConfig {
    pub enabled: bool,
    pub strip_ansi: bool,
    pub drop_progress: bool,
    pub drop_motd: bool,
}

impl CleanerConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            strip_ansi: false,
            drop_progress: false,
            drop_motd: false,
        }
    }
}

pub struct StreamCleaner {
    pending: Vec<u8>,
    cfg: CleanerConfig,
    progress_filter: ProgressFilter,
    motd_filter: MotdFilter,
}

impl StreamCleaner {
    pub fn new(cfg: CleanerConfig) -> Self {
        Self {
            pending: Vec::new(),
            cfg,
            progress_filter: ProgressFilter::new(),
            motd_filter: MotdFilter::new(),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<u8> {
        if !self.cfg.enabled {
            return chunk.to_vec();
        }

        self.pending.extend_from_slice(chunk);
        let mut output = Vec::new();

        while let Some(pos) = self.pending.iter().position(|b| *b == b'\n') {
            let mut line = self.pending.drain(..=pos).collect::<Vec<u8>>();
            let _ = line.pop();
            let cleaned = self.clean_line(&line);
            if !cleaned.is_empty() {
                output.extend_from_slice(&cleaned);
                output.push(b'\n');
            }
        }

        output
    }

    pub fn flush(&mut self) -> Vec<u8> {
        if self.pending.is_empty() {
            return Vec::new();
        }

        let remaining = std::mem::take(&mut self.pending);
        self.clean_line(&remaining)
    }

    fn clean_line(&self, raw: &[u8]) -> Vec<u8> {
        if raw.is_empty() {
            return Vec::new();
        }

        let collapsed = match raw.iter().rposition(|b| *b == b'\r') {
            Some(idx) => &raw[idx + 1..],
            None => raw,
        };
        if collapsed.is_empty() {
            return Vec::new();
        }

        let stripped = strip_ansi_escapes::strip(collapsed);
        let plain = String::from_utf8_lossy(&stripped).trim().to_string();

        if plain.is_empty() {
            return Vec::new();
        }
        if self.cfg.drop_progress && self.progress_filter.should_drop(&plain) {
            return Vec::new();
        }
        if self.cfg.drop_motd && self.motd_filter.should_drop(&plain) {
            return Vec::new();
        }

        if self.cfg.strip_ansi {
            plain.into_bytes()
        } else {
            collapsed.to_vec()
        }
    }
}
