import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import '../services/audio_storage_service.dart';
import '../services/permission_service.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/directory_picker_dialog.dart';

/// StartupSetupStage: Step-by-step first-run setup procedure wizard:
/// - Step 1: Music Library & Scoped Storage directory selection
/// - Step 2: Media Playback Notifications & Android 13+ alerts permissions
/// - Step 3: Background Audio Engine Readiness & library ingestion
class StartupSetupStage extends StatefulWidget {
  final VoidCallback onSetupComplete;

  const StartupSetupStage({super.key, required this.onSetupComplete});

  @override
  State<StartupSetupStage> createState() => _StartupSetupStageState();
}

class _StartupSetupStageState extends State<StartupSetupStage> {
  int _currentStep = 0;
  final List<String> _detectedRoots = [];
  final Set<String> _selectedRoots = {};
  bool _isIngesting = false;
  bool _notificationGranted = false;
  bool _wakelockGranted = false;
  final TextEditingController _customDirController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _detectRoots();
    _checkPermissions();
  }

  void _detectRoots() {
    final candidateRoots = AudioStorageService.instance.getCandidateRoots();
    final existing = candidateRoots.where((p) => Directory(p).existsSync()).toSet().toList();

    setState(() {
      _detectedRoots.clear();
      _detectedRoots.addAll(existing);
      _selectedRoots.clear();
      _selectedRoots.addAll(existing);
    });
  }

  Future<void> _checkPermissions() async {
    if (!kIsWeb && Platform.isAndroid) {
      final notif = await PlatformPermissionService.instance.isNotificationGranted();
      final wake = await PlatformPermissionService.instance.isWakelockGranted();
      if (mounted) {
        setState(() {
          _notificationGranted = notif;
          _wakelockGranted = wake;
        });
      }
    } else {
      if (mounted) {
        setState(() {
          _notificationGranted = true;
          _wakelockGranted = true;
        });
      }
    }
  }

  @override
  void dispose() {
    _customDirController.dispose();
    super.dispose();
  }

  Future<void> _completeSetup() async {
    setState(() => _isIngesting = true);

    try {
      if (!kIsWeb && Platform.isAndroid) {
        await PlatformPermissionService.instance.requestStorage();
      }

      // 1. Mark setup as completed in config
      AudioStorageService.instance.setSetupCompleted(true, selectedRoots: _selectedRoots.toList());

      // 2. Trigger initial library scan
      await PlaybackController.instance.scanLocalAudioDirectories(forceRefresh: true);
    } finally {
      if (mounted) {
        setState(() => _isIngesting = false);
      }
      widget.onSetupComplete();
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Center(
        child: SingleChildScrollView(
          physics: const BouncingScrollPhysics(),
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 32),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 580),
            child: LiquidGlassContainer(
              padding: const EdgeInsets.all(28),
              borderRadius: 32,
              surfaceAlpha: 0.22,
              blurSigma: 40,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.center,
                children: [
                  // App Icon (clean elevation without heavy glow)
                  Container(
                    width: 68,
                    height: 68,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      gradient: LinearGradient(
                        colors: [colorScheme.primary, colorScheme.tertiary],
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                      ),
                    ),
                    child: Padding(
                      padding: const EdgeInsets.all(12.0),
                      child: Image.asset(
                        'assets/icon/icon_transparent.png',
                        fit: BoxFit.contain,
                      ),
                    ),
                  ),

                  const SizedBox(height: 16),

                  // Title
                  Text(
                    'Welcome to Attune',
                    style: AttuneTypography.soraHeading(
                      fontSize: 24,
                      fontWeight: FontWeight.bold,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Intelligent DJ-Grade Music Player & Real-Time Transition Engine',
                    textAlign: TextAlign.center,
                    style: AttuneTypography.jakartaBody(
                      color: colorScheme.onSurface.withValues(alpha: 0.65),
                      fontSize: 12,
                    ),
                  ),

                  const SizedBox(height: 24),

                  // Step Indicator Header
                  _buildStepIndicator(colorScheme),

                  const SizedBox(height: 24),

                  // Step Body
                  if (_currentStep == 0)
                    _buildStepStorage(colorScheme)
                  else if (_currentStep == 1)
                    _buildStepNotifications(colorScheme)
                  else
                    _buildStepEngine(colorScheme),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildStepIndicator(ColorScheme colorScheme) {
    const steps = ['Storage', 'Notifications', 'Engine'];
    return Row(
      children: List.generate(steps.length * 2 - 1, (index) {
        if (index.isOdd) {
          final stepBefore = index ~/ 2;
          final isPast = _currentStep > stepBefore;
          return Expanded(
            child: Container(
              height: 2,
              color: isPast ? colorScheme.primary : Colors.white12,
            ),
          );
        }

        final stepIndex = index ~/ 2;
        final isActive = _currentStep == stepIndex;
        final isCompleted = _currentStep > stepIndex;

        return Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 28,
              height: 28,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: isActive
                    ? colorScheme.primary
                    : isCompleted
                        ? colorScheme.primary.withValues(alpha: 0.35)
                        : Colors.white.withValues(alpha: 0.08),
                border: Border.all(
                  color: isActive ? colorScheme.primary : Colors.white24,
                  width: 1.5,
                ),
              ),
              child: Center(
                child: isCompleted
                    ? const Icon(Icons.check_rounded, size: 16, color: Colors.white)
                    : Text(
                        '${stepIndex + 1}',
                        style: AttuneTypography.soraSubheading(
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                          color: isActive ? colorScheme.onPrimary : Colors.white70,
                        ),
                      ),
              ),
            ),
            const SizedBox(width: 6),
            Text(
              steps[stepIndex],
              style: AttuneTypography.jakartaBodySmall(
                color: isActive ? colorScheme.onSurface : Colors.white38,
                fontWeight: isActive ? FontWeight.w600 : FontWeight.normal,
              ),
            ),
          ],
        );
      }),
    );
  }

  // --- STEP 1: STORAGE & DIRECTORIES ---
  Widget _buildStepStorage(ColorScheme colorScheme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Align(
          alignment: Alignment.centerLeft,
          child: Text(
            'STEP 1 OF 3 • CONFIGURE MUSIC DIRECTORIES',
            style: AttuneTypography.soraBadge(color: colorScheme.primary),
          ),
        ),
        const SizedBox(height: 6),
        Text(
          'Select the folders containing your audio files. On Android, you can browse scoped storage or select specific music collections.',
          style: AttuneTypography.jakartaBody(
            color: colorScheme.onSurface.withValues(alpha: 0.7),
            fontSize: 12,
          ),
        ),
        const SizedBox(height: 14),

        if (_detectedRoots.isEmpty)
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: Colors.white.withValues(alpha: 0.05),
              borderRadius: BorderRadius.circular(16),
            ),
            child: Text(
              'No default music folders detected. Add a folder below.',
              style: AttuneTypography.jakartaBody(color: colorScheme.onSurface.withValues(alpha: 0.6)),
            ),
          )
        else
          ..._detectedRoots.map((root) {
            final isSelected = _selectedRoots.contains(root);
            return Padding(
              padding: const EdgeInsets.only(bottom: 8.0),
              child: Material(
                color: Colors.transparent,
                child: CheckboxListTile(
                  value: isSelected,
                  onChanged: (val) {
                    setState(() {
                      if (val == true) {
                        _selectedRoots.add(root);
                      } else {
                        _selectedRoots.remove(root);
                      }
                    });
                  },
                  dense: true,
                  contentPadding: const EdgeInsets.symmetric(horizontal: 10),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
                  tileColor: isSelected ? colorScheme.primary.withValues(alpha: 0.12) : Colors.white.withValues(alpha: 0.04),
                  activeColor: colorScheme.primary,
                  title: Text(
                    root,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: AttuneTypography.soraSubheading(
                      fontSize: 13,
                      fontWeight: isSelected ? FontWeight.w600 : FontWeight.normal,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  secondary: Icon(Icons.folder_rounded, color: isSelected ? colorScheme.primary : Colors.white54, size: 20),
                ),
              ),
            );
          }),

        const SizedBox(height: 12),

        // Browse Scoped Storage / Device Directories
        SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            style: OutlinedButton.styleFrom(
              padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 16),
              side: BorderSide(color: colorScheme.primary.withValues(alpha: 0.5)),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
              backgroundColor: colorScheme.primary.withValues(alpha: 0.08),
            ),
            icon: Icon(Icons.folder_open_rounded, color: colorScheme.primary, size: 20),
            label: Text(
              'Browse Folders / Scoped Storage',
              style: AttuneTypography.soraSubheading(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: colorScheme.primary,
              ),
            ),
            onPressed: () async {
              final selected = await DirectoryPickerDialog.show(context);
              if (selected != null && selected.isNotEmpty) {
                setState(() {
                  if (!_detectedRoots.contains(selected)) {
                    _detectedRoots.add(selected);
                  }
                  _selectedRoots.add(selected);
                });
              }
            },
          ),
        ),

        const SizedBox(height: 10),

        // Custom Directory Input
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _customDirController,
                style: AttuneTypography.jakartaBody(color: colorScheme.onSurface),
                decoration: InputDecoration(
                  hintText: 'Or enter custom path...',
                  hintStyle: AttuneTypography.jakartaBody(color: Colors.white38),
                  isDense: true,
                  filled: true,
                  fillColor: Colors.white.withValues(alpha: 0.05),
                  contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(14),
                    borderSide: BorderSide.none,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 8),
            IconButton.filled(
              style: IconButton.styleFrom(
                backgroundColor: colorScheme.primary.withValues(alpha: 0.3),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
              ),
              onPressed: () {
                final path = _customDirController.text.trim();
                if (path.isNotEmpty && Directory(path).existsSync()) {
                  setState(() {
                    if (!_detectedRoots.contains(path)) {
                      _detectedRoots.add(path);
                    }
                    _selectedRoots.add(path);
                    _customDirController.clear();
                  });
                }
              },
              icon: const Icon(Icons.add_rounded),
            ),
          ],
        ),

        const SizedBox(height: 24),

        // Next Step Button
        SizedBox(
          width: double.infinity,
          height: 48,
          child: ElevatedButton.icon(
            style: ElevatedButton.styleFrom(
              backgroundColor: colorScheme.primary,
              foregroundColor: colorScheme.onPrimary,
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
            ),
            onPressed: () => setState(() => _currentStep = 1),
            icon: const Icon(Icons.arrow_forward_rounded, size: 20),
            label: Text(
              'Continue to Notifications',
              style: AttuneTypography.soraSubheading(
                fontSize: 14,
                fontWeight: FontWeight.w700,
                color: colorScheme.onPrimary,
              ),
            ),
          ),
        ),
      ],
    );
  }

  // --- STEP 2: NOTIFICATIONS & ALERTS ---
  Widget _buildStepNotifications(ColorScheme colorScheme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Align(
          alignment: Alignment.centerLeft,
          child: Text(
            'STEP 2 OF 3 • MEDIA NOTIFICATIONS & ALERTS',
            style: AttuneTypography.soraBadge(color: colorScheme.primary),
          ),
        ),
        const SizedBox(height: 6),
        Text(
          'Detailed explanation of why Attune utilizes notifications and system media controls:',
          style: AttuneTypography.jakartaBody(
            color: colorScheme.onSurface.withValues(alpha: 0.7),
            fontSize: 12,
          ),
        ),
        const SizedBox(height: 14),

        _buildNotificationCard(
          icon: Icons.music_note_rounded,
          title: 'Lock Screen & Quick Settings Media Player',
          description:
              'Renders live track information, high-resolution album artwork, playback scrub controls, and play/pause/skip buttons directly inside your notification shade and lock screen.',
          colorScheme: colorScheme,
        ),
        const SizedBox(height: 8),

        _buildNotificationCard(
          icon: Icons.auto_awesome_rounded,
          title: 'DJ Transition Alerts & Mix Verdicts',
          description:
              'Provides real-time notifications when the engine executes dynamic 1-bar riff handoffs, harmonic key aligns, or vocal sustain morphs between queued tracks.',
          colorScheme: colorScheme,
        ),
        const SizedBox(height: 8),

        _buildNotificationCard(
          icon: Icons.shield_rounded,
          title: 'Android 13+ Background Stability (POST_NOTIFICATIONS)',
          description:
              'On Android 13 (API 33) and above, notification permission is mandatory for foreground playback services. Without this, the Android OS may kill audio playback when the screen is locked.',
          colorScheme: colorScheme,
        ),

        const SizedBox(height: 16),

        // Permission Action / Status
        Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          decoration: BoxDecoration(
            color: _notificationGranted
                ? const Color(0xFF10B981).withValues(alpha: 0.12)
                : colorScheme.primary.withValues(alpha: 0.12),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(
              color: _notificationGranted ? const Color(0xFF10B981).withValues(alpha: 0.4) : colorScheme.primary.withValues(alpha: 0.3),
            ),
          ),
          child: Row(
            children: [
              Icon(
                _notificationGranted ? Icons.check_circle_rounded : Icons.notifications_active_rounded,
                color: _notificationGranted ? const Color(0xFF10B981) : colorScheme.primary,
                size: 24,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      _notificationGranted ? 'Notifications Allowed' : 'Permission Required for Android 13+',
                      style: AttuneTypography.soraSubheading(
                        fontSize: 13,
                        fontWeight: FontWeight.w600,
                        color: colorScheme.onSurface,
                      ),
                    ),
                    Text(
                      _notificationGranted
                          ? 'Media player notification & transition alerts are fully enabled.'
                          : 'Tap to grant POST_NOTIFICATIONS for uninterrupted background playback.',
                      style: AttuneTypography.jakartaBodySmall(
                        color: colorScheme.onSurface.withValues(alpha: 0.65),
                      ),
                    ),
                  ],
                ),
              ),
              if (!_notificationGranted && !kIsWeb && Platform.isAndroid)
                FilledButton.tonal(
                  style: FilledButton.styleFrom(
                    backgroundColor: colorScheme.primary,
                    foregroundColor: colorScheme.onPrimary,
                  ),
                  onPressed: () async {
                    final granted = await PlatformPermissionService.instance.requestNotification();
                    if (mounted) {
                      setState(() => _notificationGranted = granted);
                    }
                  },
                  child: const Text('Allow'),
                ),
            ],
          ),
        ),

        const SizedBox(height: 24),

        // Navigation Row
        Row(
          children: [
            Expanded(
              child: OutlinedButton(
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                  side: const BorderSide(color: Colors.white24),
                ),
                onPressed: () => setState(() => _currentStep = 0),
                child: Text('Back', style: AttuneTypography.soraSubheading(fontSize: 14, color: Colors.white70)),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              flex: 2,
              child: ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: colorScheme.primary,
                  foregroundColor: colorScheme.onPrimary,
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                ),
                onPressed: () => setState(() => _currentStep = 2),
                icon: const Icon(Icons.arrow_forward_rounded, size: 20),
                label: Text(
                  'Continue to Engine',
                  style: AttuneTypography.soraSubheading(
                    fontSize: 14,
                    fontWeight: FontWeight.w700,
                    color: colorScheme.onPrimary,
                  ),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }

  // --- STEP 3: ENGINE READINESS & ENTER ---
  Widget _buildStepEngine(ColorScheme colorScheme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Align(
          alignment: Alignment.centerLeft,
          child: Text(
            'STEP 3 OF 3 • ENGINE READINESS & FINISH',
            style: AttuneTypography.soraBadge(color: colorScheme.primary),
          ),
        ),
        const SizedBox(height: 6),
        Text(
          'Attune native audio engine is ready to profile your tracks and execute seamless automixes.',
          style: AttuneTypography.jakartaBody(
            color: colorScheme.onSurface.withValues(alpha: 0.7),
            fontSize: 12,
          ),
        ),
        const SizedBox(height: 14),

        _buildEngineFeature(
          icon: Icons.graphic_eq_rounded,
          title: 'Fractional Hermite Resampling',
          description: 'Accurate 1.000x speed & pitch preservation for 44.1k/48k DACs',
          colorScheme: colorScheme,
        ),
        const SizedBox(height: 8),
        _buildEngineFeature(
          icon: Icons.waves_rounded,
          title: '1-Bar (4-Beat) Phrased Riff Handoff',
          description: 'Multi-band isolator EQ carve + vocal integrity protection',
          colorScheme: colorScheme,
        ),
        const SizedBox(height: 8),
        _buildEngineFeature(
          icon: Icons.storage_rounded,
          title: 'Local SQLite Cache Indexing',
          description: 'Instant startup in < 5ms with zero blocking UI freezes',
          colorScheme: colorScheme,
        ),

        const SizedBox(height: 14),

        // Background Wakelock card for Android
        if (!kIsWeb && Platform.isAndroid)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            decoration: BoxDecoration(
              color: Colors.white.withValues(alpha: 0.04),
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: Colors.white12),
            ),
            child: Row(
              children: [
                Icon(
                  _wakelockGranted ? Icons.battery_charging_full_rounded : Icons.battery_saver_rounded,
                  color: _wakelockGranted ? const Color(0xFF10B981) : Colors.amber,
                  size: 20,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Background Audio Optimization',
                        style: AttuneTypography.soraSubheading(fontSize: 12, fontWeight: FontWeight.w600, color: colorScheme.onSurface),
                      ),
                      Text(
                        _wakelockGranted
                          ? 'Battery optimization exemption active for stutter-free playback.'
                          : 'Recommended to prevent Android from sleeping the audio engine.',
                        style: AttuneTypography.jakartaBodySmall(fontSize: 11, color: colorScheme.onSurface.withValues(alpha: 0.6)),
                      ),
                    ],
                  ),
                ),
                if (!_wakelockGranted)
                  TextButton(
                    onPressed: () async {
                      final ok = await PlatformPermissionService.instance.requestWakelock();
                      if (mounted) setState(() => _wakelockGranted = ok);
                    },
                    child: const Text('Optimize'),
                  ),
              ],
            ),
          ),

        const SizedBox(height: 24),

        // Navigation Row
        Row(
          children: [
            Expanded(
              child: OutlinedButton(
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                  side: const BorderSide(color: Colors.white24),
                ),
                onPressed: _isIngesting ? null : () => setState(() => _currentStep = 1),
                child: Text('Back', style: AttuneTypography.soraSubheading(fontSize: 14, color: Colors.white70)),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              flex: 2,
              child: ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: colorScheme.primary,
                  foregroundColor: colorScheme.onPrimary,
                  elevation: 4,
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                ),
                onPressed: _isIngesting ? null : _completeSetup,
                icon: _isIngesting
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                      )
                    : const Icon(Icons.play_arrow_rounded, size: 22),
                label: Text(
                  _isIngesting ? 'Ingesting Library...' : 'Finish & Enter Attune',
                  style: AttuneTypography.soraSubheading(
                    fontSize: 14,
                    fontWeight: FontWeight.w700,
                    color: colorScheme.onPrimary,
                  ),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildNotificationCard({
    required IconData icon,
    required String title,
    required String description,
    required ColorScheme colorScheme,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.04),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2.0),
            child: Icon(icon, color: colorScheme.primary, size: 18),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: AttuneTypography.soraSubheading(fontSize: 12, fontWeight: FontWeight.w600, color: colorScheme.onSurface),
                ),
                const SizedBox(height: 2),
                Text(
                  description,
                  style: AttuneTypography.jakartaBody(fontSize: 11, color: colorScheme.onSurface.withValues(alpha: 0.65)),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildEngineFeature({
    required IconData icon,
    required String title,
    required String description,
    required ColorScheme colorScheme,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.04),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        children: [
          Icon(icon, color: colorScheme.primary, size: 18),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: AttuneTypography.soraSubheading(fontSize: 12, fontWeight: FontWeight.w600, color: colorScheme.onSurface),
                ),
                Text(
                  description,
                  style: AttuneTypography.jakartaBody(fontSize: 11, color: colorScheme.onSurface.withValues(alpha: 0.5)),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
