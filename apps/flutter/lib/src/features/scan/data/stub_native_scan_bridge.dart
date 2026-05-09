import '../../../core/native/native_scan_bridge.dart';
import '../domain/activity_event.dart';
import '../domain/hash_algorithm.dart';
import '../domain/scan_failure.dart';
import '../domain/scan_report.dart';
import '../domain/scan_request.dart';
import '../domain/scan_mode.dart';
import '../domain/volume_destination.dart';

class StubNativeScanBridge implements NativeScanBridge {
  const StubNativeScanBridge();

  @override
  Future<ScanReport> runScan(
    ScanRequest request, {
    required void Function(ActivityEvent event) onActivity,
  }) async {
    onActivity(
      const ActivityEvent(
        stage: '브리지 준비',
        detail: 'Rust 브리지가 아직 연결되지 않았습니다.',
        progress: 0,
      ),
    );

    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'Rust 브리지 연결 후 검사를 실행할 수 있습니다.',
    );
  }

  @override
  Future<NativeScanSettings> loadSettings() async {
    return const NativeScanSettings(
      cacheLimitMb: 256,
      cacheLimitConfigured: false,
      algorithm: HashAlgorithm.blake3,
      algorithmConfigured: false,
      scanMode: ScanMode.fast,
      scanModeConfigured: false,
    );
  }

  @override
  Future<int> saveCacheLimit(int value) async {
    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'Rust 브리지 연결 후 캐시 설정을 저장할 수 있습니다.',
    );
  }

  @override
  Future<int> clearCache() async {
    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'Rust 브리지 연결 후 SQLite 캐시를 삭제할 수 있습니다.',
    );
  }

  @override
  Future<void> saveHashAlgorithm(HashAlgorithm algorithm) async {
    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'Rust 브리지 연결 후 비교 기준을 저장할 수 있습니다.',
    );
  }

  @override
  Future<void> saveScanMode(ScanMode scanMode) async {
    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'Rust 브리지 연결 후 검사 모드를 저장할 수 있습니다.',
    );
  }

  @override
  Future<List<VolumeDestination>> volumeDestinations(List<String> roots) async {
    return const [];
  }

  @override
  Future<void> saveQuarantineDestination({
    required String volumeKey,
    required String targetPath,
  }) async {
    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'Rust 브리지 연결 후 보관 폴더를 저장할 수 있습니다.',
    );
  }

  @override
  Future<int> clearQuarantineDestinations() async {
    return 0;
  }

  @override
  Future<void> revealFile(String path) async {
    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'Rust 브리지 연결 후 파일 위치를 열 수 있습니다.',
    );
  }

  @override
  Future<void> cancelActiveScan() async {}
}
