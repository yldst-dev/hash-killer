import 'duplicate_relation.dart';

class ScanReport {
  const ScanReport({
    required this.scannedFiles,
    required this.candidateFiles,
    required this.hashedFiles,
    required this.reusedHashes,
    required this.duplicateGroups,
    required this.deletedFiles,
    required this.keptFiles,
    required this.reclaimedBytes,
    required this.failedFiles,
    required this.duplicateRelations,
  });

  const ScanReport.empty()
    : scannedFiles = 0,
      candidateFiles = 0,
      hashedFiles = 0,
      reusedHashes = 0,
      duplicateGroups = 0,
      deletedFiles = 0,
      keptFiles = 0,
      reclaimedBytes = 0,
      failedFiles = const [],
      duplicateRelations = const [];

  final int scannedFiles;
  final int candidateFiles;
  final int hashedFiles;
  final int reusedHashes;
  final int duplicateGroups;
  final int deletedFiles;
  final int keptFiles;
  final int reclaimedBytes;
  final List<String> failedFiles;
  final List<DuplicateRelation> duplicateRelations;

  double get progress {
    if (candidateFiles > 0) {
      final processed = hashedFiles + reusedHashes;
      return (processed / candidateFiles).clamp(0, 1).toDouble();
    }

    if (scannedFiles > 0) {
      return 1;
    }

    return 0;
  }
}
