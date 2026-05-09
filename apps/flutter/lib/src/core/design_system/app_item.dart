import 'package:flutter/widgets.dart';

import 'app_icon_box.dart';
import 'app_tokens.dart';

class AppItemRow extends StatelessWidget {
  const AppItemRow({
    required this.title,
    this.icon,
    this.subtitle,
    this.details,
    this.suffix,
    this.onPressed,
    this.selected = false,
    this.compactIcon = true,
    this.showBottomBorder = true,
    super.key,
  });

  final Widget title;
  final IconData? icon;
  final Widget? subtitle;
  final Widget? details;
  final Widget? suffix;
  final VoidCallback? onPressed;
  final bool selected;
  final bool compactIcon;
  final bool showBottomBorder;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      cursor: onPressed == null
          ? SystemMouseCursors.basic
          : SystemMouseCursors.click,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onPressed,
        child: SizedBox(
          height: 40,
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: showBottomBorder
                  ? const Border(bottom: BorderSide(color: AppTokens.border))
                  : null,
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: [
                  if (icon != null) ...[
                    AppIconBox(icon: icon!, compact: compactIcon),
                    const SizedBox(width: 14),
                  ],
                  SizedBox(width: 92, child: title),
                  Expanded(
                    child: Align(
                      alignment: Alignment.centerRight,
                      child: details ?? const SizedBox.shrink(),
                    ),
                  ),
                  if (suffix != null) ...[const SizedBox(width: 8), suffix!],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class AppSectionHeader extends StatelessWidget {
  const AppSectionHeader({
    required this.icon,
    required this.label,
    this.action,
    super.key,
  });

  final IconData icon;
  final String label;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 42,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: AppTokens.muted,
          shape: RoundedRectangleBorder(
            side: const BorderSide(color: AppTokens.border),
            borderRadius: BorderRadius.circular(8),
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: [
              AppIconBox(icon: icon, compact: true),
              const SizedBox(width: 10),
              Text(
                label,
                style: const TextStyle(
                  color: AppTokens.foreground,
                  fontSize: 14,
                  fontWeight: FontWeight.w700,
                ),
              ),
              const Spacer(),
              ?action,
            ],
          ),
        ),
      ),
    );
  }
}
