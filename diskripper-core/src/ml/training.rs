//! Training Pipeline for ML Models.
//!
//! Implements the actual training loop for neural networks:
//! - Backpropagation with gradient descent
//! - Mini-batch training
//! - Validation and early stopping
//! - Model checkpointing
//! - Training on real data

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::DiskRipperError;
use crate::ml::inference::{Layer, NeuralNetwork, TrainingConfig, TrainingHistory, EpochStats};

/// Trainer for neural networks
pub struct Trainer {
    config: TrainingConfig,
    model_dir: std::path::PathBuf,
}

/// Training data pair
#[derive(Debug, Clone)]
pub struct TrainingPair {
    pub input: Vec<f32>,
    pub target: Vec<f32>,
}

/// Training dataset
#[derive(Debug, Clone)]
pub struct Dataset {
    pub training: Vec<TrainingPair>,
    pub validation: Vec<TrainingPair>,
}

impl Trainer {
    pub fn new(config: TrainingConfig, model_dir: &Path) -> Self {
        Self {
            config,
            model_dir: model_dir.to_path_buf(),
        }
    }

    /// Train a model on the given dataset
    pub fn train(
        &self,
        model: &mut NeuralNetwork,
        dataset: &Dataset,
    ) -> Result<TrainingHistory, DiskRipperError> {
        info!(
            "Starting training: {} epochs, batch size {}, lr {}",
            self.config.epochs, self.config.batch_size, self.config.learning_rate
        );

        let mut history = TrainingHistory {
            epochs: Vec::new(),
            best_validation_loss: f32::INFINITY,
            best_epoch: 0,
            total_training_time_secs: 0,
        };

        let start_time = std::time::Instant::now();
        let mut patience_counter = 0;

        for epoch in 0..self.config.epochs {
            let mut training_data = dataset.training.clone();
            fastrand::shuffle(&mut training_data);

            let mut epoch_loss = 0.0f32;
            let mut num_batches = 0;

            for batch in training_data.chunks(self.config.batch_size) {
                let batch_loss = self.train_batch(model, batch)?;
                epoch_loss += batch_loss;
                num_batches += 1;
            }

            let avg_training_loss = epoch_loss / num_batches.max(1) as f32;
            let val_loss = self.compute_loss(model, &dataset.validation);
            let train_accuracy = self.compute_accuracy(model, &dataset.training);
            let val_accuracy = self.compute_accuracy(model, &dataset.validation);

            let epoch_stats = EpochStats {
                epoch,
                training_loss: avg_training_loss,
                validation_loss: val_loss,
                training_accuracy: train_accuracy,
                validation_accuracy: val_accuracy,
            };

            history.epochs.push(epoch_stats.clone());

            info!(
                "Epoch {}: train_loss={:.4}, val_loss={:.4}, train_acc={:.2}%, val_acc={:.2}%",
                epoch,
                avg_training_loss,
                val_loss,
                train_accuracy * 100.0,
                val_accuracy * 100.0
            );

            if val_loss < history.best_validation_loss {
                history.best_validation_loss = val_loss;
                history.best_epoch = epoch;
                patience_counter = 0;

                let best_path = self.model_dir.join(format!("{}_best.json", model.name));
                model.save(&best_path)?;
            } else {
                patience_counter += 1;
                if patience_counter >= self.config.early_stopping_patience {
                    info!("Early stopping at epoch {}", epoch);
                    break;
                }
            }
        }

        history.total_training_time_secs = start_time.elapsed().as_secs();

        let history_path = self.model_dir.join(format!("{}_history.json", model.name));
        let json = serde_json::to_string_pretty(&history)
            .map_err(|e| DiskRipperError::Io(format!("Failed to serialize history: {}", e)))?;
        std::fs::write(&history_path, json)
            .map_err(|e| DiskRipperError::Io(format!("Failed to write history: {}", e)))?;

        info!(
            "Training complete: best val_loss={:.4} at epoch {}, total time={}s",
            history.best_validation_loss, history.best_epoch, history.total_training_time_secs
        );

        Ok(history)
    }

