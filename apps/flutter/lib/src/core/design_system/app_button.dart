import 'package:flutter/widgets.dart';

import 'app_tokens.dart';

enum AppButtonTone {
  primary,
  secondary,
  outline,
  accentOutline,
  ghost,
  destructive,
  destructiveSolid,
  dashed,
}

enum AppButtonSize { small, medium }

enum AppButtonIconPosition { leading, trailing }

class AppButton extends StatelessWidget {
  const AppButton({
    required this.label,
    required this.onPressed,
    this.tone = AppButtonTone.primary,
    this.size = AppButtonSize.medium,
    this.icon,
    this.iconPosition = AppButtonIconPosition.leading,
    this.expanded = false,
    this.selected = false,
    super.key,
  });

  final String label;
  final VoidCallback? onPressed;
  final AppButtonTone tone;
  final AppButtonSize size;
  final IconData? icon;
  final AppButtonIconPosition iconPosition;
  final bool expanded;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final enabled = onPressed != null;
    final height = switch (size) {
      AppButtonSize.small => 30.0,
      AppButtonSize.medium => 36.0,
    };
    final horizontalPadding = switch (size) {
      AppButtonSize.small => 12.0,
      AppButtonSize.medium => 18.0,
    };
    final (background, foreground, border) = switch (tone) {
      AppButtonTone.primary => (
        AppTokens.primary,
        AppTokens.background,
        AppTokens.primary,
      ),
      AppButtonTone.secondary => (
        AppTokens.primarySoft,
        AppTokens.primary,
        AppTokens.primarySoft,
      ),
      AppButtonTone.outline => (
        AppTokens.background,
        AppTokens.foreground,
        AppTokens.border,
      ),
      AppButtonTone.accentOutline => (
        AppTokens.background,
        AppTokens.primary,
        AppTokens.border,
      ),
      AppButtonTone.ghost => (
        AppTokens.background,
        AppTokens.gray,
        AppTokens.border,
      ),
      AppButtonTone.destructive => (
        AppTokens.background,
        AppTokens.destructive,
        AppTokens.destructive,
      ),
      AppButtonTone.destructiveSolid => (
        AppTokens.destructive,
        AppTokens.background,
        AppTokens.destructive,
      ),
      AppButtonTone.dashed => (
        AppTokens.background,
        AppTokens.primary,
        AppTokens.border,
      ),
    };
    final resolvedBackground = !enabled
        ? AppTokens.background
        : selected
        ? AppTokens.primarySoft
        : background;
    final resolvedForeground = !enabled
        ? AppTokens.disabled
        : selected
        ? AppTokens.primary
        : foreground;
    final resolvedBorder = !enabled
        ? AppTokens.border
        : selected
        ? AppTokens.primarySoft
        : border;
    final content = Row(
      mainAxisSize: expanded ? MainAxisSize.max : MainAxisSize.min,
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        if (icon != null && iconPosition == AppButtonIconPosition.leading) ...[
          Icon(icon, size: 16, color: resolvedForeground),
          const SizedBox(width: 8),
        ],
        Flexible(
          child: Text(
            label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              color: resolvedForeground,
              fontSize: 14,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
        if (icon != null && iconPosition == AppButtonIconPosition.trailing) ...[
          const SizedBox(width: 8),
          Icon(icon, size: 16, color: resolvedForeground),
        ],
      ],
    );
    final button = MouseRegion(
      cursor: enabled ? SystemMouseCursors.click : SystemMouseCursors.basic,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onPressed,
        child: SizedBox(
          height: height,
          child: CustomPaint(
            painter: tone == AppButtonTone.dashed
                ? _DashedButtonPainter(
                    color: resolvedBorder,
                    radius: 8,
                    background: resolvedBackground,
                  )
                : null,
            child: DecoratedBox(
              decoration: tone == AppButtonTone.dashed
                  ? const BoxDecoration()
                  : ShapeDecoration(
                      color: resolvedBackground,
                      shape: RoundedRectangleBorder(
                        side: BorderSide(color: resolvedBorder),
                        borderRadius: BorderRadius.circular(6),
                      ),
                    ),
              child: Padding(
                padding: EdgeInsets.symmetric(horizontal: horizontalPadding),
                child: Center(child: content),
              ),
            ),
          ),
        ),
      ),
    );

    if (!expanded) {
      return button;
    }

    return SizedBox(width: double.infinity, child: button);
  }
}

class _DashedButtonPainter extends CustomPainter {
  const _DashedButtonPainter({
    required this.color,
    required this.radius,
    required this.background,
  });

  final Color color;
  final double radius;
  final Color background;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final rrect = RRect.fromRectAndRadius(rect, Radius.circular(radius));
    canvas.drawRRect(rrect, Paint()..color = background);
    final paint = Paint()
      ..color = color
      ..strokeWidth = 1
      ..style = PaintingStyle.stroke;
    const dash = 5.0;
    const gap = 5.0;
    final path = Path()..addRRect(rrect.deflate(0.5));
    for (final metric in path.computeMetrics()) {
      var distance = 0.0;
      while (distance < metric.length) {
        canvas.drawPath(metric.extractPath(distance, distance + dash), paint);
        distance += dash + gap;
      }
    }
  }

  @override
  bool shouldRepaint(covariant _DashedButtonPainter oldDelegate) {
    return oldDelegate.color != color ||
        oldDelegate.radius != radius ||
        oldDelegate.background != background;
  }
}
