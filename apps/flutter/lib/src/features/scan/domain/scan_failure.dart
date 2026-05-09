enum ScanFailureKind {
  invalidInput,
  bridgeUnavailable,
  nativeFailure,
  cancelled,
}

class ScanFailure implements Exception {
  const ScanFailure(this.kind, this.message);

  final ScanFailureKind kind;
  final String message;

  @override
  String toString() => message;
}
