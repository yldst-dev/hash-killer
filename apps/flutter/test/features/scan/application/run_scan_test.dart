import 'package:flutter_test/flutter_test.dart';
import 'package:hash_killer_frontend/src/features/scan/application/run_scan.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/activity_event.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/hash_algorithm.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_failure.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_mode.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_report.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_repository.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_request.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/volume_destination.dart';

void main() {
  test('normalizes paths before calling repository', () async {
    final repository = _RecordingScanRepository();
    final runScan = RunScan(repository);
    final events = <ActivityEvent>[];

    await runScan(
      const ScanRequest(roots: [' /tmp/a ', '', '/tmp/a', '/tmp/b']),
      onActivity: events.add,
    );

    expect(repository.request?.roots, ['/tmp/a', '/tmp/b']);
    expect(events.single.stage, '완료');
  });

  test('rejects empty paths before bridge call', () async {
    final repository = _RecordingScanRepository();
    final runScan = RunScan(repository);

    expect(
      () => runScan(const ScanRequest(roots: []), onActivity: (_) {}),
      throwsA(isA<ScanFailure>()),
    );
    expect(repository.request, isNull);
  });
}

class _RecordingScanRepository implements ScanRepository {
  ScanRequest? request;

  @override
  Future<void> cancelActiveScan() async {}

  @override
  Future<int> clearCache() async => 0;

  @override
  Future<int> clearQuarantineDestinations() async => 0;

  @override
  Future<ScanSettings> loadSettings() async {
    return const ScanSettings(
      cacheLimitMb: 256,
      cacheLimitConfigured: false,
      algorithm: HashAlgorithm.blake3,
      algorithmConfigured: false,
      scanMode: ScanMode.fast,
      scanModeConfigured: false,
    );
  }

  @override
  Future<void> revealFile(String path) async {}

  @override
  Future<ScanReport> run(
    ScanRequest request, {
    required void Function(ActivityEvent event) onActivity,
  }) async {
    this.request = request;
    onActivity(const ActivityEvent(stage: '완료', detail: '테스트 완료'));
    return const ScanReport.empty();
  }

  @override
  Future<int> saveCacheLimit(int value) async => 0;

  @override
  Future<void> saveHashAlgorithm(HashAlgorithm algorithm) async {}

  @override
  Future<void> saveQuarantineDestination({
    required String volumeKey,
    required String targetPath,
  }) async {}

  @override
  Future<void> saveScanMode(ScanMode scanMode) async {}

  @override
  Future<List<VolumeDestination>> volumeDestinations(List<String> roots) async {
    return const [];
  }
}
