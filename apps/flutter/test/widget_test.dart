import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:hash_killer_frontend/src/app/hash_killer_app.dart';
import 'package:hash_killer_frontend/src/core/design_system/app_button.dart';
import 'package:hash_killer_frontend/src/features/scan/application/scan_controller.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/activity_event.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/hash_algorithm.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_mode.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_report.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_repository.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/scan_request.dart';
import 'package:hash_killer_frontend/src/features/scan/domain/volume_destination.dart';

void main() {
  testWidgets('scan dashboard renders', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          scanRepositoryProvider.overrideWithValue(_TestRepository()),
        ],
        child: const HashKillerApp(),
      ),
    );
    await tester.pump();

    expect(find.text('검사 설정'), findsOneWidget);
    expect(find.text('진행 상태'), findsOneWidget);
    expect(find.text('결과 요약'), findsOneWidget);
  });

  testWidgets('settings buttons and radio options are interactive', (
    tester,
  ) async {
    final repository = _TestRepository();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [scanRepositoryProvider.overrideWithValue(repository)],
        child: const HashKillerApp(),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(AppButton, '설정').at(1));
    await tester.pumpAndSettle();

    expect(find.text('비교 기준'), findsWidgets);
    expect(find.text('SHA-256'), findsOneWidget);

    await tester.tap(find.text('SHA-256'));
    await tester.pumpAndSettle();

    expect(repository.algorithm, HashAlgorithm.sha256);
  });

  testWidgets('cache setting input accepts typing and saves', (tester) async {
    final repository = _TestRepository();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [scanRepositoryProvider.overrideWithValue(repository)],
        child: const HashKillerApp(),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.widgetWithText(AppButton, '설정').at(3));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(EditableText), '512');
    await tester.tap(find.widgetWithText(AppButton, '저장'));
    await tester.pumpAndSettle();

    expect(repository.cacheLimitMb, 512);
    expect(find.text('512 MB'), findsOneWidget);
  });

  test('activity log keeps full event history for export', () async {
    final repository = _TestRepository(eventCount: 250);
    final container = ProviderContainer(
      overrides: [scanRepositoryProvider.overrideWithValue(repository)],
    );

    addTearDown(container.dispose);

    final controller = container.read(scanControllerProvider.notifier);

    controller.setPathsText('/tmp/sample');
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);
    await controller.run();

    final state = container.read(scanControllerProvider);

    expect(state.activityEvents.length, 200);
    expect(state.activityLogEvents.length, 251);
    expect(state.activityLogEvents.first.detail, 'event 0');
    expect(state.activityLogEvents.last.stage, '완료');
  });
}

class _TestRepository implements ScanRepository {
  _TestRepository({this.eventCount = 0});

  final int eventCount;
  HashAlgorithm algorithm = HashAlgorithm.blake3;
  int cacheLimitMb = 256;

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
    for (var index = 0; index < eventCount; index += 1) {
      onActivity(ActivityEvent(stage: '이벤트', detail: 'event $index'));
    }

    return const ScanReport.empty();
  }

  @override
  Future<int> saveCacheLimit(int value) async {
    cacheLimitMb = value;
    return 0;
  }

  @override
  Future<void> saveHashAlgorithm(HashAlgorithm algorithm) async {
    this.algorithm = algorithm;
  }

  @override
  Future<void> saveQuarantineDestination({
    required String volumeKey,
    required String targetPath,
  }) async {}

  @override
  Future<void> saveScanMode(ScanMode scanMode) async {}

  @override
  Future<List<VolumeDestination>> volumeDestinations(List<String> roots) async {
    return roots
        .map(
          (root) => VolumeDestination(
            volumeKey: root,
            rootPath: root,
            rootPaths: [root],
            targetPath: '/tmp/quarantine',
            configured: true,
          ),
        )
        .toList(growable: false);
  }
}
