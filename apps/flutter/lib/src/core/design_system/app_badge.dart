import 'package:flutter/widgets.dart';

import 'app_tokens.dart';

enum AppBadgeTone { primary, secondary, outline, destructive, plain }

class AppBadge extends StatelessWidget {
  const AppBadge({
    required this.label,
    this.tone = AppBadgeTone.secondary,
    this.width,
    super.key,
  });

  final String label;
  final AppBadgeTone tone;
  final double? width;

  @override
  Widget build(BuildContext context) {
    final (background, foreground, border) = switch (tone) {
      AppBadgeTone.primary => (
        AppTokens.primarySoft,
        AppTokens.primary,
        AppTokens.primarySoft,
      ),
      AppBadgeTone.secondary => (
        AppTokens.primarySoft,
        AppTokens.primary,
        AppTokens.primarySoft,
      ),
      AppBadgeTone.outline => (
        AppTokens.background,
        AppTokens.foreground,
        AppTokens.border,
      ),
      AppBadgeTone.destructive => (
        AppTokens.redSoft,
        AppTokens.destructive,
        AppTokens.redSoft,
      ),
      AppBadgeTone.plain => (
        AppTokens.background,
        AppTokens.foreground,
        AppTokens.background,
      ),
    };

    return SizedBox(
      width: width,
      height: 28,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: background,
          shape: RoundedRectangleBorder(
            side: BorderSide(color: border),
            borderRadius: BorderRadius.circular(4),
          ),
        ),
        child: Center(
          child: Text(
            label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              color: foreground,
              fontSize: 14,
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
    );
  }
}
