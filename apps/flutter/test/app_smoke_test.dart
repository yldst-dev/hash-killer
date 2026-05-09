import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hash_killer_frontend/src/app/hash_killer_app.dart';
import 'package:hash_killer_frontend/src/features/scan/application/scan_controller.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/activity_event.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/hash_algorithm.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_mode.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_report.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_repository.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_request.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/volume_destination.dart';

void main() {
  testWidgets('renders scan page without startup exceptions', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          scanRepositoryProvider.overrideWithValue(_TestRepository()),
        ],
        child: const HashKillerApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('검사 설정'), findsOneWidget);
    expect(find.text('결과 요약'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

class _TestRepository implements ScanRepository {
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
