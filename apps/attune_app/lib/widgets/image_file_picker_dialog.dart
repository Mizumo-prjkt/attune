import 'dart:io';
import 'package:flutter/material.dart';
import '../theme/liquid_glass.dart';
import '../theme/typography.dart';

/// Interactive Liquid Material Glass Image File Picker Dialog.
/// Allows users to browse device storage, Scoped Storage folders, Downloads, and Pictures
/// to select custom album splash art, cover art, or artwork images (.png, .jpg, .jpeg, .webp, .gif, .bmp).
class ImageFilePickerDialog extends StatefulWidget {
  final String? initialDirectory;

  const ImageFilePickerDialog({super.key, this.initialDirectory});

  static Future<String?> show(BuildContext context, {String? initialDirectory}) {
    return showDialog<String>(
      context: context,
      barrierDismissible: true,
      barrierColor: Colors.black54,
      builder: (context) => ImageFilePickerDialog(initialDirectory: initialDirectory),
    );
  }

  @override
  State<ImageFilePickerDialog> createState() => _ImageFilePickerDialogState();
}

class _ImageFilePickerDialogState extends State<ImageFilePickerDialog> {
  late Directory _currentDir;
  final List<Directory> _subDirectories = [];
  final List<File> _imageFiles = [];
  bool _isLoading = false;
  String? _errorMessage;
  final TextEditingController _pathController = TextEditingController();

