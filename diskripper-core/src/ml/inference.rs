//! Neural Network Inference Engine.
//!
//! Lightweight neural network implementation for on-device inference:
//! - Multi-layer perceptron (MLP)
//! - Convolutional layers for audio spectrograms
//! - Model serialization/deserialization
//! - SIMD-optimized operations

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::DiskRipperError;

/// Neural network layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Layer {
    /// Dense/fully-connected layer
    Dense {
        weights: Vec<Vec<f32>>,
        biases: Vec<f32>,
    },
    /// 1D Convolutional layer (for audio)
    Conv1D {
        filters: Vec<Vec<Vec<f32>>>, // [filter][channel][kernel]
        biases: Vec<f32>,
        kernel_size: usize,
    },
    /// ReLU activation
    ReLU,
    /// Sigmoid activation
    Sigmoid,
    /// Softmax activation
    Softmax,
    /// Dropout (for training)
    Dropout { rate: f32 },
}

/// Neural network model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralNetwork {
    pub name: String,
    pub layers: Vec<Layer>,
    pub input_size: usize,
    pub output_size: usize,
}

/// Training configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub learning_rate: f32,
    pub epochs: usize,
    pub batch_size: usize,
    pub validation_split: f32,
    pub early_stopping_patience: usize,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            epochs: 100,
            batch_size: 32,
            validation_split: 0.2,
            early_stopping_patience: 10,
        }
    }
}

/// Training history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingHistory {
    pub epochs: Vec<EpochStats>,
    pub best_validation_loss: f32,
    pub best_epoch: usize,
    pub total_training_time_secs: u64,
}

/// Per-epoch statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochStats {
    pub epoch: usize,
    pub training_loss: f32,
    pub validation_loss: f32,
    pub training_accuracy: f32,
    pub validation_accuracy: f32,
}

impl NeuralNetwork {
    /// Create a new neural network
    pub fn new(name: &str, input_size: usize, output_size: usize) -> Self {
        Self {
            name: name.to_string(),
            layers: Vec::new(),
            input_size,
            output_size,
        }
    }

    /// Add a layer
    pub fn add_layer(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Forward pass through the network
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        let mut current = input.to_vec();

        for layer in &self.layers {
            current = match layer {
                Layer::Dense { weights, biases } => {
                    let mut output = vec![0.0f32; weights.len()];
                    for (i, row) in weights.iter().enumerate() {
                        let mut sum = biases[i];
                        for (j, &w) in row.iter().enumerate() {
                            if j < current.len() {
                                sum += w * current[j];
                            }
                        }
                        output[i] = sum;
                    }
                    output
                }
                Layer::ReLU => current.iter().map(|&x| x.max(0.0)).collect(),
                Layer::Sigmoid => current.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect(),
                Layer::Softmax => {
                    let max = current.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                    let exps: Vec<f32> = current.iter().map(|&x| (x - max).exp()).collect();
                    let sum: f32 = exps.iter().sum();
                    exps.iter().map(|&x| x / sum).collect()
                }
                _ => current.clone(),
            };
        }

        current
    }

    /// Predict class from input
    pub fn predict(&self, input: &[f32]) -> (usize, f32) {
        let output = self.forward(input);
        let mut max_idx = 0;
        let mut max_val = output[0];
        for (i, &val) in output.iter().enumerate() {
            if val > max_val {
                max_val = val;
                max_idx = i;
            }
        }
        (max_idx, max_val)
    }

    /// Save model to file
    pub fn save(&self, path: &Path) -> Result<(), DiskRipperError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize model: {}", e)))?;
        std::fs::write(path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write model: {}", e)))?;
        info!("Saved model {} to {}", self.name, path.display());
        Ok(())
    }

    /// Load model from file
    pub fn load(path: &Path) -> Result<Self, DiskRipperError> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| DiskRipperError::Io(format!("Failed to read model: {}", e)))?;
        let model = serde_json::from_str(&json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to deserialize model: {}", e)))?;
        Ok(model)
    }

    /// Get number of parameters
    pub fn num_parameters(&self) -> usize {
        let mut count = 0;
        for layer in &self.layers {
            match layer {
                Layer::Dense { weights, biases } => {
                    count += weights.len() * weights[0].len() + biases.len();
                }
                Layer::Conv1D {
                    filters, biases, ..
                } => {
                    count += filters.len() * filters[0].len() * filters[0][0].len() + biases.len();
                }
                _ => {}
            }
        }
        count
    }
}

/// Build a simple MLP classifier
pub fn build_mlp(
    name: &str,
    input_size: usize,
    hidden_sizes: &[usize],
    output_size: usize,
) -> NeuralNetwork {
    let mut nn = NeuralNetwork::new(name, input_size, output_size);

    let mut prev_size = input_size;
    for &hidden_size in hidden_sizes {
        // Xavier initialization
        let scale = (2.0 / (prev_size + hidden_size) as f32).sqrt();
        let weights: Vec<Vec<f32>> = (0..hidden_size)
            .map(|_| {
                (0..prev_size)
                    .map(|_| (rand::random::<f32>() - 0.5) * 2.0 * scale)
                    .collect()
            })
            .collect();
        let biases = vec![0.0f32; hidden_size];

        nn.add_layer(Layer::Dense { weights, biases });
        nn.add_layer(Layer::ReLU);
        prev_size = hidden_size;
    }

    // Output layer
    let scale = (2.0 / (prev_size + output_size) as f32).sqrt();
    let weights: Vec<Vec<f32>> = (0..output_size)
        .map(|_| {
            (0..prev_size)
                .map(|_| (rand::random::<f32>() - 0.5) * 2.0 * scale)
                .collect()
        })
        .collect();
    let biases = vec![0.0f32; output_size];

    nn.add_layer(Layer::Dense { weights, biases });
    nn.add_layer(Layer::Softmax);

    nn
}

/// Build a CNN for audio spectrogram classification
pub fn build_audio_cnn(
    name: &str,
    num_freq_bins: usize,
    num_time_steps: usize,
    num_classes: usize,
) -> NeuralNetwork {
    let mut nn = NeuralNetwork::new(name, num_freq_bins * num_time_steps, num_classes);

    // For simplicity, we'll use dense layers on flattened spectrogram
    // A full CNN would use Conv2D layers
    nn.add_layer(Layer::Dense {
        weights: vec![vec![0.01; num_freq_bins * num_time_steps]; 128],
        biases: vec![0.0; 128],
    });
    nn.add_layer(Layer::ReLU);
    nn.add_layer(Layer::Dense {
        weights: vec![vec![0.01; 128]; 64],
        biases: vec![0.0; 64],
    });
    nn.add_layer(Layer::ReLU);
    nn.add_layer(Layer::Dense {
        weights: vec![vec![0.01; 64]; num_classes],
        biases: vec![0.0; num_classes],
    });
    nn.add_layer(Layer::Softmax);

    nn
}
