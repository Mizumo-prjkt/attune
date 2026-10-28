import 'dart:io';
import 'package:flutter/material.dart';
import '../services/environment_delegator.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Interactive Liquid Material Glass Directory Picker Dialog.
/// Enables users to manually navigate, browse, and select scoped storage roots,
/// SD card paths, or custom music directories on Android, Linux, Windows, and macOS.
class DirectoryPickerDialog extends StatefulWidget {
  final String? initialDirectory;

  const DirectoryPickerDialog({super.key, this.initialDirectory});

  static Future<String?> show(BuildContext context, {String? initialDirectory}) {
    return showDialog<String>(
      context: context,
      barrierDismissible: true,
      barrierColor: Colors.black54,
      builder: (context) => DirectoryPickerDialog(initialDirectory: initialDirectory),
    );
  }

  @override
  State<DirectoryPickerDialog> createState() => _DirectoryPickerDialogState();
}

class _DirectoryPickerDialogState extends State<DirectoryPickerDialog> {
  late Directory _currentDir;
  final List<Directory> _subDirectories = [];
  bool _isLoading = false;
  String? _errorMessage;
  final TextEditingController _pathController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _initStartingDirectory();
  }

  void _initStartingDirectory() {
    Directory starting;
    if (widget.initialDirectory != null &&
        widget.initialDirectory!.isNotEmpty &&
        Directory(widget.initialDirectory!).existsSync()) {
      starting = Directory(widget.initialDirectory!);
    } else if (Platform.isAndroid) {
      final emulated = Directory('/storage/emulated/0');
      starting = emulated.existsSync() ? emulated : Directory('/sdcard');
    } else {
      starting = Directory.current;
    }

    _navigateTo(starting);
  }

  @override
  void dispose() {
    _pathController.dispose();
    super.dispose();
  }

  void _navigateTo(Directory dir) {
    setState(() {
      _currentDir = dir;
      _pathController.text = dir.path;
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final entities = dir.listSync().whereType<Directory>().toList();
      entities.sort((a, b) => a.path.toLowerCase().compareTo(b.path.toLowerCase()));

      setState(() {
        _subDirectories.clear();
        _subDirectories.addAll(entities);
        _isLoading = false;
      });
    } on FileSystemException catch (e) {
      setState(() {
        _subDirectories.clear();
        _isLoading = false;
        _errorMessage = 'Access restricted (${e.osError?.message ?? "Permission denied"})';
      });
    } catch (e) {
      setState(() {
        _subDirectories.clear();
        _isLoading = false;
        _errorMessage = 'Directory read notice: $e';
      });
    }
  }

  void _navigateUp() {
    final parent = _currentDir.parent;
    if (parent.path != _currentDir.path) {
      _navigateTo(parent);
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isDesktop = MediaQuery.of(context).size.width >= 720;

    return Dialog(
      backgroundColor: Colors.transparent,
      elevation: 0,
      insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 28),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: 620,
          maxHeight: MediaQuery.of(context).size.height * 0.82,
        ),
        child: LiquidGlassContainer(
          padding: const EdgeInsets.all(22),
          borderRadius: 28,
          surfaceAlpha: 0.28,
          blurSigma: 36,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: colorScheme.primary.withValues(alpha: 0.16),
                      shape: BoxShape.circle,
                    ),
                    child: Icon(Icons.folder_open_rounded, color: colorScheme.primary, size: 24),
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Select Music Directory',
                          style: AttuneTypography.soraTitleMedium(
                            fontSize: 18,
                            fontWeight: FontWeight.w700,
                            color: Colors.white,
                          ),
                        ),
                        Text(
                          Platform.isAndroid
                              ? 'Browse device scoped storage or SD card'
                              : 'Select local audio collection folder',
                          style: AttuneTypography.jakartaBodySmall(
                            fontSize: 12,
                            color: Colors.white70,
                          ),
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close_rounded, color: Colors.white70),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // Path input & Up navigation
              Row(
                children: [
                  IconButton.filledTonal(
                    icon: const Icon(Icons.arrow_upward_rounded, size: 20),
                    tooltip: 'Parent Directory',
                    onPressed: _currentDir.parent.path != _currentDir.path ? _navigateUp : null,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 2),
                      decoration: BoxDecoration(
                        color: Colors.black26,
                        borderRadius: BorderRadius.circular(14),
                        border: Border.all(color: Colors.white12),
                      ),
                      child: TextField(
                        controller: _pathController,
                        style: const TextStyle(fontSize: 13, fontFamily: 'monospace', color: Colors.white),
                        decoration: const InputDecoration(
                          border: InputBorder.none,
                          isDense: true,
                          hintText: 'Enter path manually...',
                          hintStyle: TextStyle(color: Colors.white38, fontSize: 13),
                        ),
                        onSubmitted: (text) {
                          final dir = Directory(text.trim());
                          if (dir.existsSync()) {
                            _navigateTo(dir);
                          }
                        },
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.refresh_rounded, size: 20, color: Colors.white70),
                    tooltip: 'Refresh',
                    onPressed: () => _navigateTo(_currentDir),
                  ),
                ],
              ),
              const SizedBox(height: 12),

              // Directory list view
              Expanded(
                child: Container(
                  decoration: BoxDecoration(
                    color: Colors.black12,
                    borderRadius: BorderRadius.circular(16),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
                  ),
                  child: _isLoading
                      ? const Center(child: CircularProgressIndicator(strokeWidth: 2))
                      : _errorMessage != null
                          ? Center(
                              child: Padding(
                                padding: const EdgeInsets.all(20),
                                child: Column(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    const Icon(Icons.lock_outline_rounded, color: Colors.orangeAccent, size: 36),
                                    const SizedBox(height: 10),
                                    Text(
                                      _errorMessage!,
                                      textAlign: TextAlign.center,
                                      style: const TextStyle(color: Colors.white70, fontSize: 13),
                                    ),
                                    const SizedBox(height: 12),
                                    ElevatedButton.icon(
                                      onPressed: _navigateUp,
                                      icon: const Icon(Icons.arrow_upward_rounded, size: 16),
                                      label: const Text('Go Up'),
                                    ),
                                  ],
                                ),
                              ),
                            )
                          : _subDirectories.isEmpty
                              ? const Center(
                                  child: Text(
                                    'No subfolders here\nTap below to select this folder',
                                    textAlign: TextAlign.center,
                                    style: TextStyle(color: Colors.white54, fontSize: 13),
                                  ),
                                )
                              : ListView.separated(
                                  itemCount: _subDirectories.length,
                                  separatorBuilder: (_, __) => const Divider(height: 1, color: Colors.white10),
                                  itemBuilder: (context, index) {
                                    final sub = _subDirectories[index];
                                    final name = sub.path.split(Platform.pathSeparator).last;

                                    return ListTile(
                                      dense: true,
                                      leading: const Icon(Icons.folder_rounded, color: Colors.amberAccent, size: 22),
                                      title: Text(
                                        name,
                                        style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: Colors.white),
                                      ),
                                      trailing: const Icon(Icons.chevron_right_rounded, size: 18, color: Colors.white38),
                                      onTap: () => _navigateTo(sub),
                                    );
                                  },
                                ),
                ),
              ),
              const SizedBox(height: 16),

              // Actions
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text('Cancel', style: TextStyle(color: Colors.white70)),
                  ),
                  const SizedBox(width: 10),
                  FilledButton.icon(
                    style: FilledButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                    ),
                    icon: const Icon(Icons.check_circle_rounded, size: 18),
                    label: const Text('Select This Folder', style: TextStyle(fontWeight: FontWeight.w600)),
                    onPressed: () {
                      Navigator.of(context).pop(_currentDir.path);
                    },
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
