import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Platform Permission Service checking and requesting real OS permissions:
/// - Android: Queries PowerManager (battery optimization / wakelock) and NotificationManager (POST_NOTIFICATIONS).
/// - Desktop (Linux/Windows/macOS): Identifies desktop targets where wakelock and notifications are disabled/managed by desktop OS.
class PlatformPermissionService {
  static final PlatformPermissionService instance = PlatformPermissionService._();
  PlatformPermissionService._();

  static const MethodChannel _channel = MethodChannel('org.mizproject.attune/permissions');

  bool get isDesktop => !kIsWeb && (Platform.isLinux || Platform.isWindows || Platform.isMacOS);
  bool get isAndroid => !kIsWeb && Platform.isAndroid;

  Future<bool> isWakelockGranted() async {
    if (isDesktop) return false;
    if (!isAndroid) return false;
    try {
      final res = await _channel.invokeMethod<bool>('isWakelockPermissionGranted');
      return res ?? false;
    } catch (e) {
      debugPrint('[PermissionService] isWakelockGranted error: $e');
      return false;
    }
  }

  Future<bool> requestWakelock() async {
    if (isDesktop) return false;
    if (!isAndroid) return false;
    try {
      final res = await _channel.invokeMethod<bool>('requestWakelockPermission');
      return res ?? false;
    } catch (e) {
      debugPrint('[PermissionService] requestWakelock error: $e');
      return false;
    }
  }

  Future<bool> isNotificationGranted() async {
    if (isDesktop) return false;
    if (!isAndroid) return false;
    try {
      final res = await _channel.invokeMethod<bool>('isNotificationPermissionGranted');
      return res ?? false;
    } catch (e) {
      debugPrint('[PermissionService] isNotificationGranted error: $e');
      return false;
    }
  }

  Future<bool> requestNotification() async {
    if (isDesktop) return false;
    if (!isAndroid) return false;
    try {
      final res = await _channel.invokeMethod<bool>('requestNotificationPermission');
      return res ?? false;
    } catch (e) {
      debugPrint('[PermissionService] requestNotification error: $e');
      return false;
    }
  }
}
