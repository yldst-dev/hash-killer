use crate::duplicate_cleaner::CleanReport;

pub fn progress(report: Option<&CleanReport>) -> f64 {
    report
        .map(|report| {
            if report.candidate_files > 0 {
                let processed = report.hashed_files + report.reused_hashes;
                (processed as f64 / report.candidate_files as f64).min(1.0)
            } else if report.scanned_files > 0 {
                1.0
            } else {
                0.0
            }
        })
        .unwrap_or(0.0)
}

pub fn format_bytes(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, units[unit])
    } else {
        format!("{:.2} {}", size, units[unit])
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn print_report(report: &CleanReport) {
    println!("스캔 파일: {}", report.scanned_files);
    println!("후보 파일: {}", report.candidate_files);
    println!("해시 계산: {}", report.hashed_files);
    println!("캐시 재사용: {}", report.reused_hashes);
    println!("중복 그룹: {}", report.duplicate_groups);
    println!("삭제 파일: {}", report.deleted_files);
    println!("보존 파일: {}", report.kept_files);
    println!("회수 용량: {}", format_bytes(report.reclaimed_bytes));

    for failed in &report.failed_files {
        eprintln!("삭제 실패: {failed}");
    }
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, progress};
    use crate::duplicate_cleaner::CleanReport;

    #[test]
    fn formats_byte_units_through_exabytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024_u64.pow(2)), "1.00 MB");
        assert_eq!(format_bytes(1024_u64.pow(3)), "1.00 GB");
        assert_eq!(format_bytes(1024_u64.pow(4)), "1.00 TB");
        assert_eq!(format_bytes(1024_u64.pow(5)), "1.00 PB");
        assert_eq!(format_bytes(1024_u64.pow(6)), "1.00 EB");
    }

    #[test]
    fn completed_report_progress_is_full() {
        let report = CleanReport {
            scanned_files: 11729,
            candidate_files: 9258,
            hashed_files: 9258,
            reused_hashes: 0,
            duplicate_groups: 1291,
            deleted_files: 1339,
            kept_files: 7919,
            reclaimed_bytes: 42_624_614,
            failed_files: Vec::new(),
            duplicate_relations: Vec::new(),
        };

        assert_eq!(progress(Some(&report)), 1.0);
    }
}
