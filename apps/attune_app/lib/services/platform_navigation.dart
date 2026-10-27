import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Navigation Service facilitating independent native Android Activities
/// (SettingsActivity, TransitionInspectorActivity) for OS-level Predictive Back
/// gesture window-peeking, with automatic fallback to Flutter Navigator on Desktop/Web.
class PlatformNavigation {
  static const MethodChannel _channel = MethodChannel('com.attune.player/navigation');

  /// Opens the Settings & Audio Configuration UI.
  /// On Android, launches the independent `SettingsActivity` task window.
  /// On Desktop / Linux / Web, navigates via Flutter Navigator.
  static Future<void> openSettings(BuildContext context) async {
    if (!kIsWeb && Platform.isAndroid) {
      try {
        await _channel.invokeMethod('openSettings');
        return;
      } catch (e) {
        debugPrint('PlatformNavigation openSettings error: $e, falling back to Flutter route');
      }
    }

    if (context.mounted) {
      Navigator.of(context).pushNamed('/settings');
    }
  }

  /// Opens the Dual-Deck Transition Inspector UI.
  /// On Android, launches the independent `TransitionInspectorActivity` task window.
  /// On Desktop / Linux / Web, navigates via Flutter Navigator.
  static Future<void> openTransitionInspector(
    BuildContext context, {
    Map<String, dynamic>? arguments,
  }) async {
    if (!kIsWeb && Platform.isAndroid) {
      try {
        await _channel.invokeMethod('openTransitionInspector', arguments);
        return;
      } catch (e) {
        debugPrint('PlatformNavigation openTransitionInspector error: $e, falling back to Flutter route');
      }
    }

    if (context.mounted) {
      Navigator.of(context).pushNamed('/transition_inspector', arguments: arguments);
    }
  }

  /// Closes the current activity or pops the current route.
  static void closeCurrent(BuildContext context) {
    if (Navigator.of(context).canPop()) {
      Navigator.of(context).pop();
    } else if (!kIsWeb && Platform.isAndroid) {
      SystemNavigator.pop();
    }
  }
}
