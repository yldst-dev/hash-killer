import 'hash_algorithm.dart';
import 'scan_mode.dart';

class ScanRequest {
  const ScanRequest({
    required this.roots,
    this.algorithm = HashAlgorithm.blake3,
    this.scanMode = ScanMode.fast,
  });

  final List<String> roots;
  final HashAlgorithm algorithm;
  final ScanMode scanMode;

  ScanRequest normalize() {
    final normalizedRoots = roots
        .map((path) => path.trim())
        .where((path) => path.isNotEmpty)
        .toSet()
        .toList(growable: false);

    return ScanRequest(
      roots: normalizedRoots,
      algorithm: algorithm,
      scanMode: scanMode,
    );
  }
}
