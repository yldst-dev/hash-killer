import 'package:flutter_test/flutter_test.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/hash_algorithm.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_mode.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_report.dart';

void main() {
  test('hash algorithm ids match Rust contract', () {
    expect(HashAlgorithm.fromId('BLAKE3'), HashAlgorithm.blake3);
    expect(HashAlgorithm.fromId('SHA-256'), HashAlgorithm.sha256);
    expect(HashAlgorithm.fromId('SHA512'), HashAlgorithm.sha512);
    expect(HashAlgorithm.fromId('MD5'), HashAlgorithm.md5);
    expect(HashAlgorithm.fromId('unknown'), HashAlgorithm.blake3);
  });

  test('scan mode ids match Rust contract', () {
    expect(ScanMode.fromId('FAST'), ScanMode.fast);
    expect(ScanMode.fromId('FULL'), ScanMode.fullHash);
    expect(ScanMode.fromId('FULL_HASH'), ScanMode.fullHash);
    expect(ScanMode.fromId('RECALCULATE'), ScanMode.rehash);
    expect(ScanMode.fromId('unknown'), ScanMode.fast);
  });

  test('report progress follows Rust reporting behavior', () {
    const report = ScanReport(
      scannedFiles: 4,
      candidateFiles: 3,
      hashedFiles: 1,
      reusedHashes: 1,
      duplicateGroups: 0,
      deletedFiles: 0,
      keptFiles: 0,
      reclaimedBytes: 0,
      failedFiles: [],
      duplicateRelations: [],
    );

    expect(report.progress, closeTo(2 / 3, 0.0001));
    expect(const ScanReport.empty().progress, 0);
  });
}
