import 'activity_event.dart';
import 'hash_algorithm.dart';
import 'scan_report.dart';
import 'scan_request.dart';
import 'scan_mode.dart';
import 'volume_destination.dart';

class ScanSettings {
  const ScanSettings({
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

abstract interface class ScanRepository {
  Future<ScanReport> run(
    ScanRequest request, {
    required void Function(ActivityEvent event) onActivity,
  });

  Future<ScanSettings> loadSettings();

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
