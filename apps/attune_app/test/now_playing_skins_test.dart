import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/settings_service.dart';
import 'package:attune_app/stages/now_playing_skin_picker_stage.dart';
import 'package:attune_app/stages/now_playing_stage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late Directory tempDir;

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('attune_skins_test_');
  });

  tearDown(() {
    try {
      if (tempDir.existsSync()) {
        tempDir.deleteSync(recursive: true);
      }
    } catch (_) {}
  });

  group('NowPlayingSkin & Settings Tests', () {
    test('NowPlayingSkin enum defines valid display names and descriptions', () {
      expect(NowPlayingSkin.defaultSkin.displayName, contains('Liquid Glass'));
      expect(NowPlayingSkin.materialYouFreeshapes.displayName, contains('Material You Freeshapes'));
      expect(NowPlayingSkin.normalSquareBig.displayName, contains('Normal Square Big'));

      for (final skin in NowPlayingSkin.values) {
        expect(skin.description.isNotEmpty, isTrue);
      }
    });

    test('SettingsService updates and retains NowPlayingSkin', () {
      final settings = SettingsService.instance;
      settings.setNowPlayingSkin(NowPlayingSkin.materialYouFreeshapes);
      expect(settings.nowPlayingSkin, equals(NowPlayingSkin.materialYouFreeshapes));

      settings.setNowPlayingSkin(NowPlayingSkin.normalSquareBig);
      expect(settings.nowPlayingSkin, equals(NowPlayingSkin.normalSquareBig));

      settings.setNowPlayingSkin(NowPlayingSkin.defaultSkin);
      expect(settings.nowPlayingSkin, equals(NowPlayingSkin.defaultSkin));
    });
  });

  group('NowPlayingSkinPickerStage Widget Tests', () {
    testWidgets('Renders all skin cards with mockups and glass mode options', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: NowPlayingSkinPickerStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Now Playing Visual Skins'), findsOneWidget);
      expect(find.text('Liquid Glass (Default)'), findsOneWidget);
      expect(find.text('Material You Freeshapes'), findsOneWidget);
      expect(find.text('Normal Square Big'), findsOneWidget);
      expect(find.text('GLASS MATERIAL PRESENTATION'), findsOneWidget);
      expect(find.text('3D Specular Mirror'), findsOneWidget);
      expect(find.text('2D Minimal Flat'), findsOneWidget);
    });

    testWidgets('Tapping a skin card activates it immediately in SettingsService', (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1080, 2400);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      final settings = SettingsService.instance;
      settings.setNowPlayingSkin(NowPlayingSkin.defaultSkin);

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: NowPlayingSkinPickerStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Tap on Material You Freeshapes
      await tester.tap(find.text('Material You Freeshapes'));
      await tester.pumpAndSettle();

      expect(settings.nowPlayingSkin, equals(NowPlayingSkin.materialYouFreeshapes));

      // Tap on Normal Square Big
      await tester.tap(find.text('Normal Square Big'));
      await tester.pumpAndSettle();

      expect(settings.nowPlayingSkin, equals(NowPlayingSkin.normalSquareBig));
    });
  });

  group('NowPlayingStage Adaptive Skin Rendering Tests', () {
    testWidgets('NowPlayingStage adapts to Material You Freeshapes and NormalSquareBig skins', (WidgetTester tester) async {
      final settings = SettingsService.instance;
      settings.setNowPlayingSkin(NowPlayingSkin.materialYouFreeshapes);

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: NowPlayingStage(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Top palette button should be present
      expect(find.byIcon(Icons.palette_rounded), findsOneWidget);

      // Freeshape clipper should be active
      expect(find.byType(ClipPath), findsWidgets);

      // Switch to NormalSquareBig
      settings.setNowPlayingSkin(NowPlayingSkin.normalSquareBig);
      await tester.pumpAndSettle();

      // Side subtitles/lyrics button should be present
      expect(find.byIcon(Icons.subtitles_rounded), findsWidgets);

      // Revert to defaultSkin
      settings.setNowPlayingSkin(NowPlayingSkin.defaultSkin);
      await tester.pumpAndSettle();
    });
  });
}
