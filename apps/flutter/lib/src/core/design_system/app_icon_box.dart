import 'package:flutter/widgets.dart';

import 'app_tokens.dart';

enum AppIconTone { neutral, blue, purple, green, orange, yellow, red, emerald }

enum AppIconBoxSize { compact, regular, result, large }

class AppIconBox extends StatelessWidget {
  const AppIconBox({
    required this.icon,
    this.compact = false,
    this.tone = AppIconTone.neutral,
    this.size,
    super.key,
  });

  final IconData icon;
  final bool compact;
  final AppIconTone tone;
  final AppIconBoxSize? size;

  @override
  Widget build(BuildContext context) {
    final resolvedSize =
        size ?? (compact ? AppIconBoxSize.compact : AppIconBoxSize.regular);
    final dimension = switch (resolvedSize) {
      AppIconBoxSize.compact => 28.0,
      AppIconBoxSize.regular => 36.0,
      AppIconBoxSize.result => 42.0,
      AppIconBoxSize.large => 48.0,
    };
    final iconSize = switch (resolvedSize) {
      AppIconBoxSize.compact => 15.0,
      AppIconBoxSize.regular => 20.0,
      AppIconBoxSize.result => 22.0,
      AppIconBoxSize.large => 26.0,
    };
    final (background, foreground) = switch (tone) {
      AppIconTone.neutral => (AppTokens.mutedStrong, AppTokens.foreground),
      AppIconTone.blue => (AppTokens.blueSoft, AppTokens.primary),
      AppIconTone.purple => (AppTokens.purpleSoft, AppTokens.purple),
      AppIconTone.green => (AppTokens.greenSoft, AppTokens.green),
      AppIconTone.orange => (AppTokens.orangeSoft, AppTokens.orange),
      AppIconTone.yellow => (AppTokens.yellowSoft, AppTokens.yellow),
      AppIconTone.red => (AppTokens.redSoft, AppTokens.destructive),
      AppIconTone.emerald => (AppTokens.emeraldSoft, AppTokens.emerald),
    };

    return SizedBox(
      width: dimension,
      height: dimension,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: background,
          shape: RoundedRectangleBorder(
            side: const BorderSide(color: AppTokens.border),
            borderRadius: BorderRadius.circular(6),
          ),
        ),
        child: Center(
          child: Icon(icon, size: iconSize, color: foreground),
        ),
      ),
    );
  }
}
