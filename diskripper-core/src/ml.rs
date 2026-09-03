//! Machine Learning System for Content Identification and Organization.

pub mod pipeline;
pub mod audio_fingerprint;
pub mod music_identification;
pub mod video_fingerprint;
pub mod content_classifier;
pub mod feature_extraction;
pub mod hybrid_identifier;
pub mod self_learning;
pub mod data_management;
pub mod model_versioning;
pub mod inference;
pub mod training;

pub use pipeline::{MlPipeline, PipelineConfig, PipelineResult, ContentType, IdentificationSource};
pub use audio_fingerprint::{AudioFingerprint, AudioFingerprinter, FingerprintMatch};
pub use music_identification::{MusicIdentifier, MusicResult};
pub use video_fingerprint::{VideoFingerprint, VideoFingerprinter, VideoMatch};
pub use content_classifier::{ContentClassifier, ClassificationResult};
pub use hybrid_identifier::{HybridIdentifier, IdentificationResult, SignalConfidence};
pub use self_learning::{SelfLearning, FeedbackEntry, TrainingBatch, AccuracyMetrics};
pub use data_management::{DataManager, TrainingSample, DatasetStats};
pub use model_versioning::{ModelVersioning, ModelVersion};
pub use inference::{NeuralNetwork, TrainingConfig, TrainingHistory, EpochStats, Layer, build_mlp, build_audio_cnn};
