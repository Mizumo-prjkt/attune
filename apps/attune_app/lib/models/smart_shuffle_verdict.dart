import 'track_item.dart';

/// Detailed breakdown of an individual candidate track evaluated by Smart Shuffle
class SmartShuffleCandidateMatch {
  final TrackItem track;
  final int queueIndex;
  final double totalScore;
  final int matchPercentage;
  final double tempoScore;
  final double harmonicScore;
  final double genreBonus;
  final double varietyScore;
  final double aggressionScore;
  final double recencyPenalty;

  final String tempoVerdict;
  final String harmonicVerdict;
  final String energyVerdict;
  final String rotationVerdict;
  final String primaryReason;

  const SmartShuffleCandidateMatch({
    required this.track,
    required this.queueIndex,
    required this.totalScore,
    required this.matchPercentage,
    required this.tempoScore,
    required this.harmonicScore,
    required this.genreBonus,
    required this.varietyScore,
    required this.aggressionScore,
    required this.recencyPenalty,
    required this.tempoVerdict,
    required this.harmonicVerdict,
    required this.energyVerdict,
    required this.rotationVerdict,
    required this.primaryReason,
  });
}

/// Comprehensive Smart Shuffle Verdict containing the winning selection,
/// comparative rationale, and the runner-up candidates evaluated.
class SmartShuffleVerdict {
  final TrackItem currentTrack;
  final SmartShuffleCandidateMatch winner;
  final List<SmartShuffleCandidateMatch> alternatives;
  final String summaryHeadline;
  final String detailedRationale;
  final DateTime timestamp;

  const SmartShuffleVerdict({
    required this.currentTrack,
    required this.winner,
    required this.alternatives,
    required this.summaryHeadline,
    required this.detailedRationale,
    required this.timestamp,
  });
}
