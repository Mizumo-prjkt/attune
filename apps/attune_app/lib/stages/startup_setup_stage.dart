import 'dart:io';
import 'package:flutter/material.dart';
import '../services/audio_storage_service.dart';
import '../services/permission_service.dart';
import '../services/playback_controller.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';
import '../widgets/directory_picker_dialog.dart';

/// StartupSetupStage: First-run setup and directory configuration welcome screen.
/// Allows the user to verify music directories, review playback engine options,
/// and transition smoothly to the primary Home stage.
class StartupSetupStage extends StatefulWidget {
  final VoidCallback onSetupComplete;

  const StartupSetupStage({super.key, required this.onSetupComplete});

  @override
  State<StartupSetupStage> createState() => _StartupSetupStageState();
}

class _StartupSetupStageState extends State<StartupSetupStage> {
  final List<String> _detectedRoots = [];
  final Set<String> _selectedRoots = {};
  bool _isIngesting = false;
  final TextEditingController _customDirController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _detectRoots();
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

  @override
  void dispose() {
    _customDirController.dispose();
    super.dispose();
  }

  Future<void> _completeSetup() async {
    setState(() => _isIngesting = true);

    try {
      if (Platform.isAndroid) {
        await PlatformPermissionService.instance.requestStorage();
      }

      // 1. Mark setup as completed in config
      AudioStorageService.instance.setSetupCompleted(true, selectedRoots: _selectedRoots.toList());

      // 2. Trigger initial library scan
      await PlaybackController.instance.scanLocalAudioDirectories(forceRefresh: true);
    } finally {
      setState(() => _isIngesting = false);
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
                  // App Icon
                  Container(
                    width: 72,
                    height: 72,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      gradient: LinearGradient(
                        colors: [colorScheme.primary, colorScheme.tertiary],
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                      ),
                      boxShadow: [
                        BoxShadow(
                          color: colorScheme.primary.withValues(alpha: 0.45),
                          blurRadius: 20,
                          spreadRadius: 2,
                        ),
                      ],
                    ),
                    child: Padding(
                      padding: const EdgeInsets.all(12.0),
                      child: Image.asset(
                        'assets/icon/icon_transparent.png',
                        fit: BoxFit.contain,
                      ),
                    ),
                  ),

                  const SizedBox(height: 18),

                  // Title
                  Text(
                    'Welcome to Attune',
                    style: AttuneTypography.soraHeading(
                      fontSize: 26,
                      fontWeight: FontWeight.bold,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text(
                    'Intelligent DJ-Grade Music Player & Real-Time Transition Engine',
                    textAlign: TextAlign.center,
                    style: AttuneTypography.jakartaBody(
                      color: colorScheme.onSurface.withValues(alpha: 0.65),
                      fontSize: 13,
                    ),
                  ),

                  const SizedBox(height: 28),

                  // Step 1: Discovered Music Folders
                  Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      'CONFIGURE MUSIC DIRECTORIES',
                      style: AttuneTypography.soraBadge(color: colorScheme.primary),
                    ),
                  ),
                  const SizedBox(height: 10),

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
                      );
                    }),

                  const SizedBox(height: 14),

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

                  // Step 2: Engine Features Preview
                  Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      'AUDIO ENGINE FEATURES ENABLED',
                      style: AttuneTypography.soraBadge(color: colorScheme.primary),
                    ),
                  ),
                  const SizedBox(height: 10),

                  _buildEngineFeature(
                    icon: Icons.graphic_eq_rounded,
                    title: 'Fractional Hermite Resampling',
                    description: 'Accurate 1.000x speed & pitch preservation for 44.1k/48k DACs',
                    colorScheme: colorScheme,
                  ),
                  const SizedBox(height: 6),
                  _buildEngineFeature(
                    icon: Icons.waves_rounded,
                    title: '1-Bar (4-Beat) Phrased Riff Handoff',
                    description: 'Multi-band isolator EQ carve + vocal integrity protection',
                    colorScheme: colorScheme,
                  ),
                  const SizedBox(height: 6),
                  _buildEngineFeature(
                    icon: Icons.storage_rounded,
                    title: 'Local Cache Indexing',
                    description: 'Instant startup in < 5ms with zero blocking freezes',
                    colorScheme: colorScheme,
                  ),

                  const SizedBox(height: 28),

                  // Get Started Button
                  SizedBox(
                    width: double.infinity,
                    height: 50,
                    child: ElevatedButton.icon(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: colorScheme.primary,
                        foregroundColor: colorScheme.onPrimary,
                        elevation: 4,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(18)),
                      ),
                      onPressed: _isIngesting ? null : _completeSetup,
                      icon: _isIngesting
                          ? const SizedBox(
                              width: 20,
                              height: 20,
                              child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                            )
                          : const Icon(Icons.play_arrow_rounded, size: 24),
                      label: Text(
                        _isIngesting ? 'Ingesting Audio Library...' : 'Get Started & Enter Attune',
                        style: AttuneTypography.soraSubheading(
                          fontSize: 15,
                          fontWeight: FontWeight.w700,
                          color: colorScheme.onPrimary,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
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
