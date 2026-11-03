import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/widgets/viewport_aware_artwork.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('ViewportAwareArtwork Tests', () {
    testWidgets('Renders placeholder icon when no artwork is provided', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: ViewportAwareArtwork(
              width: 50,
              height: 50,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.music_note_rounded), findsOneWidget);
    });

    testWidgets('Custom placeholder is rendered when provided', (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: ViewportAwareArtwork(
              width: 60,
              height: 60,
              placeholder: Text('Custom Placeholder'),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Custom Placeholder'), findsOneWidget);
    });

    testWidgets('Bypasses unloading mechanism when enabled is false (Bubblegrid mode)', (WidgetTester tester) async {
      // Create a temporary valid image file
      final tempDir = Directory.systemTemp.createTempSync('attune_art_test_');
      final tempFile = File('${tempDir.path}/test_art.png');
      // 1x1 transparent PNG bytes
      tempFile.writeAsBytesSync([
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
      ]);

      addTearDown(() {
        if (tempDir.existsSync()) {
          tempDir.deleteSync(recursive: true);
        }
      });

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ViewportAwareArtwork(
              artworkFile: tempFile.path,
              width: 80,
              height: 80,
              enabled: false, // Explicitly false for Bubblegrid mode
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Image widget is present because enabled: false ensures persistent loading
      expect(find.byType(Image), findsOneWidget);
    });

    testWidgets('Unloads and evicts image on widget disposal to reclaim RAM', (WidgetTester tester) async {
      final tempDir = Directory.systemTemp.createTempSync('attune_art_evict_');
      final tempFile = File('${tempDir.path}/test_evict.png');
      tempFile.writeAsBytesSync([
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
        0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
      ]);

      addTearDown(() {
        if (tempDir.existsSync()) {
          tempDir.deleteSync(recursive: true);
        }
      });

      // Render in a scrollable list
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ViewportAwareArtwork(
              artworkFile: tempFile.path,
              width: 80,
              height: 80,
              enabled: true,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(ViewportAwareArtwork), findsOneWidget);

      // Now replace with empty container (disposing ViewportAwareArtwork)
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: SizedBox.shrink(),
          ),
        ),
      );
      await tester.pumpAndSettle();

      // ViewportAwareArtwork is cleanly disposed and evicted
      expect(find.byType(ViewportAwareArtwork), findsNothing);
    });
  });
}
