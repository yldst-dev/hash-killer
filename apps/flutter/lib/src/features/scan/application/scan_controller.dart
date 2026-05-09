import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../data/native_scan_repository.dart';
import '../data/rust_cli_scan_bridge.dart';
import '../domain/activity_event.dart';
import '../domain/hash_algorithm.dart';
import '../domain/scan_failure.dart';
import '../domain/scan_mode.dart';
import '../domain/scan_repository.dart';
import '../domain/scan_request.dart';
import '../domain/volume_destination.dart';
import 'run_scan.dart';
import 'scan_state.dart';

final scanRepositoryProvider = Provider<ScanRepository>(
  (ref) => const NativeScanRepository(RustCliScanBridge()),
);

final runScanProvider = Provider<RunScan>(
  (ref) => RunScan(ref.watch(scanRepositoryProvider)),
);

final scanControllerProvider = NotifierProvider<ScanController, ScanState>(
  ScanController.new,
);

class ScanController extends Notifier<ScanState> {
  @override
  ScanState build() {
    Future.microtask(_loadSettings);
    return const ScanState();
  }

  void setPathsText(String value) {
    final paths = value
        .split(RegExp(r'\r?\n'))
        .map((path) => path.trim())
        .where((path) => path.isNotEmpty)
        .toSet()
        .toList(growable: false);

    state = state.copyWith(
      paths: paths,
      quarantineDestinations: _buildDestinations(
        paths,
        state.quarantineDestinations,
      ),
      statusMessage: paths.isEmpty
          ? '중복 파일을 검사할 디렉터리를 선택하십시오.'
          : '${paths.length}개 디렉터리를 선택했습니다. 보관 폴더를 지정한 뒤 검사를 시작할 수 있습니다.',
      clearReport: true,
      clearError: true,
    );
    unawaited(_resetQuarantineDestinations(paths));
  }

  Future<void> pickFolders() async {
    String? folder;

    try {
      folder = await getDirectoryPath(
        confirmButtonText: '폴더 선택',
        canCreateDirectories: false,
      );
    } catch (error) {
      _setFailureMessage(error);
      return;
    }

    if (folder == null || folder.trim().isEmpty) {
      return;
    }

    final previous = state.paths;
    final paths = [...previous];

    if (!paths.contains(folder)) {
      paths.add(folder);
    }

    if (paths.length == previous.length) {
      state = state.copyWith(statusMessage: '이미 등록된 디렉터리입니다.');
      return;
    }

    state = state.copyWith(
      paths: paths,
      quarantineDestinations: _buildDestinations(
        paths,
        state.quarantineDestinations,
      ),
      statusMessage:
          '${paths.length}개 디렉터리를 선택했습니다. 보관 폴더를 지정한 뒤 검사를 시작할 수 있습니다.',
      clearReport: true,
      clearError: true,
    );
    await _resetQuarantineDestinations(paths);
  }

  void openDialog(ScanDialog dialog) {
    if (state.running && dialog == ScanDialog.cacheConfirm) {
      return;
    }

    state = state.copyWith(
      openDialog: dialog,
      cacheLimitInput: state.cacheLimitMb.toString(),
      duplicateRelationFilter: dialog == ScanDialog.duplicateRelations
          ? DuplicateRelationFilter.all
          : state.duplicateRelationFilter,
    );
  }

  void closeDialog() {
    state = state.copyWith(openDialog: ScanDialog.none);
  }

  void togglePathSelection(String path, bool selected) {
    final selection = [...state.pathRemoveSelection];

    if (selected) {
      if (!selection.contains(path)) {
        selection.add(path);
      }
    } else {
      selection.remove(path);
    }

    state = state.copyWith(pathRemoveSelection: selection);
  }

  void removeSelectedPaths() {
    final selected = state.pathRemoveSelection;

    if (selected.isEmpty) {
      return;
    }

    final previousCount = state.paths.length;
    final paths = state.paths
        .where((path) => !selected.contains(path))
        .toList(growable: false);
    final removedCount = previousCount - paths.length;
    final statusMessage = paths.isEmpty
        ? '중복 파일을 검사할 디렉터리를 선택하십시오.'
        : '$removedCount개 디렉터리를 제거했습니다. ${paths.length}개 디렉터리가 검사 목록에 남아 있습니다.';

    state = state.copyWith(
      paths: paths,
      quarantineDestinations: _buildDestinations(
        paths,
        state.quarantineDestinations,
      ),
      pathRemoveSelection: const [],
      openDialog: paths.isEmpty ? ScanDialog.none : state.openDialog,
      statusMessage: statusMessage,
      clearReport: true,
    );
    unawaited(_resetQuarantineDestinations(paths));
  }

