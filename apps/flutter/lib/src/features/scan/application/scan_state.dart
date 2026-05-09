import '../domain/activity_event.dart';
import '../domain/duplicate_relation.dart';
import '../domain/hash_algorithm.dart';
import '../domain/scan_mode.dart';
import '../domain/scan_report.dart';
import '../domain/volume_destination.dart';

enum ScanStatus { idle, loading, success, failure, cancelled }

enum ScanDialog {
  none,
  cacheConfirm,
  scanConfirm,
  pathList,
  cacheSettings,
  scanModeSettings,
  algorithmSettings,
  quarantineSettings,
  duplicateRelations,
}

enum DuplicateRelationFilter {
  all('전체'),
  sameNameAndSize('같은 이름+용량'),
  sameSizeAndHash('다른 이름+용량+해시');

  const DuplicateRelationFilter(this.label);

  final String label;

  bool matches(DuplicateRelation relation) {
    return switch (this) {
      DuplicateRelationFilter.all => true,
      DuplicateRelationFilter.sameNameAndSize =>
        relation.kind == DuplicateRelationKind.sameNameAndSize,
      DuplicateRelationFilter.sameSizeAndHash =>
        relation.kind == DuplicateRelationKind.sameSizeAndHash,
    };
  }
}

class ScanProgressState {
  const ScanProgressState({
    required this.progress,
    required this.completed,
    required this.total,
  });

  final double progress;
  final int completed;
  final int total;
}

class ScanState {
  const ScanState({
    this.status = ScanStatus.idle,
    this.statusMessage = '중복 파일을 검사할 디렉터리를 선택하십시오.',
    this.paths = const [],
    this.cacheLimitMb = 256,
    this.cacheLimitConfigured = false,
    this.cacheLimitInput = '256',
    this.algorithm = HashAlgorithm.md5,
    this.algorithmConfigured = false,
    this.scanMode = ScanMode.fullHash,
    this.scanModeConfigured = false,
    this.activityEvents = const [],
    this.activityLogEvents = const [],
    this.scanProgress,
    this.report,
    this.errorMessage,
    this.openDialog = ScanDialog.none,
    this.pathRemoveSelection = const [],
    this.duplicateRelationFilter = DuplicateRelationFilter.all,
    this.quarantineDestinations = const [],
  });

  final ScanStatus status;
  final String statusMessage;
  final List<String> paths;
  final int cacheLimitMb;
  final bool cacheLimitConfigured;
  final String cacheLimitInput;
  final HashAlgorithm algorithm;
  final bool algorithmConfigured;
  final ScanMode scanMode;
  final bool scanModeConfigured;
  final List<ActivityEvent> activityEvents;
  final List<ActivityEvent> activityLogEvents;
  final ScanProgressState? scanProgress;
  final ScanReport? report;
  final String? errorMessage;
  final ScanDialog openDialog;
  final List<String> pathRemoveSelection;
  final DuplicateRelationFilter duplicateRelationFilter;
  final List<VolumeDestination> quarantineDestinations;

  bool get running => status == ScanStatus.loading;
  bool get hasPaths => paths.isNotEmpty;
  ActivityEvent? get lastEvent =>
      activityEvents.isEmpty ? null : activityEvents.last;
  bool get quarantineRequired =>
      hasPaths &&
      quarantineDestinations.any((destination) => !destination.configured);
  bool get canRun => !running && hasPaths && !quarantineRequired;

  double get progress {
    final currentReport = report;
    if (currentReport != null) {
      if (status == ScanStatus.success) {
        return 1;
      }

      return currentReport.progress;
    }

    return scanProgress?.progress ?? 0;
  }

  int get processed {
    final currentReport = report;
    if (currentReport != null) {
      return currentReport.scannedFiles;
    }

    return scanProgress?.completed ?? 0;
  }

  int get total {
    final currentReport = report;
    if (currentReport != null) {
      return currentReport.scannedFiles;
    }

    return scanProgress?.total ?? 0;
  }

  List<DuplicateRelation> get filteredDuplicateRelations {
    return (report?.duplicateRelations ?? const [])
        .where(duplicateRelationFilter.matches)
        .toList(growable: false);
  }

  ScanState copyWith({
    ScanStatus? status,
    String? statusMessage,
    List<String>? paths,
    int? cacheLimitMb,
    bool? cacheLimitConfigured,
    String? cacheLimitInput,
    HashAlgorithm? algorithm,
    bool? algorithmConfigured,
    ScanMode? scanMode,
    bool? scanModeConfigured,
    List<ActivityEvent>? activityEvents,
    List<ActivityEvent>? activityLogEvents,
    ScanProgressState? scanProgress,
    ScanReport? report,
    String? errorMessage,
    ScanDialog? openDialog,
    List<String>? pathRemoveSelection,
    DuplicateRelationFilter? duplicateRelationFilter,
    List<VolumeDestination>? quarantineDestinations,
    bool clearError = false,
    bool clearReport = false,
    bool clearProgress = false,
  }) {
    return ScanState(
      status: status ?? this.status,
      statusMessage: statusMessage ?? this.statusMessage,
      paths: paths ?? this.paths,
      cacheLimitMb: cacheLimitMb ?? this.cacheLimitMb,
      cacheLimitConfigured: cacheLimitConfigured ?? this.cacheLimitConfigured,
      cacheLimitInput: cacheLimitInput ?? this.cacheLimitInput,
      algorithm: algorithm ?? this.algorithm,
      algorithmConfigured: algorithmConfigured ?? this.algorithmConfigured,
      scanMode: scanMode ?? this.scanMode,
      scanModeConfigured: scanModeConfigured ?? this.scanModeConfigured,
      activityEvents: activityEvents ?? this.activityEvents,
      activityLogEvents: activityLogEvents ?? this.activityLogEvents,
      scanProgress: clearProgress ? null : scanProgress ?? this.scanProgress,
      report: clearReport ? null : report ?? this.report,
      errorMessage: clearError ? null : errorMessage ?? this.errorMessage,
      openDialog: openDialog ?? this.openDialog,
      pathRemoveSelection: pathRemoveSelection ?? this.pathRemoveSelection,
      duplicateRelationFilter:
          duplicateRelationFilter ?? this.duplicateRelationFilter,
      quarantineDestinations:
          quarantineDestinations ?? this.quarantineDestinations,
    );
  }
}
