import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hash_killer_frontend/src/features/scan/application/scan_controller.dart';
import 'package:hash_killer_frontend/src/features/scan/application/scan_state.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/activity_event.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/hash_algorithm.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_mode.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_report.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_repository.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_request.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/volume_destination.dart';

void main() {
  test(
    'updates progress from activity events and stays complete on success',
    () async {
      final repository = _SuccessfulScanRepository(
        destinationsConfigured: true,
        report: const ScanReport(
          scannedFiles: 0,
          candidateFiles: 0,
          hashedFiles: 0,
          reusedHashes: 0,
          duplicateGroups: 0,
          deletedFiles: 0,
          keptFiles: 0,
          reclaimedBytes: 0,
          failedFiles: [],
          duplicateRelations: [],
        ),
      );
      final container = ProviderContainer(
        overrides: [scanRepositoryProvider.overrideWithValue(repository)],
      );
      addTearDown(container.dispose);

      final controller = container.read(scanControllerProvider.notifier);

      controller.setPathsText('/tmp/a');
      await Future<void>.delayed(Duration.zero);
      await controller.run();

      final state = container.read(scanControllerProvider);

      expect(state.status, ScanStatus.success);
      expect(state.scanProgress?.progress, 1);
      expect(state.progress, 1);
    },
  );

  test('blocks scan until quarantine destinations are configured', () async {
    final repository = _SuccessfulScanRepository();
    final container = ProviderContainer(
      overrides: [scanRepositoryProvider.overrideWithValue(repository)],
    );
    addTearDown(container.dispose);

    final controller = container.read(scanControllerProvider.notifier);

    controller.setPathsText('/tmp/a\n/tmp/b');
    await controller.run();

    final state = container.read(scanControllerProvider);

    expect(state.status, ScanStatus.idle);
    expect(state.statusMessage, '모든 디스크의 보관 폴더를 먼저 지정하십시오.');
    expect(repository.savedQuarantinePath, isNull);
    expect(state.report, isNull);
  });

  test('clears persisted quarantine destination when paths change', () async {
    final repository = _SuccessfulScanRepository(
      savedQuarantinePath: '/var/folders/stale/quarantine',
    );
    final container = ProviderContainer(
      overrides: [scanRepositoryProvider.overrideWithValue(repository)],
    );
    addTearDown(container.dispose);

    final controller = container.read(scanControllerProvider.notifier);

    controller.setPathsText('/tmp/new');
    await Future<void>.delayed(Duration.zero);

    final state = container.read(scanControllerProvider);

    expect(repository.clearQuarantineCount, 1);
    expect(state.quarantineDestinations.single.configured, false);
    expect(state.quarantineDestinations.single.targetPath, '지정되지 않음');
  });
}

class _SuccessfulScanRepository implements ScanRepository {
  _SuccessfulScanRepository({
    this.destinationsConfigured = false,
    this.report,
    this.savedQuarantinePath,
  });

  final bool destinationsConfigured;
  final ScanReport? report;
  String? savedQuarantinePath;
  int clearQuarantineCount = 0;

  @override
  Future<void> cancelActiveScan() async {}

  @override
  Future<int> clearCache() async => 0;

  @override
  Future<int> clearQuarantineDestinations() async {
    clearQuarantineCount += 1;
    savedQuarantinePath = null;
    return 1;
  }

  @override
  Future<ScanSettings> loadSettings() async {
    return const ScanSettings(
      cacheLimitMb: 256,
      cacheLimitConfigured: false,
      algorithm: HashAlgorithm.md5,
      algorithmConfigured: false,
      scanMode: ScanMode.fullHash,
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
    onActivity(
      ActivityEvent(
        stage: '검사 완료',
        detail: '${request.roots.length}개 경로',
        progress: 1,
        completed: 1,
        total: 1,
      ),
    );

    return report ??
        ScanReport(
          scannedFiles: request.roots.length,
          candidateFiles: 0,
          hashedFiles: 0,
          reusedHashes: 0,
          duplicateGroups: 0,
          deletedFiles: 0,
          keptFiles: request.roots.length,
          reclaimedBytes: 0,
          failedFiles: const [],
          duplicateRelations: const [],
        );
  }

  @override
  Future<int> saveCacheLimit(int value) async => 0;

  @override
  Future<void> saveHashAlgorithm(HashAlgorithm algorithm) async {}

  @override
  Future<void> saveQuarantineDestination({
    required String volumeKey,
    required String targetPath,
  }) async {
    savedQuarantinePath = targetPath;
  }

  @override
  Future<void> saveScanMode(ScanMode scanMode) async {}

  @override
  Future<List<VolumeDestination>> volumeDestinations(List<String> roots) async {
    final configured = destinationsConfigured || savedQuarantinePath != null;

    return [
      VolumeDestination(
        volumeKey: 'test',
        rootPath: roots.first,
        rootPaths: roots,
        targetPath: destinationsConfigured
            ? '/tmp/quarantine'
            : savedQuarantinePath ?? '지정되지 않음',
        configured: configured,
      ),
    ];
  }
}
