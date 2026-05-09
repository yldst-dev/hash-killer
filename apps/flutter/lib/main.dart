import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/app/hash_killer_app.dart';

void main() {
  FlutterError.onError = (details) {
    final exception = details.exceptionAsString();

    if (exception.contains('A KeyDownEvent is dispatched') &&
        exception.contains('physical key is already pressed')) {
      return;
    }

    FlutterError.presentError(details);
  };

  runApp(const ProviderScope(child: HashKillerApp()));
}
