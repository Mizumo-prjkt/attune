mod benchmarks;

use attune_core::audio::AudioDecoder;
use attune_core::planner::TransitionPlanner;
use attune_core::profiler::AudioProfiler;
use attune_core::renderer::TransitionRenderer;
use attune_core::schema::{AnalysisTier, TrackAnalysisProfile, TransitionRecipe};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "attune-cli")]
#[command(about = "Headless CLI for Attune audio profiling, transition planning, and rendering")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze an audio file and output its .tap profile
    Analyze {
        /// Input audio file path
        audio: PathBuf,
        /// Optional output path for the JSON .tap profile
        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    /// Plan a transition between two .tap profiles
    Plan {
        /// Profile for outgoing Track A (.tap JSON)
        tap_a: PathBuf,
        /// Profile for incoming Track B (.tap JSON)
        tap_b: PathBuf,
        /// Optional output path for the .ptr Transition Recipe
        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    /// Render a transition between two audio files using a .ptr recipe
    Render {
        /// Outgoing Track A audio file
        audio_a: PathBuf,
        /// Incoming Track B audio file
        audio_b: PathBuf,
        /// Transition recipe (.ptr JSON)
        #[arg(short, long)]
        recipe: PathBuf,
        /// Output WAV file path
        #[arg(short, long)]
        out: PathBuf,
    },

    /// End-to-end all-in-one transition: analyzes, plans, and renders
    Transition {
        /// Outgoing Track A audio file
        audio_a: PathBuf,
        /// Incoming Track B audio file
        audio_b: PathBuf,
        /// Output WAV file path
        #[arg(short, long)]
        out: PathBuf,
        /// Save intermediate .tap and .ptr JSON files
        #[arg(long)]
        save_intermediates: bool,
    },

    /// Run empirical benchmarks for Phase 1 verification
    Benchmarks {
        /// Directory to store generated benchmark WAV files and outputs
        #[arg(short, long, default_value = "target/benchmarks")]
        dir: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    let profiler = AudioProfiler::new();
    let planner = TransitionPlanner::new();
    let renderer = TransitionRenderer::new();

    match cli.command {
        Commands::Analyze { audio, out } => {
            println!("🎵 Decoding audio from: {}", audio.display());
            let buf = AudioDecoder::load_file(&audio)?;
            println!(
                "   Decoded: {:.2}s, {} channels, {} Hz",
                buf.duration_seconds(),
                buf.num_channels(),
                buf.sample_rate
            );

            let slug = audio
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("track");

            println!("🔬 Extracting 5-Layer Cue Zones...");
            let profile = profiler.profile_track(&buf, slug, AnalysisTier::FastHeadTail);

            println!("\n📊 Track Analysis Summary:");
            println!("   Acoustic Hash: {}", profile.track_id.audio_hash);
            println!("   BPM: {:.1}", profile.global_summary.nominal_bpm);
            println!("   Key: {}", profile.global_summary.primary_key);
            println!("   Aggression Index: {:.2}", profile.global_summary.mean_aggression_index);
            println!("   Macro Vibe: {}", profile.global_summary.macro_vibe_family);
            println!("   Dominant Instruments: {:?}", profile.global_summary.dominant_instruments);
            println!("   Cue Zones Extracted: {}", profile.cue_zones.len());

            for (idx, z) in profile.cue_zones.iter().enumerate() {
                println!(
                    "   - Zone {}: {:?} [{}ms - {}ms], Sustained Regions: {}",
                    idx,
                    z.zone_type,
                    z.start_ms,
                    z.end_ms,
                    z.sustained_pitch_regions.len()
                );
            }

            if let Some(out_path) = out {
                let json = serde_json::to_string_pretty(&profile)?;
                std::fs::write(&out_path, json)?;
                println!("\n💾 Saved .tap profile to: {}", out_path.display());
            }
        }

        Commands::Plan { tap_a, tap_b, out } => {
            let json_a = std::fs::read_to_string(&tap_a)?;
            let profile_a: TrackAnalysisProfile = serde_json::from_str(&json_a)?;

            let json_b = std::fs::read_to_string(&tap_b)?;
            let profile_b: TrackAnalysisProfile = serde_json::from_str(&json_b)?;

            println!("🎯 Planning transition from Track A -> Track B...");
            let recipe = planner.plan_transition(&profile_a, &profile_b);

            println!("\n✨ Transition Recipe Generated:");
            println!("   Archetype: {:?}", recipe.archetype);
            println!("   Confidence Score: {:.2}", recipe.confidence_score);
            println!(
                "   Track A Exit: {}ms -> {}ms",
                recipe.timing.track_a_transition_start_ms,
                recipe.timing.track_a_exit_end_ms
            );
            println!(
                "   Track B Entry: {}ms -> {}ms",
                recipe.timing.track_b_entry_start_ms,
                recipe.timing.track_b_native_lock_ms
            );
            println!("   Automation Keyframes: {}", recipe.dsp_automation_keyframes.len());

            if let Some(out_path) = out {
                let json = serde_json::to_string_pretty(&recipe)?;
                std::fs::write(&out_path, json)?;
                println!("💾 Saved .ptr recipe to: {}", out_path.display());
            }
        }

        Commands::Render {
            audio_a,
            audio_b,
            recipe,
            out,
        } => {
            println!("🎵 Loading audio tracks...");
            let buf_a = AudioDecoder::load_file(&audio_a)?;
            let buf_b = AudioDecoder::load_file(&audio_b)?;

            let json = std::fs::read_to_string(&recipe)?;
            let rec: TransitionRecipe = serde_json::from_str(&json)?;

            println!("🎛️ Rendering transition recipe to WAV...");
            renderer.render_to_wav(&buf_a, &buf_b, &rec, &out)?;
            println!("✅ Successfully rendered transition to: {}", out.display());
        }

        Commands::Transition {
            audio_a,
            audio_b,
            out,
            save_intermediates,
        } => {
            println!("🎧 Loading Audio Tracks...");
            let buf_a = AudioDecoder::load_file(&audio_a)?;
            let buf_b = AudioDecoder::load_file(&audio_b)?;

            let slug_a = audio_a.file_stem().and_then(|s| s.to_str()).unwrap_or("track_a");
            let slug_b = audio_b.file_stem().and_then(|s| s.to_str()).unwrap_or("track_b");

            println!("🔬 Profiling Track A: {}", slug_a);
            let profile_a = profiler.profile_track(&buf_a, slug_a, AnalysisTier::FastHeadTail);

            println!("🔬 Profiling Track B: {}", slug_b);
            let profile_b = profiler.profile_track(&buf_b, slug_b, AnalysisTier::FastHeadTail);

            println!("🎯 Planning transition...");
            let recipe = planner.plan_transition(&profile_a, &profile_b);

            println!(
                "\n✨ Selected Archetype: {:?} (Confidence: {:.2})",
                recipe.archetype, recipe.confidence_score
            );
            println!(
                "   Mix Window: Track A [{}ms - {}ms] -> Track B [{}ms - {}ms]",
                recipe.timing.track_a_transition_start_ms,
                recipe.timing.track_a_exit_end_ms,
                recipe.timing.track_b_entry_start_ms,
                recipe.timing.track_b_native_lock_ms
            );

            if save_intermediates {
                let tap_a_path = out.with_extension("track_a.tap.json");
                let tap_b_path = out.with_extension("track_b.tap.json");
                let ptr_path = out.with_extension("recipe.ptr.json");

                std::fs::write(&tap_a_path, serde_json::to_string_pretty(&profile_a)?)?;
                std::fs::write(&tap_b_path, serde_json::to_string_pretty(&profile_b)?)?;
                std::fs::write(&ptr_path, serde_json::to_string_pretty(&recipe)?)?;
                println!("💾 Intermediate .tap and .ptr files written.");
            }

            println!("🎛️ Rendering output WAV to: {}", out.display());
            renderer.render_to_wav(&buf_a, &buf_b, &recipe, &out)?;
            println!("🎉 Transition complete!");
        }

        Commands::Benchmarks { dir } => {
            println!("🧪 Running Attune Empirical Transition Benchmarks in: {}", dir.display());
            let cases = benchmarks::generate_benchmarks(&dir)?;

            for (name, path_a, path_b) in cases {
                println!("\n------------------------------------------------------------");
                println!("▶️ Testing Benchmark: {}", name);
                let buf_a = AudioDecoder::load_file(&path_a)?;
                let buf_b = AudioDecoder::load_file(&path_b)?;

                let prof_a = profiler.profile_track(&buf_a, &name, AnalysisTier::FastHeadTail);
                let prof_b = profiler.profile_track(&buf_b, &name, AnalysisTier::FastHeadTail);

                println!(
                    "   Track A: BPM={:.1}, Aggression={:.2}, Instruments={:?}",
                    prof_a.global_summary.nominal_bpm,
                    prof_a.global_summary.mean_aggression_index,
                    prof_a.global_summary.dominant_instruments
                );
                println!(
                    "   Track B: BPM={:.1}, Aggression={:.2}, Instruments={:?}",
                    prof_b.global_summary.nominal_bpm,
                    prof_b.global_summary.mean_aggression_index,
                    prof_b.global_summary.dominant_instruments
                );

                let recipe = planner.plan_transition(&prof_a, &prof_b);
                println!(
                    "   🎯 Result Archetype: {:?} (Confidence: {:.2})",
                    recipe.archetype, recipe.confidence_score
                );

                let out_wav = dir.join(format!("{}_transition.wav", name));
                renderer.render_to_wav(&buf_a, &buf_b, &recipe, &out_wav)?;
                println!("   🔊 Rendered Output: {}", out_wav.display());
            }

            println!("\n✅ All benchmark test cases executed successfully!");
        }
    }

    Ok(())
}
