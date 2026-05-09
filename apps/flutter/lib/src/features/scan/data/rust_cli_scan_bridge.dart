import 'dart:async';
import 'dart:convert';
import 'dart:io';

import '../../../core/native/native_scan_bridge.dart';
import '../domain/activity_event.dart';
import '../domain/duplicate_relation.dart';
import '../domain/hash_algorithm.dart';
import '../domain/scan_failure.dart';
import '../domain/scan_report.dart';
import '../domain/scan_request.dart';
import '../domain/scan_mode.dart';
import '../domain/volume_destination.dart';

class RustCliScanBridge implements NativeScanBridge {
  const RustCliScanBridge();

  static Process? _activeScan;
  static bool _cancelRequested = false;

  @override
  Future<ScanReport> runScan(
    ScanRequest request, {
    required void Function(ActivityEvent event) onActivity,
  }) async {
    final process = await _startProcess();
    _activeScan = process;
    _cancelRequested = false;
    final stderrBuffer = StringBuffer();
    final stderrDone = process.stderr
        .transform(utf8.decoder)
        .listen(stderrBuffer.write)
        .asFuture<void>();
    ScanReport? report;
    ScanFailure? failure;

    process.stdin.writeln(
      jsonEncode({
        'command': 'run_scan',
        'roots': request.roots,
        'algorithm': request.algorithm.id,
        'scan_mode': request.scanMode.id,
      }),
    );
    await process.stdin.close();

    await for (final line
        in process.stdout
            .transform(utf8.decoder)
            .transform(const LineSplitter())) {
      if (line.trim().isEmpty) {
        continue;
      }

      final payload = _decodeObject(line);
      switch (payload['type']) {
        case 'activity':
          onActivity(_activityEvent(payload['event'] as Map<String, Object?>));
        case 'report':
          report = _scanReport(payload['report'] as Map<String, Object?>);
        case 'error':
          failure = ScanFailure(
            ScanFailureKind.nativeFailure,
            payload['message'] as String? ?? 'Rust 코어에서 오류가 발생했습니다.',
          );
      }
    }

    final exitCode = await process.exitCode;
    await stderrDone;
    final cancelled = _cancelRequested;
    _activeScan = null;
    _cancelRequested = false;

    if (failure != null) {
      throw failure;
    }

    if (cancelled) {
      throw const ScanFailure(ScanFailureKind.cancelled, '사용자가 검사를 중지했습니다.');
    }

    if (exitCode != 0) {
      throw ScanFailure(
        ScanFailureKind.nativeFailure,
        _processError(stderrBuffer),
      );
    }

    final result = report;
    if (result == null) {
      throw const ScanFailure(
        ScanFailureKind.nativeFailure,
        'Rust 코어가 검사 결과를 반환하지 않았습니다.',
      );
    }

    return result;
  }

  @override
  Future<NativeScanSettings> loadSettings() async {
    final data = await _request({'command': 'load_settings'});

    return NativeScanSettings(
      cacheLimitMb: _intValue(data['cache_limit_mb']),
      cacheLimitConfigured: data['cache_limit_configured'] == true,
      algorithm: HashAlgorithm.fromId(data['algorithm'] as String? ?? ''),
      algorithmConfigured: data['algorithm_configured'] == true,
      scanMode: ScanMode.fromId(data['scan_mode'] as String? ?? ''),
      scanModeConfigured: data['scan_mode_configured'] == true,
    );
  }

  @override
  Future<int> saveCacheLimit(int value) async {
    final data = await _request({
      'command': 'save_cache_limit',
      'value': value,
    });
    return _intValue(data['pruned']);
  }

  @override
  Future<int> clearCache() async {
    final data = await _request({'command': 'clear_cache'});
    return _intValue(data['removed']);
  }

  @override
  Future<void> saveHashAlgorithm(HashAlgorithm algorithm) async {
    await _request({'command': 'save_hash_algorithm', 'value': algorithm.id});
  }

  @override
  Future<void> saveScanMode(ScanMode scanMode) async {
    await _request({'command': 'save_scan_mode', 'value': scanMode.id});
  }

  @override
  Future<List<VolumeDestination>> volumeDestinations(List<String> roots) async {
    final data = await _request({
      'command': 'volume_destinations',
      'roots': roots,
    });

    return (data['items'] as List<Object?>? ??
            data['data'] as List<Object?>? ??
            const [])
        .whereType<Map<String, Object?>>()
        .map(_volumeDestination)
        .toList(growable: false);
  }

  @override
  Future<void> saveQuarantineDestination({
    required String volumeKey,
    required String targetPath,
  }) async {
    await _request({
      'command': 'save_quarantine_destination',
      'volume_key': volumeKey,
      'target_path': targetPath,
    });
  }

  @override
  Future<int> clearQuarantineDestinations() async {
    final data = await _request({'command': 'clear_quarantine_destinations'});
    return _intValue(data['count']);
  }

