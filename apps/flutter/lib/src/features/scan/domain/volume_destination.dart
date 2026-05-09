class VolumeDestination {
  const VolumeDestination({
    required this.volumeKey,
    required this.rootPath,
    required this.rootPaths,
    required this.targetPath,
    required this.configured,
  });

  final String volumeKey;
  final String rootPath;
  final List<String> rootPaths;
  final String targetPath;
  final bool configured;

  VolumeDestination copyWith({String? targetPath, bool? configured}) {
    return VolumeDestination(
      volumeKey: volumeKey,
      rootPath: rootPath,
      rootPaths: rootPaths,
      targetPath: targetPath ?? this.targetPath,
      configured: configured ?? this.configured,
    );
  }
}
