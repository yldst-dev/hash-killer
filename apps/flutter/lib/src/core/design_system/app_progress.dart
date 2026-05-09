import 'dart:math' as math;

import 'package:flutter/widgets.dart';

import 'app_tokens.dart';

class AppProgress extends StatelessWidget {
  const AppProgress({required this.value, super.key});

  final double? value;

  @override
  Widget build(BuildContext context) {
    final current = value;

    if (current == null) {
      return const _ProgressBar(value: null);
    }

    return _ProgressBar(value: current.clamp(0.0, 1.0).toDouble());
  }
}

class AppSpinner extends StatefulWidget {
  const AppSpinner({super.key});

  @override
  State<AppSpinner> createState() => _AppSpinnerState();
}

class _AppSpinnerState extends State<AppSpinner>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 820),
    )..repeat();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 18,
      height: 18,
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, child) {
          return Transform.rotate(
            angle: _controller.value * math.pi * 2,
            child: child,
          );
        },
        child: CustomPaint(painter: _SpinnerPainter()),
      ),
    );
  }
}

class _ProgressBar extends StatefulWidget {
  const _ProgressBar({required this.value});

  final double? value;

  @override
  State<_ProgressBar> createState() => _ProgressBarState();
}

class _ProgressBarState extends State<_ProgressBar>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1100),
    );
    if (widget.value == null) {
      _controller.repeat();
    }
  }

  @override
  void didUpdateWidget(covariant _ProgressBar oldWidget) {
    super.didUpdateWidget(oldWidget);

    if (widget.value == null && !_controller.isAnimating) {
      _controller.repeat();
    }

    if (widget.value != null && _controller.isAnimating) {
      _controller.stop();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final current = widget.value;

    return ClipRRect(
      borderRadius: BorderRadius.circular(999),
      child: SizedBox(
        height: 6,
        child: DecoratedBox(
          decoration: ShapeDecoration(
            color: AppTokens.mutedStrong,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(999),
            ),
          ),
          child: current == null
              ? SizedBox.expand(
                  child: AnimatedBuilder(
                    animation: _controller,
                    builder: (context, child) {
                      return CustomPaint(
                        painter: _IndeterminateProgressPainter(
                          position: _controller.value,
                        ),
                      );
                    },
                  ),
                )
              : TweenAnimationBuilder<double>(
                  tween: Tween<double>(begin: 0, end: current),
                  duration: const Duration(milliseconds: 180),
                  curve: Curves.easeOutCubic,
                  builder: (context, value, child) {
                    return Align(
                      alignment: Alignment.centerLeft,
                      child: FractionallySizedBox(
                        widthFactor: value,
                        child: child,
                      ),
                    );
                  },
                  child: const SizedBox.expand(
                    child: DecoratedBox(
                      decoration: ShapeDecoration(
                        color: AppTokens.primary,
                        shape: StadiumBorder(),
                      ),
                    ),
                  ),
                ),
        ),
      ),
    );
  }
}

class _SpinnerPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final strokeWidth = size.shortestSide * 0.18;
    final rect = Offset.zero & size;
    final paint = Paint()
      ..color = AppTokens.primary
      ..strokeWidth = strokeWidth
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;

    canvas.drawArc(
      rect.deflate(strokeWidth / 2),
      -math.pi / 2,
      math.pi * 1.45,
      false,
      paint,
    );
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class _IndeterminateProgressPainter extends CustomPainter {
  const _IndeterminateProgressPainter({required this.position});

  final double position;

  @override
  void paint(Canvas canvas, Size size) {
    final width = size.width * 0.28;
    final start = (size.width + width) * position - width;
    final rect = Rect.fromLTWH(start, 0, width, size.height);
    final paint = Paint()..color = AppTokens.primary;

    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, Radius.circular(size.height / 2)),
      paint,
    );
  }

  @override
  bool shouldRepaint(covariant _IndeterminateProgressPainter oldDelegate) {
    return oldDelegate.position != position;
  }
}