  @override
  Future<void> revealFile(String path) async {
    await _request({'command': 'reveal_file', 'path': path});
  }

  @override
  Future<void> cancelActiveScan() async {
    final process = _activeScan;

    if (process == null) {
      return;
    }

    _cancelRequested = true;
    process.kill();
  }

  Future<Map<String, Object?>> _request(Map<String, Object?> request) async {
    final process = await _startProcess();
    final stderrBuffer = StringBuffer();
    final output = <Map<String, Object?>>[];
    final stderrDone = process.stderr
        .transform(utf8.decoder)
        .listen(stderrBuffer.write)
        .asFuture<void>();

    process.stdin.writeln(jsonEncode(request));
    await process.stdin.close();

    await for (final line
        in process.stdout
            .transform(utf8.decoder)
            .transform(const LineSplitter())) {
      if (line.trim().isEmpty) {
        continue;
      }
      output.add(_decodeObject(line));
    }

    final exitCode = await process.exitCode;
    await stderrDone;

    final error = output
        .where((payload) => payload['type'] == 'error')
        .map((payload) => payload['message'] as String?)
        .whereType<String>()
        .firstOrNull;

    if (error != null) {
      throw ScanFailure(ScanFailureKind.nativeFailure, error);
    }

    if (exitCode != 0) {
      throw ScanFailure(
        ScanFailureKind.nativeFailure,
        _processError(stderrBuffer),
      );
    }

    final result = output
        .where((payload) => payload['type'] == 'result')
        .map((payload) => payload['data'])
        .firstOrNull;

    if (result is Map<String, Object?>) {
      return result;
    }

    if (result is List<Object?>) {
      return {'items': result};
    }

    return {'data': result};
  }

  Future<Process> _startProcess() async {
    final command = await _command();

    try {
      return Process.start(
        command.executable,
        command.arguments,
        workingDirectory: command.workingDirectory,
      );
    } on Object catch (error) {
      throw ScanFailure(
        ScanFailureKind.bridgeUnavailable,
        'Rust 코어 실행에 실패했습니다: $error',
      );
    }
  }

  Future<_BridgeCommand> _command() async {
    final explicit = Platform.environment['HASH_KILLER_BRIDGE'];

    if (explicit != null && explicit.trim().isNotEmpty) {
      return _BridgeCommand(explicit, const ['--bridge-json'], null);
    }

    final bundled = await _bundledCommand();
    if (bundled != null) {
      return bundled;
    }

    final root = await _findRepositoryRoot();
    final binary = File(_debugBinaryPath(root));

    if (await _usableBinary(binary, root)) {
      return _BridgeCommand(binary.path, const ['--bridge-json'], root.path);
    }

    return _BridgeCommand('cargo', const [
      'run',
      '--quiet',
      '--features',
      'desktop',
      '--',
      '--bridge-json',
    ], root.path);
  }

  Future<_BridgeCommand?> _bundledCommand() async {
    final executable = File(Platform.resolvedExecutable);
    final executableDirectory = executable.parent;
    final candidates = [
      File(
        '${executableDirectory.path}${Platform.pathSeparator}${_binaryName()}',
      ),
      if (Platform.isMacOS)
        File(
          '${executableDirectory.parent.path}${Platform.pathSeparator}Resources${Platform.pathSeparator}${_binaryName()}',
        ),
    ];

    for (final candidate in candidates) {
      if (await _usableExecutable(candidate)) {
        return _BridgeCommand(candidate.path, const [
          '--bridge-json',
        ], candidate.parent.path);
      }
    }

    return null;
  }

  String _debugBinaryPath(Directory root) {
    return [
      root.path,
      'target',
      'debug',
      _binaryName(),
    ].join(Platform.pathSeparator);
  }

  String _binaryName() {
    return Platform.isWindows ? 'hash-killer.exe' : 'hash-killer';
  }

  Future<bool> _usableBinary(File binary, Directory root) async {
    if (!await _usableExecutable(binary)) {
      return false;
    }

    final binaryTime = await binary.lastModified();
    final paths = [
      'Cargo.toml',
      'Cargo.lock',
      'src${Platform.pathSeparator}main.rs',
      'src${Platform.pathSeparator}native_bridge.rs',
    ];

    for (final path in paths) {
      final file = File('${root.path}${Platform.pathSeparator}$path');
      if (await file.exists() &&
          (await file.lastModified()).isAfter(binaryTime)) {
        return false;
      }
    }

    return true;
  }

  Future<bool> _usableExecutable(File binary) async {
    if (!await binary.exists()) {
      return false;
    }

    final stat = await binary.stat();
    return stat.type == FileSystemEntityType.file;
  }

