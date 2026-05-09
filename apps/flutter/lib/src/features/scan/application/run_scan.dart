import '../domain/activity_event.dart';
import '../domain/scan_failure.dart';
import '../domain/scan_report.dart';
import '../domain/scan_repository.dart';
import '../domain/scan_request.dart';

class RunScan {
  const RunScan(this.repository);

  final ScanRepository repository;

  Future<ScanReport> call(
    ScanRequest request, {
    required void Function(ActivityEvent event) onActivity,
  }) {
    final normalized = request.normalize();

    if (normalized.roots.isEmpty) {
      throw const ScanFailure(
        ScanFailureKind.invalidInput,
        '검사할 디렉터리를 입력해야 합니다.',
      );
    }

    return repository.run(normalized, onActivity: onActivity);
  }
}
