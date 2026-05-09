import 'package:flutter/widgets.dart';

import 'app_panel.dart';
import 'app_tokens.dart';

class AppEmptyState extends StatelessWidget {
  const AppEmptyState({required this.label, super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    return AppPanel(
      padding: const EdgeInsets.all(14),
      child: Center(
        child: Text(
          label,
          style: const TextStyle(
            color: AppTokens.mutedForeground,
            fontSize: 14,
          ),
        ),
      ),
    );
  }
}
