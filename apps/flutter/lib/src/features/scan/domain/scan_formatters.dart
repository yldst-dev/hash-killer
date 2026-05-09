String formatBytes(int bytes) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB'];
  var size = bytes.toDouble();
  var unit = 0;

  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }

  if (unit == 0) {
    return '$bytes ${units[unit]}';
  }

  return '${size.toStringAsFixed(2)} ${units[unit]}';
}

String compactPathLabel(String path) {
  const maxChars = 14;
  final normalized = path.replaceAll('\\', '/');

  if (normalized.runes.length <= maxChars) {
    return normalized;
  }

  return '${String.fromCharCodes(normalized.runes.take(maxChars - 3))}...';
}

String compactHashLabel(String hash) {
  if (hash.runes.length <= 14) {
    return hash;
  }

  return '${String.fromCharCodes(hash.runes.take(14))}...';
}
