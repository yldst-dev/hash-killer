import '../../../core/native/native_scan_bridge.dart';
import '../domain/activity_event.dart';
import '../domain/hash_algorithm.dart';
import '../domain/scan_report.dart';
import '../domain/scan_repository.dart';
import '../domain/scan_request.dart';
import '../domain/scan_mode.dart';
import '../domain/volume_destination.dart';

class NativeScanRepository implements ScanRepository {
  const NativeScanRepository(this.bridge);

  final NativeScanBridge bridge;

  @override
  Future<ScanReport> run(
    ScanRequest request, {
    required void Function(ActivityEvent event) onActivity,
  }) {
    return bridge.runScan(request, onActivity: onActivity);
  }

  @override
  Future<ScanSettings> loadSettings() async {
    final settings = await bridge.loadSettings();

    return ScanSettings(
      cacheLimitMb: settings.cacheLimitMb,
      cacheLimitConfigured: settings.cacheLimitConfigured,
      algorithm: settings.algorithm,
      algorithmConfigured: settings.algorithmConfigured,
      scanMode: settings.scanMode,
      scanModeConfigured: settings.scanModeConfigured,
    );
  }

  @override
  Future<int> saveCacheLimit(int value) {
    return bridge.saveCacheLimit(value);
  }

  @override
  Future<int> clearCache() {
    return bridge.clearCache();
  }

  @override
  Future<void> saveHashAlgorithm(HashAlgorithm algorithm) {
    return bridge.saveHashAlgorithm(algorithm);
  }

  @override
  Future<void> saveScanMode(ScanMode scanMode) {
    return bridge.saveScanMode(scanMode);
  }

  @override
  Future<List<VolumeDestination>> volumeDestinations(List<String> roots) {
    return bridge.volumeDestinations(roots);
  }

  @override
  Future<void> saveQuarantineDestination({
    required String volumeKey,
    required String targetPath,
  }) {
    return bridge.saveQuarantineDestination(
      volumeKey: volumeKey,
      targetPath: targetPath,
    );
  }

  @override
  Future<int> clearQuarantineDestinations() {
    return bridge.clearQuarantineDestinations();
  }

  @override
  Future<void> revealFile(String path) {
    return bridge.revealFile(path);
  }

  @override
  Future<void> cancelActiveScan() {
    return bridge.cancelActiveScan();
  }
}
