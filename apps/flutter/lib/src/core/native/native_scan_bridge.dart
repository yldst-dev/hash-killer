import '../../features/scan/domain/activity_event.dart';
import '../../features/scan/domain/hash_algorithm.dart';
import '../../features/scan/domain/scan_report.dart';
import '../../features/scan/domain/scan_request.dart';
import '../../features/scan/domain/scan_mode.dart';
import '../../features/scan/domain/volume_destination.dart';

class NativeScanSettings {
  const NativeScanSettings({
    required this.cacheLimitMb,
    required this.cacheLimitConfigured,
    required this.algorithm,
    required this.algorithmConfigured,
    required this.scanMode,
    required this.scanModeConfigured,
  });

  final int cacheLimitMb;
  final bool cacheLimitConfigured;
  final HashAlgorithm algorithm;
  final bool algorithmConfigured;
  final ScanMode scanMode;
  final bool scanModeConfigured;
}

abstract interface class NativeScanBridge {
  Future<ScanReport> runScan(
    ScanRequest request, {
    required void Function(ActivityEvent event) onActivity,
  });

  Future<NativeScanSettings> loadSettings();

  Future<int> saveCacheLimit(int value);

  Future<int> clearCache();

  Future<void> saveHashAlgorithm(HashAlgorithm algorithm);

  Future<void> saveScanMode(ScanMode scanMode);

  Future<List<VolumeDestination>> volumeDestinations(List<String> roots);

  Future<void> saveQuarantineDestination({
    required String volumeKey,
    required String targetPath,
  });

  Future<int> clearQuarantineDestinations();

  Future<void> revealFile(String path);

  Future<void> cancelActiveScan();
}
