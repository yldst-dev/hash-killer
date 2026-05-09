enum HashAlgorithm {
  blake3('BLAKE3', 'BLAKE3', '기본값, 빠른 일반 검사에 적합'),
  sha256('SHA256', 'SHA-256', '범용 호환성이 높은 256비트 해시'),
  sha512('SHA512', 'SHA-512', '긴 다이제스트가 필요한 검사용'),
  md5('MD5', 'MD5', '레거시 비교용, 보안 용도 아님');

  const HashAlgorithm(this.id, this.label, this.description);

  final String id;
  final String label;
  final String description;

  static HashAlgorithm fromId(String value) {
    final normalized = value.trim().toUpperCase();

    return switch (normalized) {
      'SHA256' || 'SHA-256' => HashAlgorithm.sha256,
      'SHA512' || 'SHA-512' => HashAlgorithm.sha512,
      'MD5' => HashAlgorithm.md5,
      _ => HashAlgorithm.blake3,
    };
  }
}
