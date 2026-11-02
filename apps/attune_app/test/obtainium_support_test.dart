import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/services/update_service.dart';
import 'package:attune_app/stages/about_stage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('Obtainium Support & Deep Link Verification', () {
    test('ObtainiumConfig builds correct deep link and redirect URL matching specification', () {
      final deepLink = ObtainiumConfig.buildDeepLink();
      expect(deepLink.startsWith('obtainium://app/'), isTrue);

      final redirectUrl = ObtainiumConfig.buildRedirectUrl();
      expect(redirectUrl, equals(ObtainiumConfig.redirectUrl));
      expect(deepLink, equals(ObtainiumConfig.deepLinkUrl));

      // Validate URL parsing and query extraction
      final uri = Uri.parse(redirectUrl);
      expect(uri.scheme, equals('https'));
      expect(uri.host, equals('apps.obtainium.imranr.dev'));
      expect(uri.path, equals('/redirect'));

      final rParam = uri.queryParameters['r'];
      expect(rParam, isNotNull);
      expect(rParam!.startsWith('obtainium://app/'), isTrue);

      // Extract and decode JSON payload
      final encodedJson = rParam.substring('obtainium://app/'.length);
      final decodedJsonStr = Uri.decodeComponent(encodedJson);
      final payload = json.decode(decodedJsonStr) as Map<String, dynamic>;

      expect(payload['id'], equals('org.mizproject.attune'));
      expect(payload['url'], equals('https://github.com/Mizumo-prjkt/attune'));
      expect(payload['author'], equals('Mizumo-prjkt'));
      expect(payload['name'], equals('Attune'));

      // Validate additionalSettings JSON string
      final additionalSettings = json.decode(payload['additionalSettings'] as String) as Map<String, dynamic>;
      expect(
        additionalSettings['about'],
        equals('Open-source DJ-grade audio-profile-driven transition and remixing music player.'),
      );
    });

    test('Obtainium badge asset exists on disk', () {
      final badgeFile = File('assets/images/badge_obtainium.png');
      expect(badgeFile.existsSync(), isTrue, reason: 'badge_obtainium.png must exist in assets/images/');
      expect(badgeFile.lengthSync(), greaterThan(1000));
    });

    testWidgets('AboutStage displays Obtainium card, badge, and interaction triggers', (tester) async {
      await tester.binding.setSurfaceSize(const Size(800, 1200));

      await tester.pumpWidget(
        MaterialApp(
          theme: ThemeData.dark(),
          home: const AboutStage(),
        ),
      );
      await tester.pumpAndSettle();

      // Verify Obtainium card header and badge
      expect(find.text('Obtainium (Android App Updates)'), findsOneWidget);
      expect(find.text('Get it on Obtainium'), findsOneWidget);
      expect(find.text('Add to Obtainium'), findsOneWidget);
      expect(find.text('Copy Link'), findsOneWidget);

      // Verify badge image is rendered
      final imageFinder = find.byWidgetPredicate(
        (widget) => widget is Image && widget.image is AssetImage && (widget.image as AssetImage).assetName == ObtainiumConfig.badgeAssetPath,
      );
      expect(imageFinder, findsOneWidget);

      // Tap Copy Link button and check feedback
      await tester.tap(find.text('Copy Link'));
      await tester.pumpAndSettle();

      expect(find.text('Obtainium redirect URL copied to clipboard'), findsOneWidget);
    });
  });
}
