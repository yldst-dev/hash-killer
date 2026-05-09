class ActivityEvent {
  const ActivityEvent({
    required this.stage,
    required this.detail,
    this.path,
    this.progress,
    this.completed,
    this.total,
  });

  final String stage;
  final String detail;
  final String? path;
  final double? progress;
  final int? completed;
  final int? total;
}
