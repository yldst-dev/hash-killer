enum ScanMode {
  fast('FAST', '빠른 일반 모드', '같은 용량 파일만 해시하고 캐시를 재사용'),
  fullHash('FULL_HASH', '전체 해시 모드', '모든 파일을 해시 대상으로 포함'),
  rehash('REHASH', '재계산 모드', '캐시를 쓰지 않고 후보 해시를 다시 계산');

  const ScanMode(this.id, this.label, this.description);

  final String id;
  final String label;
  final String description;

  static ScanMode fromId(String value) {
    final normalized = value.trim().toUpperCase();

    return switch (normalized) {
      'FULL_HASH' || 'FULL' => ScanMode.fullHash,
      'REHASH' || 'RECALCULATE' => ScanMode.rehash,
      _ => ScanMode.fast,
    };
  }
}
