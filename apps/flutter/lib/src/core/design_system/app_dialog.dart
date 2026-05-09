import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

class AppDialog extends StatelessWidget {
  const AppDialog({
    required this.title,
    this.message,
    this.child,
    this.actions = const [],
    this.width = 420,
    super.key,
  });

  final String title;
  final String? message;
  final Widget? child;
  final List<Widget> actions;
  final double width;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final reducedMotion = MediaQuery.disableAnimationsOf(context);

    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: 1),
      duration: reducedMotion
          ? Duration.zero
          : const Duration(milliseconds: 180),
      curve: Curves.easeOutCubic,
      builder: (context, value, child) {
        return ColoredBox(
          color: theme.colors.foreground.withValues(alpha: 0.22 * value),
          child: Opacity(
            opacity: value,
            child: Transform.scale(scale: 0.98 + (0.02 * value), child: child),
          ),
        );
      },
      child: FDialog.raw(
        semanticsLabel: title,
        constraints: BoxConstraints(minWidth: 280, maxWidth: width),
        builder: (context, style) {
          return Padding(
            padding: const EdgeInsets.all(18),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: theme.typography.lg.copyWith(
                    color: theme.colors.foreground,
                    fontWeight: FontWeight.w500,
                  ),
                ),
                if (message != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    message!,
                    style: theme.typography.sm.copyWith(
                      color: theme.colors.mutedForeground,
                    ),
                  ),
                ],
                if (child != null) ...[const SizedBox(height: 16), child!],
                if (actions.isNotEmpty) ...[
                  const SizedBox(height: 16),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children:
                        actions
                            .expand(
                              (action) => [action, const SizedBox(width: 8)],
                            )
                            .toList()
                          ..removeLast(),
                  ),
                ],
              ],
            ),
          );
        },
      ),
    );
  }
}