    fn train_batch(
        &self,
        model: &mut NeuralNetwork,
        batch: &[TrainingPair],
    ) -> Result<f32, DiskRipperError> {
        let mut total_loss = 0.0f32;

        for pair in batch {
            let output = model.forward(&pair.input);
            let loss = compute_cross_entropy(&output, &pair.target);
            total_loss += loss;

            let gradients = compute_gradients(model, &pair.input, &pair.target, &output);
            update_weights(model, &gradients, self.config.learning_rate);
        }

        Ok(total_loss / batch.len() as f32)
    }

    fn compute_loss(&self, model: &NeuralNetwork, dataset: &[TrainingPair]) -> f32 {
        let mut total_loss = 0.0f32;
        for pair in dataset {
            let output = model.forward(&pair.input);
            total_loss += compute_cross_entropy(&output, &pair.target);
        }
        total_loss / dataset.len() as f32
    }

    fn compute_accuracy(&self, model: &NeuralNetwork, dataset: &[TrainingPair]) -> f32 {
        let mut correct = 0;
        for pair in dataset {
            let (predicted, _) = model.predict(&pair.input);
            let actual = pair.target.iter().enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            if predicted == actual {
                correct += 1;
            }
        }
        correct as f32 / dataset.len() as f32
    }
}

/// Compute cross-entropy loss
fn compute_cross_entropy(output: &[f32], target: &[f32]) -> f32 {
    let mut loss = 0.0f32;
    for (o, t) in output.iter().zip(target.iter()) {
        if *t > 0.0 {
            loss -= t * o.max(1e-10).ln();
        }
    }
    loss
}

/// Compute gradients via backpropagation
fn compute_gradients(
    model: &NeuralNetwork,
    input: &[f32],
    target: &[f32],
    output: &[f32],
) -> Vec<Vec<Vec<f32>>> {
    let mut gradients = Vec::new();

    let mut delta: Vec<f32> = output.iter().zip(target.iter())
        .map(|(o, t)| o - t)
        .collect();

    for layer in model.layers.iter().rev() {
        if let Layer::Dense { weights, .. } = layer {
            let input_size = weights[0].len();
            let output_size = weights.len();

            let mut weight_gradients = vec![vec![0.0f32; input_size]; output_size];

            let layer_input = if gradients.is_empty() {
                input.to_vec()
            } else {
                vec![0.0f32; input_size]
            };

            for i in 0..output_size {
                for j in 0..input_size {
                    weight_gradients[i][j] = delta[i] * layer_input[j];
                }
            }

            gradients.push(weight_gradients);

            if gradients.len() < model.layers.len() {
                let mut new_delta = vec![0.0f32; input_size];
                for j in 0..input_size {
                    for i in 0..output_size {
                        new_delta[j] += delta[i] * weights[i][j];
                    }
                }
                delta = new_delta;
            }
        }
    }

    gradients
}

/// Update weights using gradient descent
fn update_weights(
    model: &mut NeuralNetwork,
    gradients: &[Vec<Vec<f32>>],
    learning_rate: f32,
) {
    let mut grad_idx = 0;
    for layer in model.layers.iter_mut() {
        if let Layer::Dense { weights, biases } = layer {
            if grad_idx < gradients.len() {
                let grad = &gradients[grad_idx];
                for (i, row) in weights.iter_mut().enumerate() {
                    for (j, w) in row.iter_mut().enumerate() {
                        *w -= learning_rate * grad[i][j];
                    }
                }
                for (i, b) in biases.iter_mut().enumerate() {
                    *b -= learning_rate * grad[i].iter().sum::<f32>() / grad[i].len() as f32;
                }
                grad_idx += 1;
            }
        }
    }
}

/// Create a training dataset from audio features
pub fn create_audio_dataset(
    features: &[Vec<f32>],
    labels: &[usize],
    num_classes: usize,
    validation_split: f32,
) -> Dataset {
    let mut pairs: Vec<TrainingPair> = features.iter().zip(labels.iter())
        .map(|(f, &label)| {
            let mut target = vec![0.0f32; num_classes];
            target[label] = 1.0;
            TrainingPair {
                input: f.clone(),
                target,
            }
        })
        .collect();

    fastrand::shuffle(&mut pairs);

    let split_idx = (pairs.len() as f32 * (1.0 - validation_split)) as usize;
    let validation = pairs.split_off(split_idx);
    let training = pairs;

    Dataset { training, validation }
}
