import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

class AppScaffold extends StatelessWidget {
  const AppScaffold({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return FScaffold(childPad: false, child: child);
  }
}
