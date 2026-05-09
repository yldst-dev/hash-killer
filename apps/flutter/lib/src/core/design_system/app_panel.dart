import 'package:flutter/widgets.dart';

import 'app_tokens.dart';

class AppPanel extends StatelessWidget {
  const AppPanel({
    required this.child,
    this.title,
    this.subtitle,
    this.padding = const EdgeInsets.all(12),
    this.width,
    this.backgroundColor = AppTokens.background,
    super.key,
  });

  final String? title;
  final String? subtitle;
  final Widget child;
  final EdgeInsetsGeometry padding;
  final double? width;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    final content = title == null && subtitle == null
        ? child
        : Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (title != null) Text(title!),
              if (subtitle != null) Text(subtitle!),
              child,
            ],
          );

    return SizedBox(
      width: width ?? double.infinity,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: backgroundColor,
          shape: RoundedRectangleBorder(
            side: const BorderSide(color: AppTokens.border),
            borderRadius: BorderRadius.circular(8),
          ),
        ),
        child: Padding(padding: padding, child: content),
      ),
    );
  }
}
