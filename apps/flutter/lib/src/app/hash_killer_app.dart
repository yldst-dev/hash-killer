import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

import '../features/scan/presentation/scan_page.dart';

class HashKillerApp extends StatelessWidget {
  const HashKillerApp({super.key});

  @override
  Widget build(BuildContext context) {
    final platform = switch (defaultTargetPlatform) {
      TargetPlatform.iOS || TargetPlatform.android => FPlatformVariant.iOS,
      _ => FPlatformVariant.macOS,
    };
    final theme = platform.desktop
        ? FThemes.zinc.light.desktop
        : FThemes.zinc.light.touch;

    return WidgetsApp(
      color: theme.colors.background,
      debugShowCheckedModeBanner: false,
      locale: const Locale('ko', 'KR'),
      localizationsDelegates: FLocalizations.localizationsDelegates,
      supportedLocales: FLocalizations.supportedLocales,
      pageRouteBuilder: <T>(settings, builder) => PageRouteBuilder<T>(
        settings: settings,
        pageBuilder: (context, animation, secondaryAnimation) =>
            builder(context),
      ),
      builder: (context, child) => FTheme(
        data: theme,
        platform: platform,
        child: FToaster(child: FTooltipGroup(child: child!)),
      ),
      home: const ScanPage(),
    );
  }
}
