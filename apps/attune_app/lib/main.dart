import 'dart:io';
import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'shell/responsive_glass_scaffold.dart';
import 'stages/about_stage.dart';
import 'stages/equalizer_stage.dart';
import 'stages/settings_stage.dart';
import 'stages/song_exclusion_stage.dart';
import 'stages/transition_inspector_stage.dart';
import 'theme/color_engine.dart';

/// Recovers the process working directory if launched from a stale/unlinked inode
void _ensureValidCurrentDirectory() {
  if (Platform.isAndroid || Platform.isIOS) return;
  try {
    Directory.current;
  } catch (_) {
    try {
      final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
      if (home != null && Directory(home).existsSync()) {
        Directory.current = home;
      } else {
        final exe = Platform.resolvedExecutable;
        final dir = File(exe).parent;
        if (dir.existsSync()) {
          Directory.current = dir.path;
        }
      }
    } catch (_) {}
  }
}


/// Standard Main Entrypoint (Player & Library Shell)
void main() {
  _ensureValidCurrentDirectory();
  runApp(const AttuneApp(initialRoute: '/'));
}

/// Standalone Entrypoint for Android SettingsActivity
@pragma('vm:entry-point')
void settingsMain() {
  _ensureValidCurrentDirectory();
  runApp(const AttuneApp(initialRoute: '/settings'));
}

/// Standalone Entrypoint for Android TransitionInspectorActivity
@pragma('vm:entry-point')
void transitionInspectorMain() {
  _ensureValidCurrentDirectory();
  runApp(const AttuneApp(initialRoute: '/transition_inspector'));
}

/// Standalone Entrypoint for AboutActivity
@pragma('vm:entry-point')
void aboutMain() {
  _ensureValidCurrentDirectory();
  runApp(const AttuneApp(initialRoute: '/about'));
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
        '/song_exclusion': (context) => const SongExclusionStage(),
        '/equalizer': (context) => const EqualizerStage(),
        '/about': (context) => const AboutStage(),
      },
    );
  }
}
