import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:attune_app/stages/startup_setup_stage.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('StartupSetupStage Step-by-Step Wizard Tests', () {
    testWidgets('Renders 3-step wizard and navigates between Storage, Notifications, and Engine',
        (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1200, 2000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      bool completed = false;

      await tester.pumpWidget(
        MaterialApp(
          home: StartupSetupStage(
            onSetupComplete: () => completed = true,
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Step indicator verification
      expect(find.text('Storage'), findsOneWidget);
      expect(find.text('Notifications'), findsOneWidget);
      expect(find.text('Engine'), findsOneWidget);

      // Step 0: Storage verification
      expect(find.text('STEP 1 OF 3 • CONFIGURE MUSIC DIRECTORIES'), findsOneWidget);
      expect(find.text('Browse Folders / Scoped Storage'), findsOneWidget);
      expect(find.text('Continue to Notifications'), findsOneWidget);

      // Navigate to Step 1: Notifications & Alerts
      final continueBtn1 = find.text('Continue to Notifications');
      await tester.ensureVisible(continueBtn1);
      await tester.tap(continueBtn1);
      await tester.pumpAndSettle();

      // Step 1: Notifications verification
      expect(find.text('STEP 2 OF 3 • MEDIA NOTIFICATIONS & ALERTS'), findsOneWidget);
      expect(find.text('Lock Screen & Quick Settings Media Player'), findsOneWidget);
      expect(find.text('DJ Transition Alerts & Mix Verdicts'), findsOneWidget);
      expect(find.text('Android 13+ Background Stability (POST_NOTIFICATIONS)'), findsOneWidget);
      expect(find.text('Continue to Engine'), findsOneWidget);

      // Navigate back to Step 0
      final backBtn = find.text('Back');
      await tester.ensureVisible(backBtn);
      await tester.tap(backBtn);
      await tester.pumpAndSettle();
      expect(find.text('STEP 1 OF 3 • CONFIGURE MUSIC DIRECTORIES'), findsOneWidget);

      // Navigate to Step 1 and then Step 2
      final continueBtn1Again = find.text('Continue to Notifications');
      await tester.ensureVisible(continueBtn1Again);
      await tester.tap(continueBtn1Again);
      await tester.pumpAndSettle();

      final continueBtn2 = find.text('Continue to Engine');
      await tester.ensureVisible(continueBtn2);
      await tester.tap(continueBtn2);
      await tester.pumpAndSettle();

      // Step 2: Engine Readiness verification
      expect(find.text('STEP 3 OF 3 • ENGINE READINESS & FINISH'), findsOneWidget);
      expect(find.text('Fractional Hermite Resampling'), findsOneWidget);
      expect(find.text('1-Bar (4-Beat) Phrased Riff Handoff'), findsOneWidget);
      expect(find.text('Local SQLite Cache Indexing'), findsOneWidget);
      expect(find.text('Finish & Enter Attune'), findsOneWidget);

      // Finish setup
      final finishBtn = find.text('Finish & Enter Attune');
      await tester.ensureVisible(finishBtn);
      await tester.tap(finishBtn);
      await tester.pumpAndSettle();
      expect(completed, isTrue);
    });
  });
}
