import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'shell/responsive_glass_scaffold.dart';
import 'stages/settings_stage.dart';
import 'stages/transition_inspector_stage.dart';
import 'theme/color_engine.dart';

/// Standard Main Entrypoint (Player & Library Shell)
void main() {
  runApp(const AttuneApp(initialRoute: '/'));
}

/// Standalone Entrypoint for Android SettingsActivity
@pragma('vm:entry-point')
void settingsMain() {
  runApp(const AttuneApp(initialRoute: '/settings'));
}

/// Standalone Entrypoint for Android TransitionInspectorActivity
@pragma('vm:entry-point')
void transitionInspectorMain() {
  runApp(const AttuneApp(initialRoute: '/transition_inspector'));
}

class AttuneApp extends StatelessWidget {
  final String initialRoute;

  const AttuneApp({super.key, this.initialRoute = '/'});

  @override
  Widget build(BuildContext context) {
    final colorScheme = AttuneColorEngine.defaultDarkScheme;

    return MaterialApp(
      title: 'Attune',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.dark,
      theme: ThemeData(
        useMaterial3: true,
        brightness: Brightness.dark,
        colorScheme: colorScheme,
        scaffoldBackgroundColor: colorScheme.surface,
        pageTransitionsTheme: const PageTransitionsTheme(
          builders: {
            TargetPlatform.android: PredictiveBackPageTransitionsBuilder(),
            TargetPlatform.iOS: CupertinoPageTransitionsBuilder(),
            TargetPlatform.linux: ZoomPageTransitionsBuilder(),
            TargetPlatform.windows: ZoomPageTransitionsBuilder(),
            TargetPlatform.macOS: CupertinoPageTransitionsBuilder(),
          },
        ),
      ),
      initialRoute: initialRoute,
      routes: {
        '/': (context) => const ResponsiveGlassScaffold(),
        '/settings': (context) => const SettingsAndAudioStage(isStandaloneActivity: true),
        '/transition_inspector': (context) => const TransitionInspectorStage(isStandaloneActivity: true),
      },
    );
  }
}
