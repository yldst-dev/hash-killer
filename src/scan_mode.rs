use std::fmt;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScanMode {
    #[default]
    Fast,
    FullHash,
    Rehash,
}

impl ScanMode {
    pub fn all() -> &'static [ScanMode] {
        &[ScanMode::Fast, ScanMode::FullHash, ScanMode::Rehash]
    }

    pub fn id(self) -> &'static str {
        match self {
            ScanMode::Fast => "FAST",
            ScanMode::FullHash => "FULL_HASH",
            ScanMode::Rehash => "REHASH",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ScanMode::Fast => "빠른 일반 모드",
            ScanMode::FullHash => "전체 해시 모드",
            ScanMode::Rehash => "재계산 모드",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ScanMode::Fast => "같은 용량 파일만 해시하고 캐시를 재사용",
            ScanMode::FullHash => "모든 파일을 해시 대상으로 포함",
            ScanMode::Rehash => "캐시를 쓰지 않고 후보 해시를 다시 계산",
        }
    }

    pub fn from_id(value: &str) -> ScanMode {
        match value.trim().to_ascii_uppercase().as_str() {
            "FULL_HASH" | "FULL" => ScanMode::FullHash,
            "REHASH" | "RECALCULATE" => ScanMode::Rehash,
            _ => ScanMode::Fast,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn uses_cache(self) -> bool {
        !matches!(self, ScanMode::Rehash)
    }
}

impl fmt::Display for ScanMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}