  static const List<String> _imageExtensions = [
    '.png',
    '.jpg',
    '.jpeg',
    '.webp',
    '.gif',
    '.bmp',
  ];

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
      final pictures = Directory('/storage/emulated/0/Pictures');
      final downloads = Directory('/storage/emulated/0/Download');
      final emulated = Directory('/storage/emulated/0');
      if (pictures.existsSync()) {
        starting = pictures;
      } else if (downloads.existsSync()) {
        starting = downloads;
      } else if (emulated.existsSync()) {
        starting = emulated;
      } else {
        starting = Directory('/sdcard');
      }
    } else {
      final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
      if (home != null) {
        final pictures = Directory('$home/Pictures');
        final downloads = Directory('$home/Downloads');
        if (pictures.existsSync()) {
          starting = pictures;
        } else if (downloads.existsSync()) {
          starting = downloads;
        } else {
          starting = Directory(home);
        }
      } else {
        starting = Directory.current;
      }
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
      final rawEntities = dir.listSync();

      final dirs = rawEntities.whereType<Directory>().toList();
      dirs.sort((a, b) => a.path.toLowerCase().compareTo(b.path.toLowerCase()));

      final images = rawEntities.whereType<File>().where((file) {
        final lower = file.path.toLowerCase();
        return _imageExtensions.any((ext) => lower.endsWith(ext));
      }).toList();
      images.sort((a, b) => a.path.toLowerCase().compareTo(b.path.toLowerCase()));

      setState(() {
        _subDirectories.clear();
        _subDirectories.addAll(dirs);
        _imageFiles.clear();
        _imageFiles.addAll(images);
        _isLoading = false;
      });
    } on FileSystemException catch (e) {
      setState(() {
        _subDirectories.clear();
        _imageFiles.clear();
        _isLoading = false;
        _errorMessage = 'Access restricted (${e.osError?.message ?? "Permission denied"})';
      });
    } catch (e) {
      setState(() {
        _subDirectories.clear();
        _imageFiles.clear();
        _isLoading = false;
        _errorMessage = 'Folder read notice: $e';
      });
    }
  }

  void _navigateUp() {
    final parent = _currentDir.parent;
    if (parent.path != _currentDir.path) {
      _navigateTo(parent);
    }
  }

  void _selectImageFile(File file) {
    if (!file.existsSync()) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Selected file does not exist on disk.')),
      );
      return;
    }
    final lower = file.path.toLowerCase();
    final isValid = _imageExtensions.any((ext) => lower.endsWith(ext));
    if (!isValid) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Selected file is not a supported image format (PNG, JPG, WEBP, GIF, BMP).')),
      );
      return;
    }

    Navigator.of(context).pop(file.path);
  }

  String _formatFileSize(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Dialog(
      backgroundColor: Colors.transparent,
      elevation: 0,
      insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 28),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: 640,
          maxHeight: MediaQuery.of(context).size.height * 0.85,
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
                    child: Icon(Icons.add_photo_alternate_rounded, color: colorScheme.primary, size: 24),
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Select Album Splash Art',
                          style: AttuneTypography.soraTitleMedium(
                            fontSize: 18,
                            fontWeight: FontWeight.w700,
                            color: Colors.white,
                          ),
                        ),
                        Text(
                          Platform.isAndroid
                              ? 'Choose image file from storage, DCIM, or Downloads'
                              : 'Select local image file (.png, .jpg, .webp)',
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

              // Quick Location Chips
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                physics: const BouncingScrollPhysics(),
                child: Row(
                  children: [
                    if (Platform.isAndroid) ...[
                      _buildQuickChip('Pictures', '/storage/emulated/0/Pictures', colorScheme),
                      _buildQuickChip('Downloads', '/storage/emulated/0/Download', colorScheme),
                      _buildQuickChip('DCIM', '/storage/emulated/0/DCIM', colorScheme),
                      _buildQuickChip('Music', '/storage/emulated/0/Music', colorScheme),
                    ] else ...[
                      _buildQuickChip(
                        'Pictures',
                        '${Platform.environment['HOME'] ?? Platform.environment['USERPROFILE']}/Pictures',
                        colorScheme,
                      ),
                      _buildQuickChip(
                        'Downloads',
                        '${Platform.environment['HOME'] ?? Platform.environment['USERPROFILE']}/Downloads',
                        colorScheme,
                      ),
                    ],
                  ],
                ),
              ),
              const SizedBox(height: 12),

              // Current Path Bar & Up Button
              Row(
                children: [
                  IconButton(
                    icon: const Icon(Icons.arrow_upward_rounded, color: Colors.white),
                    tooltip: 'Go up one directory level',
                    style: IconButton.styleFrom(
                      backgroundColor: Colors.white.withValues(alpha: 0.1),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
                    ),
                    onPressed: _navigateUp,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                      decoration: BoxDecoration(
                        color: Colors.black.withValues(alpha: 0.35),
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(color: Colors.white12),
                      ),
                      child: Text(
                        _currentDir.path,
                        style: AttuneTypography.jakartaBodySmall(
                          fontSize: 12,
                          color: Colors.white70,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),

              // Error notification if directory access is restricted
              if (_errorMessage != null)
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  margin: const EdgeInsets.only(bottom: 10),
                  decoration: BoxDecoration(
                    color: Colors.amber.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(color: Colors.amber.withValues(alpha: 0.4)),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.warning_amber_rounded, size: 16, color: Colors.amber),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _errorMessage!,
                          style: AttuneTypography.jakartaBodySmall(fontSize: 11, color: Colors.amber),
                        ),
                      ),
                    ],
                  ),
                ),

              // File and Directory List
              Expanded(
                child: _isLoading
                    ? const Center(child: CircularProgressIndicator())
                    : Container(
                        decoration: BoxDecoration(
                          color: Colors.black.withValues(alpha: 0.22),
                          borderRadius: BorderRadius.circular(16),
                          border: Border.all(color: Colors.white10),
                        ),
                        child: (_subDirectories.isEmpty && _imageFiles.isEmpty)
                            ? Center(
                                child: Text(
                                  'No subdirectories or image files found in this folder.',
                                  style: AttuneTypography.jakartaBodySmall(
                                    fontSize: 13,
                                    color: Colors.white38,
                                  ),
                                ),
                              )
                            : ListView.separated(
                                padding: const EdgeInsets.symmetric(vertical: 8),
                                physics: const BouncingScrollPhysics(),
                                itemCount: _subDirectories.length + _imageFiles.length,
                                separatorBuilder: (context, index) => const Divider(
                                  color: Colors.white10,
                                  height: 1,
                                  indent: 16,
                                  endIndent: 16,
                                ),
                                itemBuilder: (context, index) {
                                  // 1. Folders first
                                  if (index < _subDirectories.length) {
                                    final dir = _subDirectories[index];
                                    final dirName = dir.path.split(Platform.pathSeparator).last;
                                    return ListTile(
                                      leading: const Icon(Icons.folder_rounded, color: Color(0xFFFDD835), size: 22),
                                      title: Text(
                                        dirName,
                                        style: AttuneTypography.jakartaBodyMedium(
                                          fontSize: 14,
                                          weight: FontWeight.w500,
                                          color: Colors.white,
                                        ),
                                      ),
                                      trailing: const Icon(Icons.chevron_right_rounded, color: Colors.white38, size: 18),
                                      onTap: () => _navigateTo(dir),
                                    );
                                  }

                                  // 2. Image files
                                  final file = _imageFiles[index - _subDirectories.length];
                                  final fileName = file.path.split(Platform.pathSeparator).last;
                                  int fileBytes = 0;
                                  try {
                                    fileBytes = file.lengthSync();
                                  } catch (_) {}

                                  return ListTile(
                                    leading: ClipRRect(
                                      borderRadius: BorderRadius.circular(8),
                                      child: Container(
                                        width: 38,
                                        height: 38,
                                        color: Colors.black26,
                                        child: Image.file(
                                          file,
                                          fit: BoxFit.cover,
                                          errorBuilder: (ctx, err, stack) => Icon(
                                            Icons.image_rounded,
                                            size: 20,
                                            color: colorScheme.primary,
                                          ),
                                        ),
                                      ),
                                    ),
                                    title: Text(
                                      fileName,
                                      style: AttuneTypography.jakartaBodyMedium(
                                        fontSize: 13,
                                        weight: FontWeight.w600,
                                        color: Colors.white,
                                      ),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                    subtitle: Text(
                                      _formatFileSize(fileBytes),
                                      style: AttuneTypography.jakartaBodySmall(
                                        fontSize: 11,
                                        color: Colors.white54,
                                      ),
                                    ),
                                    trailing: FilledButton.tonal(
                                      style: FilledButton.styleFrom(
                                        backgroundColor: colorScheme.primary.withValues(alpha: 0.20),
                                        foregroundColor: colorScheme.primary,
                                        visualDensity: VisualDensity.compact,
                                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                                      ),
                                      onPressed: () => _selectImageFile(file),
                                      child: const Text('Select', style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold)),
                                    ),
                                    onTap: () => _selectImageFile(file),
                                  );
                                },
                              ),
                      ),
              ),

              const SizedBox(height: 16),

              // Manual Path Fallback / Paste
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _pathController,
                      style: AttuneTypography.jakartaBodySmall(color: Colors.white, fontSize: 12),
                      decoration: InputDecoration(
                        hintText: 'Enter absolute image file path or URL...',
                        hintStyle: AttuneTypography.jakartaBodySmall(color: Colors.white38),
                        isDense: true,
                        filled: true,
                        fillColor: Colors.black.withValues(alpha: 0.35),
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(12),
                          borderSide: BorderSide.none,
                        ),
                        contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  FilledButton(
                    style: FilledButton.styleFrom(
                      backgroundColor: colorScheme.primary,
                      foregroundColor: colorScheme.onPrimary,
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                    ),
                    onPressed: () {
                      final input = _pathController.text.trim();
                      if (input.isEmpty) return;
                      final f = File(input);
                      if (f.existsSync()) {
                        _selectImageFile(f);
                      } else {
                        // Check if it's a directory
                        final d = Directory(input);
                        if (d.existsSync()) {
                          _navigateTo(d);
                        } else {
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('File or directory does not exist.')),
                          );
                        }
                      }
                    },
                    child: const Text('Use Path', style: TextStyle(fontWeight: FontWeight.bold)),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildQuickChip(String label, String path, ColorScheme colorScheme) {
    final dir = Directory(path);
    if (!dir.existsSync()) return const SizedBox.shrink();

    final isSelected = _currentDir.path == dir.path;
    return Padding(
      padding: const EdgeInsets.only(right: 8.0),
      child: FilterChip(
        label: Text(label, style: const TextStyle(fontSize: 11)),
        selected: isSelected,
        selectedColor: colorScheme.primary.withValues(alpha: 0.25),
        backgroundColor: Colors.white.withValues(alpha: 0.08),
        checkmarkColor: colorScheme.primary,
        labelStyle: TextStyle(
          color: isSelected ? colorScheme.primary : Colors.white70,
          fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
        ),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(
            color: isSelected ? colorScheme.primary : Colors.white12,
          ),
        ),
        onSelected: (_) => _navigateTo(dir),
      ),
    );
  }
}
