import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/stages/about_stage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('AboutStage renders hero, credits, AI disclosure, donation and license sections', (WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1080, 1920));

    await tester.pumpWidget(
      const MaterialApp(
        home: AboutStage(),
      ),
    );
    await tester.pump(const Duration(milliseconds: 200));

    // Verify Brand title & Tagline
    expect(find.text('Attune'), findsWidgets);
    expect(find.textContaining('Intelligent Audio-Profile'), findsOneWidget);

    // Verify Credits Section
    expect(find.text('CREDITS & ARCHITECTURE'), findsOneWidget);
    expect(find.text('Mizumo-prjkt'), findsOneWidget);
    expect(find.textContaining('Signalsmith, Symphonia'), findsOneWidget);

    // Verify AI Involvement Section
    expect(find.text('AI PAIR PROGRAMMING INVOLVEMENT'), findsOneWidget);
    expect(find.textContaining('Gemini 3.8 Flash'), findsWidgets);
    expect(find.textContaining('Knowledge Ledgering: Every architectural decision is recorded'), findsOneWidget);

    // Verify Donation Section
    expect(find.text('SUPPORT & DONATIONS'), findsOneWidget);
    expect(find.text('COMING SOON'), findsOneWidget);
    expect(find.textContaining('Community sponsorship avenues'), findsOneWidget);

    // Verify MIT License & Third-party disclosure
    expect(find.text('LICENSE & LEGAL NOTICES'), findsOneWidget);
    expect(find.textContaining('MIT License (Attune Player & Core)'), findsOneWidget);
    expect(find.textContaining('NOTE ON THIRD-PARTY LIBRARIES'), findsOneWidget);
    expect(find.text('View Complete Open-Source Licenses'), findsOneWidget);

    // Verify Build & Compiler Environment Section
    expect(find.text('BUILD ENVIRONMENT & COMPILER METADATA'), findsOneWidget);
    expect(find.text('Target OS'), findsOneWidget);
    expect(find.text('Architecture'), findsOneWidget);
  });

  testWidgets('showAttuneAboutDialog displays floating dialog with launcher button', (WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1080, 1920));

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showAttuneAboutDialog(context),
              child: const Text('Open Dialog'),
            ),
          ),
        ),
      ),
    );

    // Open dialog
    await tester.tap(find.text('Open Dialog'));
    await tester.pumpAndSettle();

    expect(find.text('Attune'), findsOneWidget);
    expect(find.text('Full About & Credits'), findsOneWidget);
    expect(find.text('Close'), findsOneWidget);

    // Close dialog
    await tester.tap(find.text('Close'));
    await tester.pumpAndSettle();

    expect(find.text('Full About & Credits'), findsNothing);
  });
}