  void setCacheLimitInput(String value) {
    state = state.copyWith(cacheLimitInput: value);
  }

  Future<void> saveCacheLimit() async {
    final value = int.tryParse(state.cacheLimitInput.trim());

    if (value == null) {
      state = state.copyWith(statusMessage: '캐시 제한은 숫자로 입력하십시오.');
      return;
    }

    if (value < 16) {
      state = state.copyWith(statusMessage: '캐시 제한은 16 MB 이상으로 입력하십시오.');
      return;
    }

    state = state.copyWith(
      cacheLimitMb: value,
      cacheLimitConfigured: true,
      openDialog: ScanDialog.none,
      statusMessage: 'SQLite 캐시 제한을 $value MB로 저장하는 중입니다.',
      clearError: true,
    );

    try {
      final pruned = await ref
          .read(scanRepositoryProvider)
          .saveCacheLimit(value);
      state = state.copyWith(
        statusMessage: pruned > 0
            ? 'SQLite 캐시 제한을 $value MB로 저장하고 오래된 해시 $pruned개를 정리했습니다.'
            : 'SQLite 캐시 제한을 $value MB로 저장했습니다.',
        clearError: true,
      );
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  Future<void> clearCache() async {
    if (state.running) {
      state = state.copyWith(
        openDialog: ScanDialog.none,
        statusMessage: '검사 중에는 SQLite 캐시를 삭제할 수 없습니다.',
      );
      return;
    }

    try {
      final removed = await ref.read(scanRepositoryProvider).clearCache();
      state = state.copyWith(
        report: null,
        openDialog: ScanDialog.none,
        statusMessage: removed > 0
            ? 'SQLite 캐시 파일 $removed개를 삭제했습니다.'
            : '삭제할 SQLite 캐시 파일이 없습니다.',
        clearReport: true,
        clearError: true,
      );
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  Future<void> setAlgorithm(HashAlgorithm algorithm) async {
    state = state.copyWith(
      algorithm: algorithm,
      algorithmConfigured: true,
      statusMessage: '비교 기준을 ${algorithm.label}로 저장하는 중입니다.',
      clearError: true,
    );

    try {
      await ref.read(scanRepositoryProvider).saveHashAlgorithm(algorithm);
      state = state.copyWith(
        statusMessage: '비교 기준을 ${algorithm.label}로 저장했습니다.',
        clearError: true,
      );
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  Future<void> setScanMode(ScanMode scanMode) async {
    state = state.copyWith(
      scanMode: scanMode,
      scanModeConfigured: true,
      statusMessage: '검사 모드를 ${scanMode.label}로 저장하는 중입니다.',
      clearError: true,
    );

    try {
      await ref.read(scanRepositoryProvider).saveScanMode(scanMode);
      state = state.copyWith(
        statusMessage: '검사 모드를 ${scanMode.label}로 저장했습니다.',
        clearError: true,
      );
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  Future<void> pickQuarantineDestination(VolumeDestination destination) async {
    if (state.paths.isEmpty) {
      state = state.copyWith(statusMessage: '검사 경로를 먼저 선택하십시오.');
      return;
    }

    final target = await getDirectoryPath(
      initialDirectory: destination.configured
          ? destination.targetPath
          : destination.rootPath,
      confirmButtonText: '폴더 선택',
      canCreateDirectories: true,
    );

    if (target == null) {
      return;
    }

    try {
      final nativeDestination = await _nativeDestinationFor(destination);
      await ref
          .read(scanRepositoryProvider)
          .saveQuarantineDestination(
            volumeKey: nativeDestination.volumeKey,
            targetPath: target,
          );
      final destinations = await ref
          .read(scanRepositoryProvider)
          .volumeDestinations(state.paths);
      state = state.copyWith(
        quarantineDestinations: destinations,
        statusMessage: '보관 폴더를 저장했습니다.',
        clearError: true,
      );
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  void setDuplicateRelationFilter(DuplicateRelationFilter filter) {
    state = state.copyWith(duplicateRelationFilter: filter);
  }

  Future<void> exportActivityLog() async {
    final events = state.activityLogEvents;

    if (events.isEmpty) {
      state = state.copyWith(statusMessage: '저장할 실시간 작업 로그가 없습니다.');
      return;
    }

    await _saveTextFile(
      suggestedName: 'hash-killer-activity.log',
      contents: _formatActivityLog(events),
      emptyMessage: '저장할 실시간 작업 로그가 없습니다.',
      successMessage: '실시간 작업 로그를 저장했습니다.',
    );
  }

  Future<void> exportDuplicateRelationsLog() async {
    final relations = state.report?.duplicateRelations ?? const [];

    if (relations.isEmpty) {
      state = state.copyWith(statusMessage: '저장할 중복 관계 로그가 없습니다.');
      return;
    }

    await _saveTextFile(
      suggestedName: 'hash-killer-duplicates.log',
      contents: relations
          .map(
            (relation) =>
                '${relation.kind.id}\t${relation.size}\t${relation.hash}\t${relation.originalPath}\t${relation.currentDuplicatePath}',
          )
          .join('\n'),
      emptyMessage: '저장할 중복 관계 로그가 없습니다.',
      successMessage: '중복 관계 로그를 저장했습니다.',
    );
  }

  Future<void> revealFileLocation(String path) async {
    try {
      await ref.read(scanRepositoryProvider).revealFile(path);
      state = state.copyWith(statusMessage: '파일 위치를 열었습니다.', clearError: true);
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  Future<void> stop() async {
    await ref.read(scanRepositoryProvider).cancelActiveScan();
    state = state.copyWith(
      status: ScanStatus.cancelled,
      statusMessage: '사용자가 검사를 중지했습니다.',
    );
  }

  Future<void> run() async {
    if (state.running) {
      stop();
      return;
    }

    if (state.paths.isEmpty) {
      state = state.copyWith(statusMessage: '중복 파일을 검사할 디렉터리를 선택하십시오.');
      return;
    }

    if (state.quarantineRequired) {
      state = state.copyWith(statusMessage: '모든 디스크의 보관 폴더를 먼저 지정하십시오.');
      return;
    }

    final request = ScanRequest(
      roots: state.paths,
      algorithm: state.algorithm,
      scanMode: state.scanMode,
    );

    state = state.copyWith(
      status: ScanStatus.loading,
      openDialog: ScanDialog.none,
      activityEvents: const [],
      activityLogEvents: const [],
      statusMessage: '검사 및 중복 제거를 실행 중입니다.',
      clearError: true,
      clearReport: true,
      clearProgress: true,
    );

    try {
      final report = await ref.read(runScanProvider)(
        request,
        onActivity: _pushActivityEvent,
      );

      _pushActivityEvent(
        const ActivityEvent(stage: '완료', detail: '검사가 완료되었습니다.'),
      );
      state = state.copyWith(
        status: ScanStatus.success,
        report: report,
        paths: const [],
        quarantineDestinations: const [],
        statusMessage: '완료되었습니다. 검사 경로와 보관 폴더 설정을 초기화했습니다.',
      );
    } on ScanFailure catch (failure) {
      _pushActivityEvent(ActivityEvent(stage: '오류', detail: failure.message));
      state = state.copyWith(
        status: failure.kind == ScanFailureKind.cancelled
            ? ScanStatus.cancelled
            : ScanStatus.failure,
        errorMessage: failure.message,
        statusMessage: failure.message,
      );
    } catch (error) {
      final message = error.toString();
      _pushActivityEvent(ActivityEvent(stage: '오류', detail: message));
      state = state.copyWith(
        status: ScanStatus.failure,
        errorMessage: message,
        statusMessage: message,
      );
    }
  }

  void _pushActivityEvent(ActivityEvent event) {
    final logEvents = [...state.activityLogEvents, event];
    final visibleEvents = logEvents.length > 200
        ? logEvents.sublist(logEvents.length - 200)
        : logEvents;
    final progress =
        event.progress == null || event.completed == null || event.total == null
        ? state.scanProgress
        : ScanProgressState(
            progress: event.progress!,
            completed: event.completed!,
            total: event.total!,
          );

    state = state.copyWith(
      activityEvents: visibleEvents,
      activityLogEvents: logEvents,
      scanProgress: progress,
    );
  }

  Future<void> _loadSettings() async {
    try {
      final settings = await ref.read(scanRepositoryProvider).loadSettings();
      state = state.copyWith(
        cacheLimitMb: settings.cacheLimitMb,
        cacheLimitInput: settings.cacheLimitMb.toString(),
        cacheLimitConfigured: settings.cacheLimitConfigured,
        algorithm: settings.algorithm,
        algorithmConfigured: settings.algorithmConfigured,
        scanMode: settings.scanMode,
        scanModeConfigured: settings.scanModeConfigured,
        clearError: true,
      );
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  Future<void> _refreshQuarantineDestinations(List<String> paths) async {
    if (paths.isEmpty) {
      return;
    }

    try {
      final destinations = await ref
          .read(scanRepositoryProvider)
          .volumeDestinations(paths);

      if (_samePaths(paths, state.paths)) {
        state = state.copyWith(
          quarantineDestinations: destinations,
          clearError: true,
        );
      }
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  Future<void> _resetQuarantineDestinations(List<String> paths) async {
    if (paths.isEmpty) {
      return;
    }

    try {
      await ref.read(scanRepositoryProvider).clearQuarantineDestinations();
    } catch (error) {
      _setFailureMessage(error);
      return;
    }

    await _refreshQuarantineDestinations(paths);
  }

  Future<void> _saveTextFile({
    required String suggestedName,
    required String contents,
    required String emptyMessage,
    required String successMessage,
  }) async {
    if (contents.trim().isEmpty) {
      state = state.copyWith(statusMessage: emptyMessage);
      return;
    }

    try {
      final location = await getSaveLocation(suggestedName: suggestedName);

      if (location == null) {
        return;
      }

      await File(location.path).writeAsString(contents);
      state = state.copyWith(statusMessage: successMessage, clearError: true);
    } catch (error) {
      _setFailureMessage(error);
    }
  }

  String _formatActivityLog(List<ActivityEvent> events) {
    return events
        .map((event) {
          final progress = event.progress == null
              ? ''
              : '\t${(event.progress! * 100).toStringAsFixed(1)}%';
          final path = event.path == null ? '' : '\t${event.path}';
          return '${event.stage}\t${event.detail}$progress$path';
        })
        .join('\n');
  }

  bool _samePaths(List<String> left, List<String> right) {
    if (left.length != right.length) {
      return false;
    }

    for (var index = 0; index < left.length; index += 1) {
      if (left[index] != right[index]) {
        return false;
      }
    }

    return true;
  }

  void _setFailureMessage(Object error) {
    final message = error is ScanFailure ? error.message : error.toString();
    state = state.copyWith(statusMessage: message, errorMessage: message);
  }

  Future<VolumeDestination> _nativeDestinationFor(
    VolumeDestination destination,
  ) async {
    final destinations = await ref
        .read(scanRepositoryProvider)
        .volumeDestinations(state.paths);

    return destinations.firstWhere(
      (current) =>
          current.volumeKey == destination.volumeKey ||
          current.rootPaths.any(destination.rootPaths.contains) ||
          destination.rootPaths.any(current.rootPaths.contains),
      orElse: () => destination,
    );
  }

  List<VolumeDestination> _buildDestinations(
    List<String> paths,
    List<VolumeDestination> previous,
  ) {
    if (paths.isEmpty) {
      return const [];
    }

    final previousByKey = {
      for (final destination in previous) destination.volumeKey: destination,
    };
    final grouped = <String, List<String>>{};

    for (final path in paths) {
      final key = _volumeKey(path);
      grouped.putIfAbsent(key, () => []).add(path);
    }

    return grouped.entries
        .map((entry) {
          final rootPaths = [...entry.value]..sort();
          final rootPath = rootPaths.first;
          final previous = previousByKey[entry.key];

          return VolumeDestination(
            volumeKey: entry.key,
            rootPath: rootPath,
            rootPaths: rootPaths,
            targetPath: previous?.targetPath ?? '지정되지 않음',
            configured: previous?.configured ?? false,
          );
        })
        .toList(growable: false)
      ..sort((left, right) => left.rootPath.compareTo(right.rootPath));
  }

  String _volumeKey(String path) {
    final normalized = path.replaceAll('\\', '/');
    final parts = normalized
        .split('/')
        .where((part) => part.isNotEmpty)
        .toList();

    if (normalized.startsWith('/') && parts.isNotEmpty) {
      return '/${parts.first}';
    }

    if (normalized.length >= 2 && normalized[1] == ':') {
      return normalized.substring(0, 2).toUpperCase();
    }

    return parts.isEmpty ? normalized : parts.first;
  }
}