  Future<Directory> _findRepositoryRoot() async {
    final candidates = <Directory>[
      Directory.current,
      if (Platform.script.scheme == 'file')
        File(Platform.script.toFilePath()).parent,
      File(Platform.resolvedExecutable).parent,
    ];

    for (final candidate in candidates) {
      final root = await _walkToRepositoryRoot(candidate);
      if (root != null) {
        return root;
      }
    }

    throw const ScanFailure(
      ScanFailureKind.bridgeUnavailable,
      'hash-killer Rust 프로젝트 루트를 찾을 수 없습니다.',
    );
  }

  Future<Directory?> _walkToRepositoryRoot(Directory start) async {
    var current = start.absolute;

    while (true) {
      final cargoToml = File(
        '${current.path}${Platform.pathSeparator}Cargo.toml',
      );
      final source = File(
        '${current.path}${Platform.pathSeparator}src${Platform.pathSeparator}main.rs',
      );

      if (await cargoToml.exists() && await source.exists()) {
        final contents = await cargoToml.readAsString();
        if (contents.contains('name = "hash-killer"')) {
          return current;
        }
      }

      final parent = current.parent;
      if (parent.path == current.path) {
        return null;
      }
      current = parent;
    }
  }

  Map<String, Object?> _decodeObject(String line) {
    final decoded = jsonDecode(line);

    if (decoded is Map<String, Object?>) {
      return decoded;
    }

    throw const ScanFailure(
      ScanFailureKind.nativeFailure,
      'Rust 코어 응답 형식이 올바르지 않습니다.',
    );
  }

  String _processError(StringBuffer stderrBuffer) {
    final stderr = stderrBuffer.toString().trim();
    return stderr.isEmpty ? 'Rust 코어 프로세스가 실패했습니다.' : stderr;
  }

  ActivityEvent _activityEvent(Map<String, Object?> json) {
    return ActivityEvent(
      stage: json['stage'] as String? ?? '',
      detail: json['detail'] as String? ?? '',
      path: json['path'] as String?,
      progress: _doubleValue(json['progress']),
      completed: _nullableIntValue(json['completed']),
      total: _nullableIntValue(json['total']),
    );
  }

  ScanReport _scanReport(Map<String, Object?> json) {
    return ScanReport(
      scannedFiles: _intValue(json['scanned_files']),
      candidateFiles: _intValue(json['candidate_files']),
      hashedFiles: _intValue(json['hashed_files']),
      reusedHashes: _intValue(json['reused_hashes']),
      duplicateGroups: _intValue(json['duplicate_groups']),
      deletedFiles: _intValue(json['deleted_files']),
      keptFiles: _intValue(json['kept_files']),
      reclaimedBytes: _intValue(json['reclaimed_bytes']),
      failedFiles: (json['failed_files'] as List<Object?>? ?? const [])
          .whereType<String>()
          .toList(growable: false),
      duplicateRelations:
          (json['duplicate_relations'] as List<Object?>? ?? const [])
              .whereType<Map<String, Object?>>()
              .map(_duplicateRelation)
              .toList(growable: false),
    );
  }

  DuplicateRelation _duplicateRelation(Map<String, Object?> json) {
    return DuplicateRelation(
      originalPath: json['original_path'] as String? ?? '',
      duplicatePath: json['duplicate_path'] as String? ?? '',
      currentDuplicatePath: json['current_duplicate_path'] as String? ?? '',
      size: _intValue(json['size']),
      hash: json['hash'] as String? ?? '',
      kind: _relationKind(json['kind'] as String? ?? ''),
    );
  }

  DuplicateRelationKind _relationKind(String id) {
    return DuplicateRelationKind.values.firstWhere(
      (kind) => kind.id == id,
      orElse: () => DuplicateRelationKind.sameSizeAndHash,
    );
  }

  VolumeDestination _volumeDestination(Map<String, Object?> json) {
    return VolumeDestination(
      volumeKey: json['volume_key'] as String? ?? '',
      rootPath: json['root_path'] as String? ?? '',
      rootPaths: (json['root_paths'] as List<Object?>? ?? const [])
          .whereType<String>()
          .toList(growable: false),
      targetPath: json['target_path'] as String? ?? '지정되지 않음',
      configured: json['configured'] == true,
    );
  }

  int _intValue(Object? value) {
    return switch (value) {
      int value => value,
      double value => value.toInt(),
      String value => int.tryParse(value) ?? 0,
      _ => 0,
    };
  }

  int? _nullableIntValue(Object? value) {
    if (value == null) {
      return null;
    }
    return _intValue(value);
  }

  double? _doubleValue(Object? value) {
    return switch (value) {
      double value => value,
      int value => value.toDouble(),
      String value => double.tryParse(value),
      _ => null,
    };
  }
}

class _BridgeCommand {
  const _BridgeCommand(this.executable, this.arguments, this.workingDirectory);

  final String executable;
  final List<String> arguments;
  final String? workingDirectory;
}
